use crate::error::Result;
use crate::linking::Linker;
use crate::store::StoreManager;
use crate::{
    context::StowawayContext, lifecycle::LifecyclePhase, linking::FileSystemLinker,
    store::FileSystemStoreManager, StowawayError,
};
use tracing::{debug, info};

pub struct LinkPhase;

impl LifecyclePhase for LinkPhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()> {
        let linker = FileSystemLinker;
        let store_manager = FileSystemStoreManager::new()?;

        if context.dry_run {
            info!(
                symlink_count = context.files.len(),
                "DRY RUN: Would create symlinks"
            );
            for file_entry in &context.files {
                debug!(
                    target = %file_entry.target_path.display(),
                    source = %file_entry.source_path.display(),
                    "Would create symlink"
                );
            }
            return Ok(());
        }

        let store_hash = context
            .store_hash
            .as_ref()
            .ok_or_else(|| StowawayError::Store("No store hash available".to_string()))?;

        let store_path = store_manager.get_store_path(store_hash);

        for file_entry in &context.files {
            let store_file_path = store_path.join(&file_entry.relative_path);

            // Verify the store file exists before creating symlink
            if !store_file_path.exists() {
                return Err(StowawayError::Store(format!(
                    "Store file does not exist: {}",
                    store_file_path.display()
                )));
            }

            linker.create_symlink(&store_file_path, &file_entry.target_path)?;
        }

        info!(symlink_count = context.files.len(), "Created symlinks");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StowawayConfig;
    use crate::context::StowawayContext;
    use crate::file_entry::FileEntry;
    use crate::store::{FileSystemStoreManager, StoreManager, StoreVersion};
    use std::fs;
    use std::os::unix::fs::symlink;
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
        }
    }

    fn setup_store_with_files(store_hash: &str, files: Vec<(&str, &str)>) -> TempDir {
        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.create_version(store_hash).unwrap();

        for (relative_path, content) in files {
            let store_file_path = store_path.join(relative_path);
            if let Some(parent) = store_file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&store_file_path, content).unwrap();
        }

        TempDir::new().unwrap()
    }

    #[test]
    fn test_dry_run_vs_regular() {
        let temp_dir = TempDir::new().unwrap();
        let _store_temp =
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
    fn test_symlinks_files() {
        let temp_dir = TempDir::new().unwrap();
        let _store_temp = setup_store_with_files(
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
    fn test_handles_symlink_creation_failure() {
        let temp_dir = TempDir::new().unwrap();
        let _store_temp =
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
    fn test_creates_target_directories() {
        let temp_dir = TempDir::new().unwrap();
        let _store_temp = setup_store_with_files(
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
    fn test_empty_files_list() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec![], false);

        let phase = LinkPhase;
        let result = phase.execute(&mut context);

        assert!(result.is_ok());
    }

    #[test]
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
