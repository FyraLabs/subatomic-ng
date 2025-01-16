use std::{path::PathBuf, sync::Arc};

use crate::{cache::Cache, obj_store::{ObjectStorage, StorageBackend}};
use clap::{Parser, ValueEnum};
use object_store::ObjectStore;
use std::sync::OnceLock;

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(ValueEnum, Debug, Clone)]
pub enum ObjectStoreType {
    /// S3 object store
    #[value(name = "s3")]
    S3,
    /// Local FS object store, uses the object cache directory to store objects
    #[value(name = "local")]
    Local,
    
    /// Only serve from the cache
    #[value(name = "cacheonly")]
    CacheOnly,
}

#[derive(Parser, Debug, Clone)]
#[group(id = "object_store", multiple = true)]
#[group(requires = "object_store_type")]
pub struct S3StoreConfig {
    #[clap(long, env = "S3_BUCKET")]
    pub s3_bucket: String,

    #[clap(long, env = "S3_REGION")]
    pub s3_region: String,

    #[clap(long, env = "S3_ACCESS_KEY")]
    pub s3_access_key: String,

    #[clap(long, env = "S3_SECRET_KEY")]
    pub s3_secret_key: String,

    #[clap(long, env = "S3_ENDPOINT")]
    pub s3_endpoint: String,
}

#[derive(Parser, Debug, Clone)]
pub struct Config {
    #[clap(long, env = "SURREAL_HOST")]
    pub host: String,

    #[clap(long, env = "SURREAL_DB", default_value = "subatomic")]
    pub surreal_db: String,

    #[clap(long, env = "SURREAL_NS", default_value = "subatomic")]
    pub surreal_ns: String,

    #[clap(flatten)]
    pub s3_config: Option<S3StoreConfig>,

    #[clap(long, env = "OBJECT_STORE_TYPE", default_value = "s3")]
    pub object_store_type: ObjectStoreType,
    
    
    /// Delete RPMs when they are marked as unavailable
    /// 
    /// This mimics old subatomic behavior, where setting the prune flag
    /// would delete the object entirely, marking them permanently unavailable.
    #[clap(long, env = "DELETE_WHEN_PRUNE", default_value = "false")]
    pub delete_when_prune: bool,

    // #[clap(long, env = "S3_BUCKET")]
    // pub s3_bucket: String,

    // #[clap(long, env = "S3_REGION")]
    // pub s3_region: String,

    // #[clap(long, env = "S3_ACCESS_KEY")]
    // pub s3_access_key: String,

    // #[clap(long, env = "S3_SECRET_KEY")]
    // pub s3_secret_key: String,

    // #[clap(long, env = "S3_ENDPOINT")]
    // pub s3_endpoint: String,
    /// Location for the local object cache
    #[clap(long, env = "CACHE_DIR", default_value = "/tmp/subatomic")]
    pub cache_dir: PathBuf,

    #[clap(long, env = "REPO_CACHE_DIR", default_value = "/tmp/subatomic/repo")]
    /// Directory to cache generated repos to
    ///
    /// This is where the generated repos will by symlinked to.
    ///
    /// This directory shouldn't really be accessible in the web directory,
    /// but the web server should be able to read it to resolve the symlinks.
    ///
    /// This directory ideally should be on the same filesystem as the export directory.
    pub repo_cache_dir: PathBuf,

    #[clap(
        long,
        env = "OBJECT_CACHE_DIR",
        default_value = "/tmp/subatomic/objects"
    )]
    /// Directory to download objects to before moving to the cache directory
    pub object_cache_dir: PathBuf,

    /// Directory to export the repo to
    ///
    /// This is where you should point your web server to serve the repository.
    ///
    /// Contains repos generated from each tag, symlinked to another cache directory.
    #[clap(long, env = "EXPORT_DIR", default_value = "/tmp/subatomic/export")]
    pub export_dir: PathBuf,

    /// Address to listen on for the HTTP API
    #[clap(long, env = "LISTEN_ADDR", default_value = "0.0.0.0:3000")]
    pub listen_addr: String,
}

impl Config {
    pub fn init() -> Self {
            let cfg = Self::parse();
            CONFIG.set(cfg.clone()).expect("cannot read CLI configs");
            // let region = s3::Region::Custom {
            //     region: cfg.s3_config.s3_region.clone(),
            //     endpoint: cfg.s3_config.s3_endpoint.clone(),
            // };
            // tracing::info!("Initializing object store");
            // let creds = s3::creds::Credentials::new(
            //     Some(&cfg.s3_config.s3_access_key),
            //     Some(&cfg.s3_config.s3_secret_key),
            //     None,
            //     None,
            //     None,
            // )
            // .expect("cannot create credentials");
            // ObjectStore::init(region, creds, &cfg.s3_config.s3_bucket);
            //

            match cfg.object_store_type {
                ObjectStoreType::Local => {
                    let obj_cache_dir = cfg.object_cache_dir.clone();
                    std::fs::create_dir_all(&obj_cache_dir).expect("cannot create object cache dir");
                    let local_objstore = object_store::local::LocalFileSystem::new_with_prefix(
                        obj_cache_dir
                    )
                    .expect("cannot create local object store")
                    .with_automatic_cleanup(true);

                    let store = Arc::new(local_objstore) as Arc<dyn ObjectStore>;
                    let store = Arc::new(store) as Arc<dyn StorageBackend>;

                    let store = ObjectStorage::new(store, cfg.cache());
                    crate::obj_store::OBJECT_STORE
                        .set(store)
                        .unwrap_or_else(|_| panic!("cannot set object store"));
                }
                ObjectStoreType::S3 => {
                    let s3_config = cfg.s3_config.clone().expect("no S3 config");
                    let s3_store = object_store::aws::AmazonS3Builder::new()
                        .with_bucket_name(s3_config.s3_bucket)
                        .with_region(s3_config.s3_region)
                        .with_endpoint(s3_config.s3_endpoint)
                        .with_access_key_id(s3_config.s3_access_key)
                        .with_secret_access_key(s3_config.s3_secret_key)
                        .build()
                        .expect("cannot create S3 object store");

                    let store = Arc::new(s3_store) as Arc<dyn ObjectStore>;
                    let store = Arc::new(store) as Arc<dyn StorageBackend>;

                    let store = ObjectStorage::new(store, cfg.cache());
                    crate::obj_store::OBJECT_STORE
                        .set(store)
                        .unwrap_or_else(|_| panic!("cannot set object store"));
                },
                ObjectStoreType::CacheOnly => {
                    let store = crate::obj_store::CacheOnlyBackend::new();
                    let store = Arc::new(store) as Arc<dyn StorageBackend>;
                    
                    let store = ObjectStorage::new(store, cfg.cache());
                
                    
                    crate::obj_store::OBJECT_STORE
                        .set(store)
                        .unwrap_or_else(|_| panic!("cannot set object store"));
                    
                }
            }
            cfg
        }

    pub fn cache(&self) -> Cache {
        Cache::new(self.cache_dir.clone())
    }
}
