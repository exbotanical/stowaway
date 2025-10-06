use crate::error::Result;
use crate::linking::Linker;
use crate::log_dryrun;
use crate::store::StoreManager;
use crate::{
    context::StowawayContext, lifecycle::LifecyclePhase, linking::FileSystemLinker,
    store::FileSystemStoreManager, StowawayError,
};
use tracing::info;

/// The link phase is among the last phases and is where we actually symlink the store files to their targets.
pub struct LinkPhase;

impl LifecyclePhase for LinkPhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()> {
        use crate::context::OperationMode;

        let linker = FileSystemLinker;
        let store_manager = FileSystemStoreManager::new()?;

        if context.dry_run {
            log_dryrun!(
                "Would create symlinks (symlink_count={})",
                context.files.len(),
            );
            for file_entry in &context.files {
                info!(
                    target = %file_entry.target_path.display(),
                    source = %file_entry.source_path.display(),
                    "Would create symlink"
                );
            }
            return Ok(());
        }

        match &context.operation_mode {
            OperationMode::Verify => self.verify_symlinks(context, &linker, &store_manager),
            OperationMode::Create => self.create_symlinks(context, &linker, &store_manager),
            OperationMode::Replace => self.replace_symlinks(context, &linker, &store_manager),
        }
    }
}

impl LinkPhase {
    fn verify_symlinks(
        &self,
        context: &StowawayContext,
        _linker: &FileSystemLinker,
        store_manager: &FileSystemStoreManager,
    ) -> Result<()> {
        info!("Verifying existing symlinks");

        let store_hash = context
            .store_hash
            .as_ref()
            .ok_or_else(|| StowawayError::Store("No store hash available".to_string()))?;

        let store_path = store_manager.get_store_path(store_hash);

        for file_entry in &context.files {
            if !file_entry.target_path.is_symlink() {
                return Err(StowawayError::Linking(format!(
                    "Expected symlink not found: {}",
                    file_entry.target_path.display()
                )));
            }

            let store_file_path = store_path.join(&file_entry.relative_path);
            let current_target = std::fs::read_link(&file_entry.target_path)?;
            if current_target != store_file_path {
                return Err(StowawayError::Linking(format!(
                    "Symlink points to wrong location: {} -> {} (expected: {})",
                    file_entry.target_path.display(),
                    current_target.display(),
                    store_file_path.display()
                )));
            }
        }

        info!(symlink_count = context.files.len(), "Verified symlinks");
        Ok(())
    }

    fn create_symlinks(
        &self,
        context: &StowawayContext,
        linker: &FileSystemLinker,
        store_manager: &FileSystemStoreManager,
    ) -> Result<()> {
        info!("Creating new symlinks");

        let store_hash = context
            .store_hash
            .as_ref()
            .ok_or_else(|| StowawayError::Store("No store hash available".to_string()))?;

        let store_path = store_manager.get_store_path(store_hash);

        for file_entry in &context.files {
            let store_file_path = store_path.join(&file_entry.relative_path);

            if !store_file_path.exists() {
                return Err(StowawayError::Store(format!(
                    "Store file does not exist: {}",
                    store_file_path.display()
                )));
            }

            linker.create_symlink(&store_file_path, &file_entry.target_path)?;
        }

        store_manager.set_current_version(&crate::store::StoreVersion {
            hash: store_hash.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_dir: context.source_dir.clone(),
            target_dir: context.target_dir.clone(),
        })?;

        info!(symlink_count = context.files.len(), "Created symlinks");
        Ok(())
    }

