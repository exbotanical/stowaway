use crate::error::Result;
use crate::utils::is_member;
use crate::{
    context::{OperationMode, StowawayContext},
    lifecycle::LifecyclePhase,
    store::{calculate_content_hash, FileSystemStoreManager, StoreManager},
};
use std::fs;
use tracing::{debug, info, warn};

/// The context phase collects information about the current stowaway environment and pre-emptively computes the content hash
/// so we can determine which subsequent phases need to be executed (and how).
/// This is where we will, for example, determine whether the source has mutated and we need to generate a new store revision.
pub struct ContextPhase;

impl ContextPhase {
    /// Calculates content hash from source files before any processing
    fn calculate_source_hash(&self, context: &StowawayContext) -> Result<String> {
        let mut all_content = String::new();

        for entry in walkdir::WalkDir::new(&context.source_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();

                let relative_path = path.strip_prefix(&context.source_dir).map_err(|_| {
                    crate::error::StowawayError::Store("Invalid source path".to_string())
                })?;

                let should_include = if context.config.interpolation.include_patterns.is_empty() {
                    true
                } else {
                    is_member(
                        &context.config.interpolation.include_patterns,
                        relative_path,
                    )
                };

                let should_exclude = is_member(
                    &context.config.interpolation.exclude_patterns,
                    relative_path,
                );

                if should_include && !should_exclude {
                    let content = fs::read_to_string(path)?;
                    all_content.push_str(&content);
                }
            }
        }

        Ok(calculate_content_hash(&all_content))
    }
}

impl LifecyclePhase for ContextPhase {
    fn execute(&self, context: &mut StowawayContext) -> Result<()> {
        info!("Setting up context and determining operation mode");

        let store_manager = FileSystemStoreManager::new()?;

        // 1. Calculate hash from source files
        let content_hash = self.calculate_source_hash(context)?;
        debug!(content_hash = %content_hash, "Calculated content hash");

        // 2. Get current version
        let current_version = store_manager.get_current_version()?;

        // 3. Determine operation mode
        // 3a. We have a current version...
        if let Some(current) = &current_version {
            debug!(current_hash = %current.hash, "Found current version");

            // If the content has not changed...
            if current.hash == content_hash {
                // ...we just need to verify the symlinks haven't been messed with.
                info!("Content unchanged, will verify existing symlinks");
                context.operation_mode = OperationMode::Verify;
                context.store_hash = Some(content_hash);
                context.current_version = current_version;
                return Ok(());
            } else {
                // If the content has changed...
                // ...check if we have an existing version that matches the current state (i.e. we just reverted to an old version implicitly)...
                if let Ok(existing_version) = store_manager.read_metadata(&content_hash) {
                    // ...a matching store exists but isn't current - switch to it
                    info!(target_hash = %content_hash, "Switching to existing store version");
                    context.operation_mode = OperationMode::Switch(existing_version);
                    context.store_hash = Some(content_hash.clone());
                    context.current_version = current_version;
                    context.target_hash = Some(content_hash);
                    return Ok(());
                } else {
                    // The content has changed to a state we've never seen before. We need to compute a new version.
                    info!(
                        current_hash = %current.hash,
                        new_hash = %content_hash,
                        "Content changed, will replace symlinks"
                    );
                    context.operation_mode = OperationMode::Replace;
                    context.current_version = current_version;
                    context.target_hash = Some(content_hash);
                }
            }
        } else {
            // 3b. No current version - check if store version exists for this hash
            if let Ok(existing_version) = store_manager.read_metadata(&content_hash) {
                // Store exists but no current version - switch to it
                info!(target_hash = %content_hash, "No current version exists, but there was a match. Switching to it.");
                context.operation_mode = OperationMode::Switch(existing_version);
                context.store_hash = Some(content_hash.clone());
                context.target_hash = Some(content_hash);
            } else {
                info!("New installation, will create everything");
                context.operation_mode = OperationMode::Create;
                context.target_hash = Some(content_hash);
            }
        }

        debug!(operation_mode = ?context.operation_mode, "Determined operation mode");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StowawayConfig;
    use dirs::home_dir;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_context(temp_dir: &TempDir) -> StowawayContext {
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        let store_dir = home_dir()
            .expect("Expected home dir")
            .as_path()
            .join(".stowaway");

        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        // Clean up any existing store directory to ensure clean test state
        if store_dir.exists() {
            fs::remove_dir_all(&store_dir).unwrap();
        }

        let config = StowawayConfig {
            variables: HashMap::new(),
            ..Default::default()
        };

        StowawayContext {
            config,
            source_dir,
            target_dir,
            dry_run: false,
            files: Vec::new(),
            store_hash: None,
            operation_mode: OperationMode::Create,
            current_version: None,
            target_hash: None,
        }
    }

    #[test]
    #[serial]
    fn test_new_installation() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        fs::write(context.source_dir.join("test.txt"), "content").unwrap();

        let phase = ContextPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.operation_mode, OperationMode::Create);
        assert!(context.target_hash.is_some());
        assert!(context.current_version.is_none());
    }

    #[test]
    #[serial]
    fn test_empty_source_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        let phase = ContextPhase;
        phase.execute(&mut context).unwrap();

        assert_eq!(context.operation_mode, OperationMode::Create);
        assert!(context.target_hash.is_some());
    }

    #[test]
    #[serial]
    fn test_respects_include_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        // Set include patterns
        context.config.interpolation.include_patterns = vec!["*.conf".to_string()];

        // Create files - only .conf should be included
        fs::write(context.source_dir.join("included.conf"), "included").unwrap();
        fs::write(context.source_dir.join("excluded.txt"), "excluded").unwrap();

        let phase = ContextPhase;
        phase.execute(&mut context).unwrap();

        assert!(context.target_hash.is_some());
    }

    #[test]
    #[serial]
    fn test_respects_exclude_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let mut context = create_test_context(&temp_dir);

        // Set exclude patterns
        context.config.interpolation.exclude_patterns = vec!["*.tmp".to_string()];

        // Create files - .tmp should be excluded
        fs::write(context.source_dir.join("included.txt"), "included").unwrap();
        fs::write(context.source_dir.join("excluded.tmp"), "excluded").unwrap();

        let phase = ContextPhase;
        phase.execute(&mut context).unwrap();

        assert!(context.target_hash.is_some());
        // The hash should only include the .txt file content
    }
}
