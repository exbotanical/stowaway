use crate::{context::StowawayContext, interpolation::{Interpolator, VariableInterpolator}, lifecycle::LifecyclePhase, store::{calculate_content_hash, FileSystemStoreManager, StoreManager, StoreVersion}};
use crate::error::{Result};

pub struct InterpolatePhase;

impl LifecyclePhase for InterpolatePhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()> {
        println!("Interpolating variables");

        let interpolator = VariableInterpolator;
        let store_manager = FileSystemStoreManager::new()?;

        // Calculate content hash for this interpolation
        let mut all_content = String::new();
        for file_entry in &context.files {
            if file_entry.should_interpolate {
                let content = std::fs::read_to_string(&file_entry.source_path)?;
                let interpolated = interpolator.interpolate(&content, &context.config.variables)?;
                all_content.push_str(&interpolated);
            } else {
                let content = std::fs::read_to_string(&file_entry.source_path)?;
                all_content.push_str(&content);
            }
        }

        let content_hash = calculate_content_hash(&all_content);
        let store_path = store_manager.create_version(&content_hash)?;

        // Process each file
        for file_entry in &context.files {
            let store_file_path = store_path.join(&file_entry.relative_path);

            if let Some(parent) = store_file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            if file_entry.should_interpolate {
                let content = std::fs::read_to_string(&file_entry.source_path)?;
                let interpolated = interpolator.interpolate(&content, &context.config.variables)?;
                std::fs::write(&store_file_path, interpolated)?;
            } else {
                std::fs::copy(&file_entry.source_path, &store_file_path)?;
            }
        }

        // Update session with new version
        let version = StoreVersion {
            hash: content_hash.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_dir: context.source_dir.clone(),
        };

        store_manager.set_current_version(&version)?;
        context.store_hash = Some(content_hash);

        println!("Interpolated files to store");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StowawayConfig;
    use crate::context::{ StowawayContext};
    use crate::file_entry::FileEntry;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_context(temp_dir: &TempDir, files: Vec<(&str, &str, bool)>) -> StowawayContext {
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        let mut file_entries = Vec::new();
        for (relative_path, content, should_interpolate) in files {
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

        StowawayContext {
            config,
            source_dir,
            target_dir,
            dry_run: false,
            files: file_entries,
            store_hash: None,
        }
    }

    #[test]
    fn test_interpolates_all_vars_in_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec![
            ("config.txt", "User: @username@\nEditor: @editor@", true),
        ]);

        let phase = InterpolatePhase;
        phase.execute(&mut context).unwrap();

        // Verify store hash was set
        assert!(context.store_hash.is_some());

        // Verify interpolated content in store
        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.get_store_path(context.store_hash.as_ref().unwrap());
        let interpolated_file = store_path.join("config.txt");

        assert!(interpolated_file.exists());
        let content = fs::read_to_string(&interpolated_file).unwrap();
        assert_eq!(content, "User: testuser\nEditor: vim");
    }

    #[test]
    fn test_handles_missing_var_mappings() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec![
            ("config.txt", "User: @username@\nMissing: @nonexistent@", true),
        ]);

        let phase = InterpolatePhase;
        phase.execute(&mut context).unwrap();

        // Verify store hash was set
        assert!(context.store_hash.is_some());

        // Verify missing variables are left unchanged
        let store_manager = FileSystemStoreManager::new().unwrap();
        let store_path = store_manager.get_store_path(context.store_hash.as_ref().unwrap());
        let interpolated_file = store_path.join("config.txt");

        assert!(interpolated_file.exists());
        let content = fs::read_to_string(&interpolated_file).unwrap();
        assert_eq!(content, "User: testuser\nMissing: @nonexistent@");
    }

    #[test]
    fn test_copies_non_interpolated_files() {
        let temp_dir = TempDir::new().unwrap();
        let original_content = "This should not be interpolated: @username@";
        let mut context = create_test_context(&temp_dir, vec![
            ("binary.dat", original_content, false),
        ]);

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
    fn test_mixed_interpolated_and_non_interpolated_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec![
            ("config.txt", "User: @username@", true),
            ("binary.dat", "Raw data: @username@", false),
            ("template.conf", "Editor: @editor@\nUser: @username@", true),
        ]);

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
    fn test_creates_nested_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir, vec![
            ("deep/nested/config.txt", "User: @username@", true),
            ("another/path/file.dat", "Raw: @editor@", false),
        ]);

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
    fn test_content_hash_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let mut context1 = create_test_context(&temp_dir, vec![
            ("config.txt", "User: @username@", true),
        ]);

        let phase = InterpolatePhase;
        phase.execute(&mut context1).unwrap();
        let hash1 = context1.store_hash.clone();

        let mut context2 = create_test_context(&temp_dir, vec![
            ("config.txt", "User: @username@", true),
        ]);
        phase.execute(&mut context2).unwrap();
        let hash2 = context2.store_hash.clone();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_content_different_hash() {
        let temp_dir = TempDir::new().unwrap();
        let mut context1 = create_test_context(&temp_dir, vec![
            ("config.txt", "User: @username@", true),
        ]);

        let phase = InterpolatePhase;
        phase.execute(&mut context1).unwrap();
        let hash1 = context1.store_hash.clone();

        let mut context2 = create_test_context(&temp_dir, vec![
            ("config.txt", "Editor: @editor@", true),
        ]);
        phase.execute(&mut context2).unwrap();
        let hash2 = context2.store_hash.clone();

        assert_ne!(hash1, hash2);
    }
}
