use color_eyre::eyre::eyre;
use rpm::{signature::Signing, DependencyFlags, PackageMetadata};
use serde::{Deserialize, Serialize};
use surrealdb::{sql::Thing, RecordId};
use tracing::trace;
use ulid::Ulid;

use crate::obj_store::object_store;

use super::{gpg_key::GpgKey, tag::TAG_TABLE, DB};
pub const RPM_PREFIX: &str = "rpm";
pub const RPM_TABLE: &str = "rpm_package";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// A lighter reference to an RPM object, used for linking to the full object
/// in lockfiles and other places where the full object is not needed.
pub struct RpmRef {
    pub id: ulid::Ulid,
    #[serde(skip_serializing)]
    rpm_id: RecordId,
    pub name: String,
    pub object_key: String,
    pub signed_object_key: Option<String>,
    pub tag: Option<String>,
}

impl RpmRef {
    pub fn new(id: ulid::Ulid, name: String, object_key: String) -> Self {
        Self {
            id,
            name,
            rpm_id: RecordId::from_table_key(RPM_TABLE, id.to_string()),
            object_key,
            signed_object_key: None,
            tag: None,
        }
    }
    pub async fn get(id: ulid::Ulid) -> color_eyre::Result<Option<Self>> {
        DB.get()
            .select((RPM_TABLE, id.to_string()))
            .await
            .map_err(Into::into)
    }

    pub async fn get_full(&self) -> color_eyre::Result<Rpm> {
        Rpm::get(self.id).await?.ok_or_else(|| eyre!("not found"))
    }
}

impl From<&Rpm> for RpmRef {
    fn from(rpm: &Rpm) -> Self {
        Self {
            id: Ulid::from_string(&rpm.id.id.to_raw()).unwrap(),
            name: rpm.name.clone(),
            object_key: rpm.object_key.clone(),
            rpm_id: RecordId::from_table_key(RPM_TABLE, rpm.id.id.to_raw()),
            signed_object_key: rpm.signed_object_key.clone(),
            tag: Some(rpm.tag.key().to_string())
        }
    }
}

/* original json data
    {
        "id": 79224,
        "name": "ctwm-debuginfo",
        "epoch": 0,
        "version": "4.1.0",
        "release": "1.fc41",
        "arch": "x86_64",
        "file_path": "ctwm-debuginfo-0:4.1.0-1.fc41.x86_64.rpm"
    },
*/

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PkgDependency {
    pub flag: Option<String>,
    pub name: String,
    pub version: Option<String>,
}

macro_rules! convert_depflags {
    ($($flag:ident)+) => {::paste::paste! {[$(
        (DependencyFlags::[<$flag:snake:upper>], stringify!([<$flag:lower>])),
    )+]}};
}

impl From<&rpm::Dependency> for PkgDependency {
    fn from(dep: &rpm::Dependency) -> Self {
        let flags = dep.flags;
        let flag = convert_depflags![ScriptPre ScriptPost ScriptPreun ScriptPostun ScriptVerify FindRequires FindProvides Triggerin Triggerun Triggerpostun Missingok Preuntrans Postuntrans]
        .iter()
        .find(|(f, _)| flags.contains(*f))
        .map(|(_, name)| name.to_string());
        let version = if dep.version.is_empty() {
            None
        } else {
            Some(dep.version.clone())
        };

        Self {
            flag: flag.to_owned(),
            name: dep.name.clone(),
            version,
        }
    }
}

// we want to replace the id field with a ulid, and the path to be a key to the object

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rpm {
    // ID of the object
    // #[serde(skip_serializing)]
    pub id: Thing,
    pub epoch: u32,
    pub name: String,
    pub version: String,
    pub release: String,
    pub arch: String,
    pub object_key: String,
    #[serde(default)]
    pub provides: Vec<PkgDependency>,
    #[serde(default)]
    pub requires: Vec<PkgDependency>,
    #[serde(default)]
    pub signed_object_key: Option<String>,

    pub tag: RecordId,
    pub timestamp: surrealdb::sql::Datetime,
    /// The latest tag means that this package is probably the latest available version.
    ///
    /// There should only be one package with the same name and architecture that has this flag set.
    // XXX: This flag also determines if the package should be available in a tag,
    // so to delist a package from a tag, we should set this to false.
    available: bool,
}