    fn replace_symlinks(
        &self,
        context: &StowawayContext,
        linker: &FileSystemLinker,
        store_manager: &FileSystemStoreManager,
    ) -> Result<()> {
        info!("Replacing existing symlinks");
        for file_entry in &context.files {
            if file_entry.target_path.exists() {
                std::fs::remove_file(&file_entry.target_path)?;
            }
        }

        self.create_symlinks(context, linker, store_manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StowawayConfig;
    use crate::context::StowawayContext;
    use crate::file_entry::FileEntry;
    use crate::store::{FileSystemStoreManager, StoreManager, StoreVersion};
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_context(
        temp_dir: &TempDir,
        files: Vec<(&str, &str)>,
        dry_run: bool,
    ) -> StowawayContext {
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        let mut file_entries = Vec::new();
        for (relative_path, content) in files {
            let source_path = source_dir.join(relative_path);
            if let Some(parent) = source_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&source_path, content).unwrap();

            let target_path = target_dir.join(relative_path);
            file_entries.push(FileEntry {
                source_path,
                relative_path: relative_path.into(),
                target_path,
                should_interpolate: false,
            });
        }

        let config = StowawayConfig::default();

        StowawayContext {
            config,
            source_dir,
            target_dir,
            dry_run,
            files: file_entries,
            store_hash: Some("test_hash_123".to_string()),
            operation_mode: crate::context::OperationMode::Create,
            current_version: None,
            target_hash: None,
        }
    }

    fn setup_store_with_files(store_hash: &str, files: Vec<(&str, &str)>) -> () {
        let store_manager = FileSystemStoreManager::new().unwrap();

        let version = StoreVersion {
            hash: store_hash.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_dir: std::path::PathBuf::from("/test/source"),
            target_dir: std::path::PathBuf::from("/test/target"),
        };

        let store_path = store_manager.create_version(&version).unwrap();

        for (relative_path, content) in files {
            let store_file_path = store_path.join(relative_path);
            if let Some(parent) = store_file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&store_file_path, content).unwrap();
        }
    }

    #[test]
    #[serial]
    fn test_dry_run_vs_regular() {
        let temp_dir = TempDir::new().unwrap();
        setup_store_with_files("test_hash_123", vec![("config.txt", "test content")]);

        let mut dry_context =
            create_test_context(&temp_dir, vec![("config.txt", "test content")], true);

        let phase = LinkPhase;
        phase.execute(&mut dry_context).unwrap();

        let target_file = dry_context.target_dir.join("config.txt");
        assert!(!target_file.exists());

        let mut regular_context =
            create_test_context(&temp_dir, vec![("config.txt", "test content")], false);

        phase.execute(&mut regular_context).unwrap();

        let target_file = regular_context.target_dir.join("config.txt");
        assert!(target_file.exists());
        assert!(target_file.is_symlink());
    }

    #[test]
    #[serial]
    fn test_symlinks_files() {
        let temp_dir = TempDir::new().unwrap();
        setup_store_with_files(
            "test_hash_456",
            vec![
                ("config.txt", "config content"),
                ("nested/file.conf", "nested content"),
            ],
        );

        let mut context = create_test_context(
            &temp_dir,
            vec![
                ("config.txt", "config content"),
                ("nested/file.conf", "nested content"),
            ],
            false,
        );
        context.store_hash = Some("test_hash_456".to_string());

        let phase = LinkPhase;
        phase.execute(&mut context).unwrap();

        let config_link = context.target_dir.join("config.txt");
        assert!(config_link.exists());
        assert!(config_link.is_symlink());

        let nested_link = context.target_dir.join("nested/file.conf");
        assert!(nested_link.exists());
        assert!(nested_link.is_symlink());

        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.get_store_path("test_hash_456");

        let config_target = fs::read_link(&config_link).unwrap();
        assert_eq!(config_target, store_path.join("config.txt"));

        let nested_target = fs::read_link(&nested_link).unwrap();
        assert_eq!(nested_target, store_path.join("nested/file.conf"));
    }

    #[test]
    #[serial]
    fn test_handles_missing_store_hash() {
        let temp_dir = TempDir::new().unwrap();
        let mut context =
            create_test_context(&temp_dir, vec![("config.txt", "test content")], false);
        context.store_hash = None;

        let phase = LinkPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
        match result.unwrap_err() {
            StowawayError::Store(msg) => {
                assert_eq!(msg, "No store hash available");
            }
            _ => panic!("Expected Store error"),
        }
    }

