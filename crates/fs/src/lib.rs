use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait Fs: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>>;
    async fn read_to_string(&self, path: &Path) -> Result<String>;
    async fn write(&self, path: &Path, data: &[u8]) -> Result<()>;
    async fn create_dir(&self, path: &Path) -> Result<()>;
    async fn remove_file(&self, path: &Path) -> Result<()>;
    async fn remove_dir(&self, path: &Path) -> Result<()>;
    async fn copy(&self, from: &Path, to: &Path) -> Result<()>;
    async fn exists(&self, path: &Path) -> bool;
    async fn is_dir(&self, path: &Path) -> bool;
    async fn is_file(&self, path: &Path) -> bool;
    async fn metadata(&self, path: &Path) -> Result<FsMetadata>;
    async fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>>;
}

#[derive(Debug, Clone)]
pub struct FsMetadata {
    pub len: u64,
    pub is_dir: bool,
    pub is_file: bool,
    pub modified: Option<std::time::SystemTime>,
    pub created: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: std::path::PathBuf,
    pub file_name: String,
    pub is_dir: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fs_trait_exists() {
        fn _assert_send_sync<T: Send + Sync>() {}
        fn _assert_fs_trait_bounds<T: Fs>() {
            _assert_send_sync::<T>();
        }
    }

    #[test]
    fn test_fs_metadata_creation() {
        let metadata = FsMetadata {
            len: 1024,
            is_dir: false,
            is_file: true,
            modified: None,
            created: None,
        };
        assert_eq!(metadata.len, 1024);
        assert!(!metadata.is_dir);
        assert!(metadata.is_file);
    }

    #[test]
    fn test_dir_entry_creation() {
        let entry = DirEntry {
            path: std::path::PathBuf::from("/test/file.txt"),
            file_name: "file.txt".to_string(),
            is_dir: false,
        };
        assert_eq!(entry.file_name, "file.txt");
        assert!(!entry.is_dir);
    }
}