fn get_split_id_string(id: &str) -> String {
    // split into a tree-like directory structure using first two chars
    format!("{}/{}/{}", &id[0..1], &id[1..2], id)
}

fn get_rpm_path(name: &str, epoch: u32, version: &str, release: &str, arch: &str) -> String {
    format!("{name}-{epoch}:{version}-{release}.{arch}.rpm")
}
/// Generate the object key and signed key for an RPM object
/// The object key is the path to the RPM object in the object store
///
/// The signed key is the path to the signed version of the RPM object, which can be used later to store the signed
/// RPM object.
fn rpm_object_key(id: String, rpm: &PackageMetadata) -> (String, String) {
    let id_string = get_split_id_string(&id);

    let name = rpm.get_name().unwrap();
    let epoch = rpm.get_epoch().unwrap_or_default();
    let version = rpm.get_version().unwrap();
    let release = rpm.get_release().unwrap();
    let arch = rpm.get_arch().unwrap();

    let rpm_path = get_rpm_path(name, epoch, version, release, arch);
    let object_key = format!("{RPM_PREFIX}/{id_string}/{rpm_path}");
    let signed_key = format!("{RPM_PREFIX}/{id_string}/signed/{rpm_path}");

    (object_key, signed_key)
}

impl Rpm {
    pub fn new(pkg_meta: PackageMetadata, tag: &str) -> color_eyre::Result<Self> {
        let id = Thing::from((RPM_TABLE, surrealdb::sql::Id::ulid()));

        let epoch = pkg_meta.get_epoch().unwrap_or_default();
        let name = pkg_meta.get_name()?.to_owned();
        let version = pkg_meta.get_version()?.to_owned();
        let release = pkg_meta.get_release()?.to_owned();
        let arch = pkg_meta.get_arch()?.to_owned();
        let provides = pkg_meta
            .get_provides()?
            .iter()
            .map(|dep| dep.into())
            .collect();
        let requires = pkg_meta
            .get_requires()?
            .iter()
            .map(|dep| dep.into())
            .collect();
        // Requires(post): ...
        //          ^^^^ flags
        // let full_meta = pkg_meta;
        Ok(Rpm {
            object_key: rpm_object_key(id.id.to_raw(), &pkg_meta).0,
            // this should stay none until the package itself is signed
            signed_object_key: None,
            id,
            epoch,
            name,
            version,
            release,
            arch,
            provides,
            requires,
            tag: RecordId::from_table_key(TAG_TABLE, tag),
            timestamp: chrono::Utc::now().into(),
            available: false,
        })
    }
    pub fn from_path(path: impl AsRef<std::path::Path>, tag: &str) -> color_eyre::Result<Self> {
        let pkg = rpm::Package::open(path.as_ref())?;
        Self::new(pkg.metadata, tag)
    }

    /// Mark this package as the latest package, and unmark every package with the same name + architecture
    /// as not the latest package.
    pub async fn mark_available(&self) -> color_eyre::Result<Self> {
        // query all packages with the same name, architecture, and tag
        // and mark them as not the latest package

        DB.query("BEGIN;")
        .query("UPDATE rpm_package SET available = false WHERE name = $name AND arch = $arch AND tag = $tag;")
        .query("UPDATE rpm_package SET available = true WHERE id = $id;")
        .query("COMMIT;")
        .bind(("name", self.name.clone()))
        .bind(("arch", self.arch.clone()))
        .bind(("tag", self.tag.clone()))
        .bind(("id", self.id.clone()))
        .await?;

        let mut new_entry = self.clone();
        new_entry.available = true;
        let a: Option<Self> = DB
            .update((RPM_TABLE, self.id.id.to_raw()))
            .content(new_entry)
            .await?;
        self.id.id.to_raw();
        a.ok_or_else(|| eyre!("failed to update entry"))
    }

    pub async fn mark_unavailable(&self) -> color_eyre::Result<Self> {
        let mut new_entry = self.clone();
        new_entry.available = false;
        let a: Option<Self> = DB
            .query("UPDATE rpm_package SET available = false WHERE id = $id;")
            .bind(("id", self.id.clone()))
            .await?
            .take(0)?;

        Ok(a.unwrap())
    }

