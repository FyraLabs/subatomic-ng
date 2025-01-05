use std::{env, path::PathBuf};

use crate::obj_store::OBJECT_STORE;
use color_eyre::Result;
/// Object storage cache for S3 objects
#[derive(Debug)]
pub struct Cache {
    /// The directory where objects are stored
    cache_dir: PathBuf,
}

pub fn cache() -> Cache {
    crate::config::CONFIG
        .get()
        .expect("config not initialized")
        .cache()
}

impl Cache {
    /// Create a new cache
    pub fn new(cache_dir: PathBuf) -> Self {
        if cache_dir.exists() {
            assert!(cache_dir.is_dir(), "cache_dir must be a directory");
        } else {
            std::fs::create_dir_all(&cache_dir).unwrap();
        }
        Self { cache_dir }
    }

    /// Get a cache entry
    ///
    /// This only gets the entry if it exists.
    /// If you would like to download the object when it doesn't exist, use `get_or_download`.
    pub fn get(&self, key: &str) -> Option<PathBuf> {
        let path = self.cache_dir.join(key);
        path.exists().then_some(path)
    }

    /// Set a cache entry from a file
    #[tracing::instrument]
    pub async fn put(&self, key: &str, path: &PathBuf) -> Result<()> {
        let dest = self.cache_dir.join(key);

        // make preceding directories if they don't exist
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::copy(&path, &dest).await?;
        tokio::fs::remove_file(path).await?;

        Ok(())
    }

    /// Get a cache entry, downloading it if it doesn't exist.
    ///
    /// This is probably what you want to use.
    ///
    /// This will download the object from the object store if it doesn't exist in the cache.
    #[tracing::instrument]
    pub async fn get_or_download(&self, key: &str) -> Result<PathBuf> {
        if let Some(path) = self.get(key) {
            return Ok(path);
        }

        let file = OBJECT_STORE.get().unwrap().get(key).await.unwrap();
        self.put(key, &file).await?;

        Ok(self.cache_dir.join(key))
    }

    #[tracing::instrument]
    pub async fn put_and_upload(&self, key: &str, path: PathBuf) -> Result<()> {
        let path_clone = path.clone();

        if env::var("NO_UPLOAD").is_err() {
            tracing::debug!("uploading to object store");
            OBJECT_STORE.get().unwrap().put_file(key, path).await?;
        }
        self.put(key, &path_clone).await?;

        Ok(())
    }

    /// Refresh a cache entry, redownloading it from the object store
    #[tracing::instrument]
    pub async fn refresh(&self, key: &str) -> Result<PathBuf> {
        let file = OBJECT_STORE.get().unwrap().get(key).await.unwrap();
        self.put(key, &file).await?;

        Ok(self.cache_dir.join(key))
    }

    #[tracing::instrument]
    pub async fn remove(&self, key: &str) -> Result<()> {
        let path = self.cache_dir.join(key);
        tokio::fs::remove_file(&path).await?;

        // Remove empty parent directories
        let mut current = path.parent();
        while let Some(dir) = current {
            if dir == self.cache_dir {
                break;
            }

            match tokio::fs::read_dir(dir).await {
                Ok(mut entries) => {
                    if entries.next_entry().await?.is_some() {
                        break; // Directory not empty, stop here
                    }
                    tokio::fs::remove_dir(dir).await?;
                }
                Err(e) => {
                    tracing::error!("failed to read dir while clearing: {:?}", e);
                    break;
                }
            }
            current = dir.parent();
        }

        Ok(())
    }

    pub async fn list_cached(&self) -> Result<Vec<String>> {
        // read 2 levels deep to get the actual keys
        let files = walkdir::WalkDir::new(&self.cache_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                e.path()
                    .strip_prefix(&self.cache_dir)
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<String>>();

        Ok(files)
    }

    /// Delete both the object from the object store and the cache
    ///
    /// You shouldn't need to use this unless you're hiding something
    #[tracing::instrument]
    pub async fn remove_upstream(&self, key: &str) -> Result<()> {
        OBJECT_STORE.get().unwrap().delete(key).await?;
        self.remove(key).await
    }
}
