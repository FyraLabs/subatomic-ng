//! Object storage module for Subatomic
//!
//! Wraps around the S3 client to provide a more ergonomic interface for interacting with objects.

use crate::config::CONFIG;
use color_eyre::Result;
use s3::{creds::Credentials, Bucket};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct ObjectStore {
    bucket: Box<Bucket>,
}

pub static OBJECT_STORE: OnceLock<ObjectStore> = OnceLock::new();

fn get_cache_dir(key: &str) -> PathBuf {
    CONFIG.get().unwrap().object_cache_dir.clone().join(key)
}

impl ObjectStore {
    fn new(
        region: s3::Region,
        creds: Credentials,
        bucket_name: &str,
    ) -> Result<Self, s3::error::S3Error> {
        let bucket = Bucket::new(bucket_name, region, creds)?.with_path_style();

        Ok(Self { bucket })
    }

    pub fn bucket(&self) -> &Bucket {
        &self.bucket
    }

    pub fn init(region: s3::Region, creds: Credentials, bucket_name: &str) {
        OBJECT_STORE
            .set(Self::new(region, creds, bucket_name).unwrap())
            .unwrap();
    }

    pub async fn get(&self, key: &str) -> Result<PathBuf> {
        let obj = self.bucket.get_object(key).await?;
        let key_filename = key.split('/').last().unwrap();
        let dest = get_cache_dir(key_filename);
        tokio::fs::write(&dest, obj.bytes()).await?;
        Ok(dest)
    }

    pub async fn put_file(&self, key: &str, path: PathBuf) -> Result<()> {
        let bytes = tokio::fs::read(&path).await?;
        self.bucket.put_object(key, &bytes).await?;
        Ok(())
    }

    pub async fn put_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        self.bucket.put_object(key, &bytes).await?;
        Ok(())
    }

    /// Delete an object from the object store,
    /// given its key.
    ///
    /// Hopefully you will never need to use this.
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.bucket.delete_object(key).await?;
        Ok(())
    }
}