    pub async fn delete(&self) -> color_eyre::Result<()> {
        let a: Option<Self> = DB.delete((RPM_TABLE, self.id.id.to_raw())).await?;

        tracing::debug!("deleted from db: {:#?}", a);

        // Delete artifact

        object_store().remove(&self.object_key).await?;

        Ok(())
    }

    /// Commits the RPM object to the database, optionally marking it as the latest version in that tag
    pub async fn commit_to_db(&self, latest: bool) -> color_eyre::Result<()> {
        trace!("committing to db");
        // insert into db
        let a: Option<Self> = DB
            .get()
            .insert((RPM_TABLE, self.id.id.to_raw()))
            .content(self.clone())
            .await?;

        if latest {
            tracing::debug!("marking as latest");
            self.mark_available().await?;
        }

        tracing::trace!("inserted into db: {:#?}", a);

        // if latest {
        //     return self.mark_one_latest().await;
        // }

        Ok(())
    }

    /// Fetches the RPM object from the database
    #[tracing::instrument]
    pub async fn get(id: ulid::Ulid) -> color_eyre::Result<Option<Self>> {
        let a: Option<Self> = DB.get().select((RPM_TABLE, id.to_string())).await?;

        tracing::trace!(item = ?a, "got from db");

        Ok(a)
    }

    pub async fn get_all() -> color_eyre::Result<Vec<Self>> {
        let a: Vec<Self> = DB.get().select(RPM_TABLE).await?;

        tracing::info!("got from db: {:#?}", a);

        Ok(a)
    }

    pub async fn sign(&self, key: GpgKey) -> color_eyre::Result<Self> {
        tracing::debug!("signing rpm");
        let object_file = object_store().get(&self.object_key).await?;
        tracing::trace!("got object file: {:?}", object_file);

        let signer = rpm::signature::pgp::Signer::load_from_asc(&key.secret_key)?;
        tracing::trace!(?signer, "loaded signer");

        tracing::trace!("opening rpm");
        let mut rpm = rpm::Package::open(object_file)?;

        tracing::trace!("signing rpm");
        rpm.sign(&signer)?;

        // write the signed rpm to the object store
        let mut buf = Vec::new();
        tracing::trace!("writing signed rpm to buffer");
        rpm.write(&mut buf)?;

        let signed_key = self.signed_object_key.clone().unwrap_or_else(|| {
            let (_, signed_key) = rpm_object_key(self.id.id.to_raw(), &rpm.metadata);
            signed_key
        });

        tracing::trace!("putting signed rpm in object store");
        object_store().put_bytes(&signed_key, buf).await?;

        tracing::trace!("updating db with signed key");
        let res: Option<Self> = DB
            .update((RPM_TABLE, self.id.id.to_raw()))
            .content(Rpm {
                signed_object_key: Some(signed_key),
                ..self.clone()
            })
            .await?;

        Ok(res.ok_or_else(|| eyre!("failed to update entry"))?)

        // todo!()
    }
}

// upload rpm should generate that and, upload to object store, and then insert into db

#[cfg(test)]
mod tests {
    use super::*;

    const RPM_PATH: &str = "test/data/anda-srpm-macros-0:0.2.6-1.fc41.noarch.rpm";
    #[test]
    fn test_rpm_from_path() {
        let rpm = Rpm::from_path(RPM_PATH, "foobar").unwrap();

        println!("{:#?}", rpm);
        assert_eq!(rpm.name, "anda-srpm-macros");
        assert_eq!(rpm.version, "0.2.6");
        assert_eq!(rpm.release, "1.fc41");
        assert_eq!(rpm.arch, "noarch");
    }

    #[test]
    fn test_rpm_ref_from_rpm() {
        let rpm = Rpm::from_path(RPM_PATH, "foobar").unwrap();
        let rpm_ref = RpmRef::from(&rpm);

        println!("{:#?}", rpm_ref);
        assert_eq!(rpm_ref.name, "anda-srpm-macros");
    }
}
