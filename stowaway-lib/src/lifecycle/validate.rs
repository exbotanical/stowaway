use crate::validation::Validator;
use crate::{context::StowawayContext, lifecycle::LifecyclePhase, validation::FileSystemValidator, StowawayError};
use crate::error::{Result};

pub struct ValidatePhase;
impl LifecyclePhase for ValidatePhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()> {
        println!("Validating conflicts for {} files", context.files.len());

        let validator = FileSystemValidator;
        let mut conflicts = Vec::new();

        for file_entry in &context.files {
            let validation_result = validator.validate_path(&file_entry.target_path)?;

            match validation_result.conflict_type {
                crate::validation::ConflictType::None => continue,
                crate::validation::ConflictType::SymlinkExists if validation_result.is_managed => continue,
                _ => {
                    conflicts.push((file_entry.target_path.clone(), validation_result.conflict_type));
                }
            }
        }

        if !conflicts.is_empty() {
            let conflict_msg = conflicts.iter()
                .map(|(path, conflict_type)| format!("{}: {:?}", path.display(), conflict_type))
                .collect::<Vec<_>>()
                .join("\n");

            return Err(StowawayError::Conflict(format!(
                "Found {} conflicts:\n{}", conflicts.len(), conflict_msg
            )));
        }

        println!("No conflicts found");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{StowawayConfig, InterpolationConfig};
    use crate::context::StowawayContext;
    use crate::file_entry::FileEntry;
    use std::collections::HashMap;
    use std::fs;
    use std::os::unix::fs as unix_fs;
    use tempfile::TempDir;

    fn create_test_context(temp_dir: &TempDir) -> StowawayContext {
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        let config = StowawayConfig {
            variables: HashMap::new(),
            interpolation: InterpolationConfig {
                include_patterns: vec![],
                exclude_patterns: vec![],
            },
        };

        StowawayContext {
            config,
            source_dir,
            target_dir,
            dry_run: false,
            files: Vec::new(),
            store_hash: None,
        }
    }