    #[test]
    #[serial]
    fn test_handles_missing_store_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut context =
            create_test_context(&temp_dir, vec![("config.txt", "test content")], false);
        context.store_hash = Some("nonexistent_hash".to_string());

        let phase = LinkPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_handles_symlink_creation_failure() {
        let temp_dir = TempDir::new().unwrap();
        setup_store_with_files("test_hash_789", vec![("config.txt", "test content")]);

        let mut context =
            create_test_context(&temp_dir, vec![("config.txt", "test content")], false);
        context.store_hash = Some("test_hash_789".to_string());

        let target_file = context.target_dir.join("config.txt");
        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&target_file, "existing content").unwrap();

        let phase = LinkPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_creates_target_directories() {
        let temp_dir = TempDir::new().unwrap();
        setup_store_with_files(
            "test_hash_nested",
            vec![("deep/nested/config.txt", "nested content")],
        );

        let mut context = create_test_context(
            &temp_dir,
            vec![("deep/nested/config.txt", "nested content")],
            false,
        );
        context.store_hash = Some("test_hash_nested".to_string());

        let phase = LinkPhase;
        phase.execute(&mut context).unwrap();

        let nested_link = context.target_dir.join("deep/nested/config.txt");
        assert!(nested_link.exists());
        assert!(nested_link.is_symlink());

        assert!(context.target_dir.join("deep").exists());
        assert!(context.target_dir.join("deep/nested").exists());
    }

    #[test]
    #[serial]
    fn test_multiple_files_partial_failure() {
        let temp_dir = TempDir::new().unwrap();
        let _store_temp = setup_store_with_files(
            "test_hash_partial",
            vec![("good.txt", "good content"), ("bad.txt", "bad content")],
        );

        let mut context = create_test_context(
            &temp_dir,
            vec![("good.txt", "good content"), ("bad.txt", "bad content")],
            false,
        );
        context.store_hash = Some("test_hash_partial".to_string());

        let bad_target = context.target_dir.join("bad.txt");
        if let Some(parent) = bad_target.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&bad_target, "existing").unwrap();

        let phase = LinkPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());

        // The good file gets processed first and succeeds before hitting the bad file
        let good_target = context.target_dir.join("good.txt");
        assert!(good_target.exists());
        assert!(good_target.is_symlink());

        // The bad file should still exist as a regular file (not symlink)
        let bad_target = context.target_dir.join("bad.txt");
        assert!(bad_target.exists());
        assert!(!bad_target.is_symlink());
    }

    #[test]
    #[serial]
    fn test_dry_run_with_missing_store_hash() {
        let temp_dir = TempDir::new().unwrap();
        let mut context =
            create_test_context(&temp_dir, vec![("config.txt", "test content")], true);
        context.store_hash = None;

        let phase = LinkPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_empty_files_list() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec![], false);

        let phase = LinkPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_symlink_points_to_correct_store_location() {
        let temp_dir = TempDir::new().unwrap();
        let test_content = "specific test content";
        let _store_temp =
            setup_store_with_files("test_hash_verify", vec![("verify.txt", test_content)]);

        let mut context = create_test_context(&temp_dir, vec![("verify.txt", test_content)], false);
        context.store_hash = Some("test_hash_verify".to_string());

        let phase = LinkPhase;
        phase.execute(&mut context).unwrap();

        let target_link = context.target_dir.join("verify.txt");
        assert!(target_link.is_symlink());

        let content = fs::read_to_string(&target_link).unwrap();
        assert_eq!(content, test_content);

        let store_manager = FileSystemStoreManager::new().unwrap();
        let expected_target = store_manager
            .get_store_path("test_hash_verify")
            .join("verify.txt");
        let actual_target = fs::read_link(&target_link).unwrap();
        assert_eq!(actual_target, expected_target);
    }
}
