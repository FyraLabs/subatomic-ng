use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use surrealdb::{sql::Thing, RecordId};
use tracing::{debug, warn};

use crate::obj_store::object_store;

use super::{gpg_key::GPG_KEY_TABLE, rpm::{Rpm, RpmRef}};
pub const TAG_TABLE: &str = "repo_tag";
pub const COMPOSE_TABLE: &str = "repo_assemble";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCompose {
    pub id: Thing,
    pub tag: RecordId,
    pub packages: Vec<RpmRef>,
}

impl TagCompose {
    pub fn new(tag: &str, packages: Vec<RpmRef>) -> Self {
        Self {
            id: Thing::from((COMPOSE_TABLE, surrealdb::sql::Id::ulid())),
            tag: RecordId::from_table_key(TAG_TABLE, tag),
            packages,
        }
    }

    pub async fn save(&self) -> color_eyre::Result<Self> {
        let query = super::DB
            .upsert((COMPOSE_TABLE, self.id.id.to_raw()))
            .content(self.clone())
            .await?;

        query.ok_or_else(|| color_eyre::eyre::eyre!("nothing returned from insert"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// A repo "tag" that can be assembled into an actual yum repo
pub struct Tag {
    pub id: Thing,
    pub name: String,
    pub comps_xml: Option<String>,
    #[serde(default)]
    pub signing_key: Option<RecordId>,
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: Thing::from((TAG_TABLE, surrealdb::sql::Id::String(name.clone()))),
            name,
            comps_xml: None,
            signing_key: None,
        }
    }

    pub async fn get(id: &str) -> color_eyre::Result<Option<Self>> {
        Ok(super::DB.select((TAG_TABLE, id)).await?)
    }

    pub async fn delete(&self) -> color_eyre::Result<()> {
        super::DB
            .delete((TAG_TABLE, self.id.id.to_raw()))
            .await?
            .map_or(Ok(()), Ok)
    }

    pub async fn get_all() -> color_eyre::Result<Vec<Self>> {
        Ok(super::DB.select(TAG_TABLE).await?)
    }
    
    pub fn set_gpg_key(&mut self, key: &str) {
        self.signing_key = Some(RecordId::from_table_key(GPG_KEY_TABLE, key));
    }

    /// Create or update a tag in the database
    pub async fn save(&self) -> color_eyre::Result<Self> {
        // if already exists return error
        // if (super::DB
        //     .select::<Option<Tag>>((TAG_TABLE, self.id.id.to_raw()))
        //     .await?)
        //     .is_some()
        // {
        //     return Err(color_eyre::eyre::eyre!("tag already exists"));
        // }

        let query: color_eyre::Result<Option<Self>> = super::DB
            .upsert((TAG_TABLE, self.id.id.to_raw()))
            .content(self.clone())
            .await
            .map_err(|e| color_eyre::eyre::eyre!(e));

        match query {
            Ok(query) => {
                query.ok_or_else(|| color_eyre::eyre::eyre!("nothing returned from insert"))
            }
            Err(e) => Err(e),
        }
        // query.ok_or_else(|| color_eyre::eyre::eyre!("nothing returned from insert"))
    }

    // The assembly process is as follows:
    // 1. Get all packages that are tagged to this repo
    // 2. Symlink them to a staging repo directory we create
    // 3. In that directory, run createrepo_c with the options we want
    // 4. Finally, force symlink the successful staging repo to the export directory, with the tag name

    // ln -sf $staging_repo $export_dir/$tag_name

    pub async fn get_available_rpms(&self) -> color_eyre::Result<Vec<Rpm>> {
        let mut query = super::DB
            .query("SELECT * FROM rpm_package WHERE tag = $tag_id AND available = true;")
            .bind(("tag_id", self.id.clone()))
            .await?;

        let pkgs: Vec<Rpm> = query.take(0)?;

        Ok(pkgs)
    }

    pub fn export_dir(&self) -> PathBuf {
        crate::config::CONFIG
            .get()
            .unwrap()
            .export_dir
            .join(&self.name)
    }

    pub async fn assemble(&self) -> color_eyre::Result<()> {
        // let mut pkgs: surrealdb::Response = super::DB.query("SELECT * FROM rpm_package WHERE id IN (SELECT id, name, timestamp FROM rpm_package GROUP BY name,timestamp ORDER BY timestamp DESC LIMIT 1).id;").await?;

        debug!("assembling tag: {}", self.name);
        // let pkgs_vec: Vec<Rpm> = pkgs.take(0)?;
        // let p: Option<Rpm> = pkgs_vec.into_iter().next();
        let config = crate::config::CONFIG
            .get()
            .ok_or_else(|| color_eyre::eyre::eyre!("config not loaded"))?;

        let pkgs = self.get_available_rpms().await?;

        let compose = TagCompose::new(&self.name, pkgs.iter().map(|r| r.into()).collect())
            .save()
            .await?;

        let staging_id = compose.id.id.to_raw();
        let staging_dir_name = format!("{tag}/{tag}_{staging_id}", tag = self.name);

        let staging_dir = config.repo_cache_dir.join(&staging_dir_name);

        if staging_dir.exists() {
            return Err(color_eyre::eyre::eyre!("staging directory already exists"));
        }

        tokio::fs::create_dir_all(&staging_dir).await?;

        futures::future::try_join_all(pkgs.into_iter().map(|pkg| {
            let staging_dir = staging_dir.clone();
            async move {
                let cache_key = &pkg.object_key;
                let cache_key_filename = cache_key.split('/').last().unwrap();
                let obj_store = object_store();
                let src = obj_store.get(cache_key).await?.canonicalize()?;
                tracing::debug!(?src);

                if staging_dir.join(cache_key_filename).exists() {
                    warn!(
                        ?cache_key,
                        "File name seems to conflict, removing already existing file"
                    );
                    tokio::fs::remove_file(&staging_dir.join(cache_key_filename)).await?;
                }

                let target_path = staging_dir.join(format!(
                    "{ulid}-{cache_key_filename}",
                    ulid = pkg.id.id.to_raw()
                ));
                tokio::fs::remove_file(&target_path).await.ok();
                let metadata = tokio::fs::metadata(&src).await?;
                tracing::trace!(?metadata);
                if target_path.metadata().is_ok() {
                    warn!(
                        ?cache_key,
                        "File name seems to conflict, removing already existing file"
                    );
                    tokio::fs::remove_file(&target_path).await?;
                }

                debug!("Symlinking {} to {}", src.display(), target_path.display());
                tokio::fs::symlink(src, target_path).await?;

                Result::<_, color_eyre::Report>::Ok(())
            }
        }))
        .await?;

        let mut process = tokio::process::Command::new("createrepo_c")
            .arg(&staging_dir)
            .spawn()?;

        let status = process.wait().await?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!("createrepo_c failed"));
        }

        // symlink to export directory

        let staging_dir = staging_dir.canonicalize()?;

        let export_dir = self.export_dir();

        tokio::fs::create_dir_all(&export_dir.parent().unwrap()).await?;

        tracing::info!(
            "symlinking {} to {}",
            staging_dir.display(),
            export_dir.display()
        );

        if export_dir.exists() {
            tokio::fs::remove_dir_all(&export_dir).await?;
        }

        tokio::fs::symlink(&staging_dir.canonicalize()?, &export_dir).await?;

        Ok(())
    }
}
