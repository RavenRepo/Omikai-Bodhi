use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use theasus_fs::{DirEntry, Fs, FsMetadata};
use tokio::fs;

pub struct RealFs;

impl RealFs {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RealFs {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Fs for RealFs {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        Ok(fs::read(path).await?)
    }

    async fn read_to_string(&self, path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path).await?)
    }

    async fn write(&self, path: &Path, data: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(fs::write(path, data).await?)
    }

    async fn create_dir(&self, path: &Path) -> Result<()> {
        Ok(fs::create_dir_all(path).await?)
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        Ok(fs::remove_file(path).await?)
    }

    async fn remove_dir(&self, path: &Path) -> Result<()> {
        Ok(fs::remove_dir(path).await?)
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(fs::copy(from, to).await.map(|_| ())?)
    }

    async fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    async fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    async fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    async fn metadata(&self, path: &Path) -> Result<FsMetadata> {
        let meta = fs::metadata(path).await?;
        Ok(FsMetadata {
            len: meta.len(),
            is_dir: meta.is_dir(),
            is_file: meta.is_file(),
            modified: meta.modified().ok(),
            created: meta.created().ok(),
        })
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>> {
        let mut entries = Vec::new();
        let mut dir = fs::read_dir(path).await?;
        while let Some(entry) = dir.next_entry().await? {
            entries.push(DirEntry {
                path: entry.path(),
                file_name: entry.file_name().to_string_lossy().to_string(),
                is_dir: entry.file_type().await?.is_dir(),
            });
        }
        Ok(entries)
    }
}
