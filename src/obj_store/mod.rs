//! Object storage module for Subatomic
//!
//! Wraps around the S3 client to provide a more ergonomic interface for interacting with objects.

use crate::cache::{cache, Cache};
use crate::config::CONFIG;
use async_trait::async_trait;
use color_eyre::{eyre::eyre, Result};
use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjectPath;
use object_store::{ObjectStore, PutPayload};
use tracing::{debug, info};
use std::any::Any;
// use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
// pub mod local_backend;
// pub mod s3_backend;
#[async_trait]
pub trait StorageBackend: Send + Sync {
    
    async fn put_file(&self, key: &str, path: PathBuf) -> Result<()>;
    async fn put_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<()>;
    async fn get_object(&self, key: &str) -> Result<PathBuf>;
    async fn delete_object(&self, key: &str) -> Result<()>;
    
    fn file_name(&self, key: &str) -> String {
        key.split('/').last().unwrap().to_string()
    }
}

fn object_cache_dir() -> PathBuf {
    CONFIG.get().unwrap().cache_dir.clone()
}

#[async_trait]
impl StorageBackend for Arc<dyn ObjectStore> {
    async fn put_file(&self, key: &str, path: PathBuf) -> Result<()> {
        let s = tokio::fs::read(&path).await?;
        self.put(&ObjectPath::from(key), PutPayload::from_bytes(s.into())).await?;
        Ok(())
    }
    
    async fn put_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        self.put(&ObjectPath::from(key), PutPayload::from_bytes(bytes.into())).await?;
        Ok(())
    }
    
    async fn get_object(&self, key: &str) -> Result<PathBuf> {
        let result = self.get(&ObjectPath::from(key)).await?;
        let bytes = result.bytes().await?;
        
        let dest = object_cache_dir().join(self.file_name(key));
        info!(?dest, "Writing object to object cache");
        tokio::fs::write(&dest, bytes).await?;
        Ok(dest)
    }
    
    async fn delete_object(&self, key: &str) -> Result<()> {
        self.delete(&ObjectPath::from(key)).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ObjectStorage {
    pub backend: Arc<dyn StorageBackend>,
    pub cache: Arc<Cache>,
}

impl ObjectStorage {
    pub fn new(backend: Arc<dyn StorageBackend>, cache: Cache) -> Self {
        Self {
            backend,
            cache: Arc::new(cache),
        }
    }

    /// Get or download an object from the cache if it exists
    // #[tracing::instrument]
    pub async fn get(&self, key: &str) -> Result<PathBuf> {
            if let Some(path) = self.cache.get(key) {
                return Ok(path);
            }

            let path = self.backend.get_object(key).await?;
            debug!(?path, "Putting object in cache");
            let cache_path = self.cache.put(&key, &path).await?;
            Ok(cache_path)
        }

    pub async fn put(&self, key: &str, path: &PathBuf) -> Result<PathBuf> {
        debug!(?path, "Putting object");
        // let s = tokio::fs::read(path).await?;
        self.backend
            .put_file(key, path.clone())
            .await?;
        self.cache.put(key, path).await
    }

    pub async fn remove(&self, key: &str) -> Result<()> {
        self.backend.delete_object(key).await?;
        self.cache.remove(key).await
    }

    pub async fn refresh(&self, key: &str) -> Result<PathBuf> {
        self.cache.remove(key).await?;
        self.get(key).await
    }

    pub async fn put_bytes(&self, key: &str, bytes: Vec<u8>) -> Result<PathBuf> {
        self.backend.put_bytes(key, bytes).await?;
        self.cache
            .get(key)
            .ok_or_else(|| eyre!("object not found in cache"))
    }
}


pub static OBJECT_STORE: OnceLock<ObjectStorage> = OnceLock::new();

pub fn object_store() -> ObjectStorage {
    OBJECT_STORE.get().unwrap().clone()
}

/// A wrapper around a string that represents an object in the object store.
pub struct Object {
    key: String,
}

impl Object {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_owned(),
        }
    }

    pub async fn get(&self) -> Result<PathBuf> {
        object_store().get(&self.key).await
    }

    pub async fn put_file(&self, path: PathBuf) -> Result<PathBuf> {
        object_store().put(&self.key, &path).await
    }

    pub async fn delete(&self) -> Result<()> {
        object_store().remove(&self.key).await
    }

    pub async fn refresh(&self) -> Result<PathBuf> {
        object_store().refresh(&self.key).await
    }
}