    #[test]
    fn test_passes_with_no_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let file_entry = FileEntry {
            source_path: context.source_dir.join("config.txt"),
            relative_path: "config.txt".into(),
            target_path: context.target_dir.join("config.txt"),
            should_interpolate: true,
        };
        context.files.push(file_entry);

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_ok());
    }

    #[test]
    fn test_detects_file_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let target_file = context.target_dir.join("config.txt");
        fs::write(&target_file, "existing content").unwrap();

        let file_entry = FileEntry {
            source_path: context.source_dir.join("config.txt"),
            relative_path: "config.txt".into(),
            target_path: target_file,
            should_interpolate: true,
        };
        context.files.push(file_entry);

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Found 1 conflicts"));
        assert!(error_msg.contains("FileExists"));
    }

    #[test]
    fn test_detects_directory_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let target_dir = context.target_dir.join("config");
        fs::create_dir_all(&target_dir).unwrap();

        let file_entry = FileEntry {
            source_path: context.source_dir.join("config"),
            relative_path: "config".into(),
            target_path: target_dir,
            should_interpolate: false,
        };
        context.files.push(file_entry);

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Found 1 conflicts"));
        assert!(error_msg.contains("DirectoryExists"));
    }

    #[test]
    fn test_allows_managed_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let store_dir = temp_dir.path().join(".stowaway/store/abc123");
        fs::create_dir_all(&store_dir).unwrap();
        let store_file = store_dir.join("config.txt");
        fs::write(&store_file, "store content").unwrap();

        let target_file = context.target_dir.join("config.txt");
        unix_fs::symlink(&store_file, &target_file).unwrap();

        let file_entry = FileEntry {
            source_path: context.source_dir.join("config.txt"),
            relative_path: "config.txt".into(),
            target_path: target_file,
            should_interpolate: true,
        };
        context.files.push(file_entry);

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_ok());
    }

    #[test]
    fn test_detects_unmanaged_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let external_file = temp_dir.path().join("external.txt");
        fs::write(&external_file, "external content").unwrap();

        let target_file = context.target_dir.join("config.txt");
        unix_fs::symlink(&external_file, &target_file).unwrap();

        let file_entry = FileEntry {
            source_path: context.source_dir.join("config.txt"),
            relative_path: "config.txt".into(),
            target_path: target_file,
            should_interpolate: true,
        };
        context.files.push(file_entry);

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Found 1 conflicts"));
        assert!(error_msg.contains("SymlinkExists"));
    }

    #[test]
    fn test_handles_multiple_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let file1 = context.target_dir.join("config.txt");
        fs::write(&file1, "existing file").unwrap();

        let dir1 = context.target_dir.join("scripts");
        fs::create_dir_all(&dir1).unwrap();

        let external_file = temp_dir.path().join("external.txt");
        fs::write(&external_file, "external content").unwrap();
        let symlink1 = context.target_dir.join("link.txt");
        unix_fs::symlink(&external_file, &symlink1).unwrap();

        context.files.push(FileEntry {
            source_path: context.source_dir.join("config.txt"),
            relative_path: "config.txt".into(),
            target_path: file1,
            should_interpolate: true,
        });

        context.files.push(FileEntry {
            source_path: context.source_dir.join("scripts"),
            relative_path: "scripts".into(),
            target_path: dir1,
            should_interpolate: false,
        });

        context.files.push(FileEntry {
            source_path: context.source_dir.join("link.txt"),
            relative_path: "link.txt".into(),
            target_path: symlink1,
            should_interpolate: true,
        });

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Found 3 conflicts"));
        assert!(error_msg.contains("FileExists"));
        assert!(error_msg.contains("DirectoryExists"));
        assert!(error_msg.contains("SymlinkExists"));
    }

    #[test]
    fn test_mixed_valid_and_invalid_files() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let store_dir = temp_dir.path().join(".stowaway/store/abc123");
        fs::create_dir_all(&store_dir).unwrap();
        let store_file = store_dir.join("managed.txt");
        fs::write(&store_file, "store content").unwrap();

        let managed_symlink = context.target_dir.join("managed.txt");
        unix_fs::symlink(&store_file, &managed_symlink).unwrap();

        let conflicting_file = context.target_dir.join("conflict.txt");
        fs::write(&conflicting_file, "existing content").unwrap();

        context.files.push(FileEntry {
            source_path: context.source_dir.join("valid.txt"),
            relative_path: "valid.txt".into(),
            target_path: context.target_dir.join("valid.txt"),
            should_interpolate: true,
        });

        context.files.push(FileEntry {
            source_path: context.source_dir.join("managed.txt"),
            relative_path: "managed.txt".into(),
            target_path: managed_symlink,
            should_interpolate: true,
        });

        context.files.push(FileEntry {
            source_path: context.source_dir.join("conflict.txt"),
            relative_path: "conflict.txt".into(),
            target_path: conflicting_file,
            should_interpolate: true,
        });

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Found 1 conflicts"));
        assert!(error_msg.contains("FileExists"));
        assert!(!error_msg.contains("managed.txt"));
    }

    #[test]
    fn test_empty_file_list() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let nested_file = context.target_dir.join("config/app/settings.toml");
        fs::create_dir_all(nested_file.parent().unwrap()).unwrap();

        let file_entry = FileEntry {
            source_path: context.source_dir.join("config/app/settings.toml"),
            relative_path: "config/app/settings.toml".into(),
            target_path: nested_file,
            should_interpolate: true,
        };
        context.files.push(file_entry);

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_ok());
    }

    #[test]
    fn test_broken_symlink_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let nonexistent_target = temp_dir.path().join("nonexistent.txt");
        let broken_symlink = context.target_dir.join("broken.txt");
        unix_fs::symlink(&nonexistent_target, &broken_symlink).unwrap();

        let file_entry = FileEntry {
            source_path: context.source_dir.join("broken.txt"),
            relative_path: "broken.txt".into(),
            target_path: broken_symlink,
            should_interpolate: true,
        };
        context.files.push(file_entry);

        let phase = ValidatePhase;
        let result = phase.execute(&mut context);

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Found 1 conflicts"));
        assert!(error_msg.contains("SymlinkExists"));
    }
}
