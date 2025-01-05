use std::path::PathBuf;

use crate::{cache::Cache, obj_store::ObjectStore};
use clap::Parser;
use std::sync::OnceLock;

fn fallback_region() -> String {
    "us-west-2".to_string()
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Parser, Debug, Clone)]
pub struct Config {
    #[clap(long, env = "SURREAL_HOST")]
    pub host: String,

    #[clap(long, env = "SURREAL_DB", default_value = "subatomic")]
    pub surreal_db: String,

    #[clap(long, env = "SURREAL_NS", default_value = "subatomic")]
    pub surreal_ns: String,

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
}

impl Config {
    pub fn init() -> Self {
        let cfg = Self::parse();
        CONFIG.set(cfg.clone()).expect("cannot read CLI configs");
        let region = s3::Region::Custom {
            region: cfg.s3_region.clone(),
            endpoint: cfg.s3_endpoint.clone(),
        };
        tracing::info!("Initializing object store");
        let creds = s3::creds::Credentials::new(
            Some(&cfg.s3_access_key),
            Some(&cfg.s3_secret_key),
            None,
            None,
            None,
        )
        .expect("cannot create credentials");
        ObjectStore::init(region, creds, &cfg.s3_bucket);
        cfg
    }

    pub fn cache(&self) -> Cache {
        Cache::new(self.cache_dir.clone())
    }
}
