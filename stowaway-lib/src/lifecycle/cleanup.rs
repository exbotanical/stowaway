use crate::context::StowawayContext;
use crate::error::Result;
use crate::lifecycle::LifecyclePhase;
use crate::log_dryrun;
use crate::store::{FileSystemStoreManager, StoreManager};
use tracing::{debug, info, warn};

/// The cleanup phase removes old and temporary resources created during store state transitions e.g. old store versions
pub struct CleanupPhase;

impl LifecyclePhase for CleanupPhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()> {
        if context.dry_run {
            log_dryrun!("Would clean up old store versions");
            return Ok(());
        }

        info!("Cleaning up old store versions");

        let store_manager = FileSystemStoreManager::new()?;

        let all_versions_before = store_manager.list_all_versions().unwrap_or_default();
        let version_count_before = all_versions_before.len();

        if version_count_before <= 1 {
            debug!("No old versions to clean up");
            return Ok(());
        }

        match store_manager.cleanup_old_versions(1) {
            Ok(()) => {
                let all_versions_after = store_manager.list_all_versions().unwrap_or_default();
                let version_count_after = all_versions_after.len();
                let cleaned_count = version_count_before.saturating_sub(version_count_after);

                info!(
                    versions_before = version_count_before,
                    versions_after = version_count_after,
                    cleaned_count = cleaned_count,
                    "Successfully cleaned up old store versions"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to clean up old store versions, but operation completed successfully"
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StowawayConfig;
    use crate::context::OperationMode;
    use crate::store::StoreVersion;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_context(temp_dir: &TempDir, dry_run: bool) -> StowawayContext {
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        let config = StowawayConfig {
            variables: HashMap::new(),
            ..Default::default()
        };

        StowawayContext {
            config,
            source_dir,
            target_dir,
            dry_run,
            files: Vec::new(),
            store_hash: Some("current_hash".to_string()),
            operation_mode: OperationMode::Create,
            current_version: None,
            target_hash: Some("current_hash".to_string()),
        }
    }

    fn create_test_store_versions(store_manager: &FileSystemStoreManager, count: usize) {
        for i in 0..count {
            let version = StoreVersion {
                hash: format!("hash_{}", i),
                timestamp: 1000 + i as u64,
                source_dir: PathBuf::from("/test/source"),
                target_dir: PathBuf::from("/test/target"),
            };
            store_manager.create_version(&version).unwrap();
        }
    }

    #[test]
    #[serial]
    fn test_cleanup_removes_old_versions() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("STOWAWAY_STORE_DIR", temp_dir.path().join("store"));

        let store_manager = FileSystemStoreManager::new().unwrap();
        let mut context = create_test_context(&temp_dir, false);

        create_test_store_versions(&store_manager, 5);

        let versions_before = store_manager.list_all_versions().unwrap();
        assert_eq!(versions_before.len(), 5);

        let phase = CleanupPhase;
        phase.execute(&mut context).unwrap();

        let versions_after = store_manager.list_all_versions().unwrap();
        assert_eq!(versions_after.len(), 1);

        std::env::remove_var("STOWAWAY_STORE_DIR");
    }

    #[test]
    #[serial]
    fn test_cleanup_dry_run_does_nothing() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("STOWAWAY_STORE_DIR", temp_dir.path().join("store"));

        let store_manager = FileSystemStoreManager::new().unwrap();

        create_test_store_versions(&store_manager, 3);

        let versions_before = store_manager.list_all_versions().unwrap();
        assert_eq!(versions_before.len(), 3);

        let phase = CleanupPhase;
        let mut context = create_test_context(&temp_dir, true);

        phase.execute(&mut context).unwrap();

        let versions_after = store_manager.list_all_versions().unwrap();
        assert_eq!(versions_after.len(), 3);

        std::env::remove_var("STOWAWAY_STORE_DIR");
    }

    #[test]
    #[serial]
    fn test_cleanup_with_single_version_does_nothing() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("STOWAWAY_STORE_DIR", temp_dir.path().join("store"));

        let store_manager = FileSystemStoreManager::new().unwrap();
        let mut context = create_test_context(&temp_dir, false);

        create_test_store_versions(&store_manager, 1);

        let versions_before = store_manager.list_all_versions().unwrap();
        assert_eq!(versions_before.len(), 1);

        let phase = CleanupPhase;
        phase.execute(&mut context).unwrap();

        let versions_after = store_manager.list_all_versions().unwrap();
        assert_eq!(versions_after.len(), 1);

        std::env::remove_var("STOWAWAY_STORE_DIR");
    }

    #[test]
    #[serial]
    fn test_cleanup_with_no_versions_succeeds() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("STOWAWAY_STORE_DIR", temp_dir.path().join("store"));

        let store_manager = FileSystemStoreManager::new().unwrap();
        let mut context = create_test_context(&temp_dir, false);

        let versions_before = store_manager.list_all_versions().unwrap();
        assert_eq!(versions_before.len(), 0);

        let phase = CleanupPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_ok());

        std::env::remove_var("STOWAWAY_STORE_DIR");
    }

    #[test]
    #[serial]
    fn test_cleanup_keeps_most_recent_version() {
        let temp_dir = TempDir::new().unwrap();

        std::env::set_var("STOWAWAY_STORE_DIR", temp_dir.path().join("store"));

        let store_manager = FileSystemStoreManager::new().unwrap();
        let mut context = create_test_context(&temp_dir, false);

        let old_version = StoreVersion {
            hash: "old_hash".to_string(),
            timestamp: 1000,
            source_dir: PathBuf::from("/test/source"),
            target_dir: PathBuf::from("/test/target"),
        };
        let new_version = StoreVersion {
            hash: "new_hash".to_string(),
            timestamp: 2000,
            source_dir: PathBuf::from("/test/source"),
            target_dir: PathBuf::from("/test/target"),
        };

        store_manager.create_version(&old_version).unwrap();
        store_manager.create_version(&new_version).unwrap();

        let phase = CleanupPhase;
        phase.execute(&mut context).unwrap();

        let versions_after = store_manager.list_all_versions().unwrap();
        assert_eq!(versions_after.len(), 1);

        assert_eq!(versions_after[0].hash, "new_hash");
        assert_eq!(versions_after[0].timestamp, 2000);

        std::env::remove_var("STOWAWAY_STORE_DIR");
    }
}
