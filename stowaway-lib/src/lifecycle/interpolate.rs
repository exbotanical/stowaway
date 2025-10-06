use crate::error::Result;
use crate::{
    context::StowawayContext,
    interpolation::{Interpolator, VariableInterpolator},
    lifecycle::LifecyclePhase,
    store::{FileSystemStoreManager, StoreManager},
    StowawayError,
};
use tracing::info;

/// The interpolate phase is where we take the user's stowaway variables and shared state and apply it to their source files.
/// The result of the interpolate phase is the store fileset, which we then symlink to their home-relative (or in some scenarios, root) counterparts.
/// This phase is also responsible for creating the new store version, which we defer to as late in the process as possible.
pub struct InterpolatePhase;

impl LifecyclePhase for InterpolatePhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()> {
        use crate::context::OperationMode;

        info!("Interpolating variables");

        let store_manager = FileSystemStoreManager::new()?;
        let interpolator = VariableInterpolator;

        match &context.operation_mode {
            OperationMode::Verify => {
                info!("Verify mode: skipping interpolation");
                return Ok(());
            }
            OperationMode::Create | OperationMode::Replace => {
                let target_hash = context
                    .target_hash
                    .as_ref()
                    .ok_or_else(|| StowawayError::Store("No target hash available".to_string()))?;

                info!(target_hash = %target_hash, "Creating new store version");

                // 1. Create the new store version
                let version = crate::store::StoreVersion {
                    hash: target_hash.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    source_dir: context.source_dir.clone(),
                    target_dir: context.target_dir.clone(),
                };

                let store_path = store_manager.create_version(&version)?;
                context.store_hash = Some(target_hash.clone());

                // 2. Populate the store with the source files, performing interpolations where applicable.
                for file_entry in &context.files {
                    let store_file_path = store_path.join(&file_entry.relative_path);

                    if let Some(parent) = store_file_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    if file_entry.should_interpolate {
                        let content = std::fs::read_to_string(&file_entry.source_path)?;
                        let interpolated =
                            interpolator.interpolate(&content, &context.config.variables)?;
                        std::fs::write(&store_file_path, interpolated)?;
                    } else {
                        std::fs::copy(&file_entry.source_path, &store_file_path)?;
                    }
                }

                info!(store_hash = %target_hash, "Created and populated new store version");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StowawayConfig;
    use crate::context::StowawayContext;
    use crate::file_entry::FileEntry;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_context(temp_dir: &TempDir, files: Vec<(&str, &str, bool)>) -> StowawayContext {
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        let mut file_entries = Vec::new();
        let mut all_content = String::new();
        for (relative_path, content, should_interpolate) in files {
            let source_path = source_dir.join(relative_path);
            if let Some(parent) = source_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&source_path, content).unwrap();
            all_content.push_str(content);

            let target_path = target_dir.join(relative_path);
            file_entries.push(FileEntry {
                source_path,
                relative_path: relative_path.into(),
                target_path,
                should_interpolate,
            });
        }

        let mut variables = HashMap::new();
        variables.insert("username".to_string(), "testuser".to_string());
        variables.insert("editor".to_string(), "vim".to_string());

        let config = StowawayConfig {
            variables,
            ..Default::default()
        };

        let target_hash = crate::store::calculate_content_hash(&all_content);

        StowawayContext {
            config,
            source_dir,
            target_dir,
            dry_run: false,
            files: file_entries,
            store_hash: None,
            operation_mode: crate::context::OperationMode::Create,
            current_version: None,
            target_hash: Some(target_hash),
        }
    }

    #[test]
    #[serial]
    fn test_interpolates_all_vars_in_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(
            &temp_dir,
            vec![("config.txt", "User: @username@\nEditor: @editor@", true)],
        );

        let phase = InterpolatePhase;
        phase.execute(&mut context).unwrap();

        assert!(context.store_hash.is_some());

        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.get_store_path(context.store_hash.as_ref().unwrap());
        let interpolated_file = store_path.join("config.txt");

        assert!(interpolated_file.exists());
        let content = fs::read_to_string(&interpolated_file).unwrap();
        assert_eq!(content, "User: testuser\nEditor: vim");
    }

