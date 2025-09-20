use crate::error::{Result, StowawayError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreVersion {
    pub hash: String,
    pub timestamp: u64,
    pub source_dir: PathBuf,
}

pub trait StoreManager {
    fn create_version(&self, content_hash: &str) -> Result<PathBuf>;
    fn get_current_version(&self) -> Result<Option<StoreVersion>>;
    fn set_current_version(&self, version: &StoreVersion) -> Result<()>;
    fn get_store_path(&self, hash: &str) -> PathBuf;
    fn cleanup_old_versions(&self, keep_count: usize) -> Result<()>;
}

pub struct FileSystemStoreManager {
    store_root: PathBuf,
}

impl FileSystemStoreManager {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            StowawayError::Store("Could not determine home directory".to_string())
        })?;

        let store_root = home_dir.join(".stowaway");

        Ok(Self { store_root })
    }

    fn ensure_store_directory(&self) -> Result<()> {
        if !self.store_root.exists() {
            std::fs::create_dir_all(&self.store_root)?;
        }
        Ok(())
    }

    fn session_file_path(&self) -> PathBuf {
        self.store_root.join("session.json")
    }
}

impl StoreManager for FileSystemStoreManager {
    fn create_version(&self, content_hash: &str) -> Result<PathBuf> {
        self.ensure_store_directory()?;

        let store_path = self.get_store_path(content_hash);

        if !store_path.exists() {
            std::fs::create_dir_all(&store_path)?;
        }

        Ok(store_path)
    }

    fn get_current_version(&self) -> Result<Option<StoreVersion>> {
        let session_file = self.session_file_path();

        if !session_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&session_file)?;
        let version: StoreVersion = serde_json::from_str(&content)
            .map_err(|e| StowawayError::Store(format!("Failed to parse session file: {}", e)))?;

        Ok(Some(version))
    }

    fn set_current_version(&self, version: &StoreVersion) -> Result<()> {
        self.ensure_store_directory()?;

        let session_file = self.session_file_path();
        let content = serde_json::to_string_pretty(version)
            .map_err(|e| StowawayError::Store(format!("Failed to serialize version: {}", e)))?;

        std::fs::write(&session_file, content)?;
        Ok(())
    }

    fn get_store_path(&self, hash: &str) -> PathBuf {
        self.store_root.join("store").join(hash)
    }

    fn cleanup_old_versions(&self, keep_count: usize) -> Result<()> {
        let store_dir = self.store_root.join("store");

        if !store_dir.exists() {
            return Ok(());
        }

        let mut versions = Vec::new();
        for entry in std::fs::read_dir(&store_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(created) = metadata.created() {
                            versions.push((name.to_string(), created, entry.path()));
                        }
                    }
                }
            }
        }

        versions.sort_by(|a, b| b.1.cmp(&a.1));

        for (_, _, path) in versions.iter().skip(keep_count) {
            std::fs::remove_dir_all(path)?;
        }

        Ok(())
    }
}

impl Default for FileSystemStoreManager {
    fn default() -> Self {
        Self::new().expect("Failed to create store manager")
    }
}

pub fn calculate_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_calculate_content_hash() {
        let content = "test content";
        let hash1 = calculate_content_hash(content);
        let hash2 = calculate_content_hash(content);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 produces 64 character hex string
    }

    #[test]
    fn test_different_content_different_hash() {
        let hash1 = calculate_content_hash("content1");
        let hash2 = calculate_content_hash("content2");

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_store_version_creation() {
        let version = StoreVersion {
            hash: "abc123".to_string(),
            timestamp: 1234567890,
            source_dir: PathBuf::from("/source"),
        };

        assert_eq!(version.hash, "abc123");
        assert_eq!(version.timestamp, 1234567890);
        assert_eq!(version.source_dir, PathBuf::from("/source"));
    }
}