    #[test]
    #[serial]
    fn test_handles_missing_var_mappings() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(
            &temp_dir,
            vec![(
                "config.txt",
                "User: @username@\nMissing: @nonexistent@",
                true,
            )],
        );

        let phase = InterpolatePhase;
        phase.execute(&mut context).unwrap();

        assert!(context.store_hash.is_some());

        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.get_store_path(context.store_hash.as_ref().unwrap());
        let interpolated_file = store_path.join("config.txt");

        assert!(interpolated_file.exists());
        let content = fs::read_to_string(&interpolated_file).unwrap();
        assert_eq!(content, "User: testuser\nMissing: @nonexistent@");
    }

    #[test]
    #[serial]
    fn test_copies_non_interpolated_files() {
        let temp_dir = TempDir::new().unwrap();
        let original_content = "This should not be interpolated: @username@";
        let mut context =
            create_test_context(&temp_dir, vec![("binary.dat", original_content, false)]);

        let phase = InterpolatePhase;
        phase.execute(&mut context).unwrap();

        assert!(context.store_hash.is_some());

        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.get_store_path(context.store_hash.as_ref().unwrap());
        let copied_file = store_path.join("binary.dat");

        assert!(copied_file.exists());
        let content = fs::read_to_string(&copied_file).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    #[serial]
    fn test_mixed_interpolated_and_non_interpolated_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(
            &temp_dir,
            vec![
                ("config.txt", "User: @username@", true),
                ("binary.dat", "Raw data: @username@", false),
                ("template.conf", "Editor: @editor@\nUser: @username@", true),
            ],
        );

        let phase = InterpolatePhase;
        phase.execute(&mut context).unwrap();

        assert!(context.store_hash.is_some());

        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.get_store_path(context.store_hash.as_ref().unwrap());

        let config_content = fs::read_to_string(store_path.join("config.txt")).unwrap();
        assert_eq!(config_content, "User: testuser");

        let template_content = fs::read_to_string(store_path.join("template.conf")).unwrap();
        assert_eq!(template_content, "Editor: vim\nUser: testuser");

        let binary_content = fs::read_to_string(store_path.join("binary.dat")).unwrap();
        assert_eq!(binary_content, "Raw data: @username@");
    }

    #[test]
    #[serial]
    fn test_creates_nested_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(
            &temp_dir,
            vec![
                ("deep/nested/config.txt", "User: @username@", true),
                ("another/path/file.dat", "Raw: @editor@", false),
            ],
        );

        let phase = InterpolatePhase;
        phase.execute(&mut context).unwrap();

        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.get_store_path(context.store_hash.as_ref().unwrap());

        let nested_file = store_path.join("deep/nested/config.txt");
        assert!(nested_file.exists());
        let content = fs::read_to_string(&nested_file).unwrap();
        assert_eq!(content, "User: testuser");

        let another_file = store_path.join("another/path/file.dat");
        assert!(another_file.exists());
        let content = fs::read_to_string(&another_file).unwrap();
        assert_eq!(content, "Raw: @editor@");
    }

    #[test]
    #[serial]
    fn test_content_hash_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let mut context1 =
            create_test_context(&temp_dir, vec![("config.txt", "User: @username@", true)]);

        let phase = InterpolatePhase;
        phase.execute(&mut context1).unwrap();
        let hash1 = context1.store_hash.clone();

        let mut context2 =
            create_test_context(&temp_dir, vec![("config.txt", "User: @username@", true)]);
        phase.execute(&mut context2).unwrap();
        let hash2 = context2.store_hash.clone();

        assert_eq!(hash1, hash2);
    }

    #[test]
    #[serial]
    fn test_different_content_different_hash() {
        let temp_dir = TempDir::new().unwrap();
        let mut context1 =
            create_test_context(&temp_dir, vec![("config.txt", "User: @username@", true)]);

        let phase = InterpolatePhase;
        phase.execute(&mut context1).unwrap();
        let hash1 = context1.store_hash.clone();

        let mut context2 =
            create_test_context(&temp_dir, vec![("config.txt", "Editor: @editor@", true)]);
        phase.execute(&mut context2).unwrap();
        let hash2 = context2.store_hash.clone();

        assert_ne!(hash1, hash2);
    }
}
