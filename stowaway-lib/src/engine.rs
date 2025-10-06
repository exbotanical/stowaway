use crate::config::{ConfigLoader, YamlConfigLoader};
use crate::context::StowawayContext;
use crate::error::{Result, StowawayError};
use crate::lifecycle::cleanup::CleanupPhase;
use crate::lifecycle::context::ContextPhase;
use crate::lifecycle::interpolate::InterpolatePhase;
use crate::lifecycle::link::LinkPhase;
use crate::lifecycle::scan::ScanPhase;
use crate::lifecycle::validate::ValidatePhase;
use crate::lifecycle::LifecyclePhase;
use crate::linking::{FileSystemLinker, Linker};
use crate::log_dryrun;
use crate::store::{FileSystemStoreManager, StoreManager};
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, instrument, warn};
use walkdir::WalkDir;

pub struct ExecutionFlags {
    pub is_dry_run: bool,
}

pub struct StowawayEngine {
    config_loader: Box<dyn ConfigLoader>,
    phases: Vec<Box<dyn LifecyclePhase>>,
}

impl StowawayEngine {
    pub fn new() -> Self {
        Self {
            config_loader: Box::new(YamlConfigLoader),
            phases: vec![
                Box::new(ContextPhase),
                Box::new(ScanPhase),
                Box::new(ValidatePhase),
                Box::new(InterpolatePhase),
                Box::new(LinkPhase),
                Box::new(CleanupPhase),
            ],
        }
    }

    #[instrument(skip(self), fields(source = %source.display(), target = %target.display(), flags))]
    pub fn execute(&self, source: &Path, target: &Path, flags: ExecutionFlags) -> Result<()> {
        let start_time = Instant::now();

        info!(
            source_dir = %source.display(),
            target_dir = %target.display(),
            flags.is_dry_run,
            "Starting stow operation"
        );

        let config = self.config_loader.load_config(source)?;
        debug!(
            variables_count = config.variables.len(),
            include_patterns = config.interpolation.include_patterns.len(),
            exclude_patterns = config.interpolation.exclude_patterns.len(),
            "Loaded configuration"
        );

        let mut context =
            StowawayContext::default(config, source.to_path_buf(), target.to_path_buf(), flags);

        for (phase_index, phase) in self.phases.iter().enumerate() {
            let phase_start = Instant::now();
            debug!(phase_index, "Starting lifecycle phase");

            phase.execute(&mut context)?;

            debug!(
                phase_index,
                duration_ms = phase_start.elapsed().as_millis(),
                "Completed lifecycle phase"
            );
        }

        let duration = start_time.elapsed();
        debug!(
            files_processed = context.files.len(),
            duration_ms = duration.as_millis(),
            "Operation completed successfully"
        );

        Ok(())
    }

    #[instrument(skip(self), fields(dry_run))]
    pub fn unstow(&self, dry_run: bool) -> Result<()> {
        let start_time = Instant::now();

        info!(dry_run, "Starting unstow operation");

        let store_manager = FileSystemStoreManager::new()?;
        let linker = FileSystemLinker;

        let current_version = store_manager.get_current_version()?;
        if current_version.is_none() {
            error!("No current version exists - nothing to unstow");
            return Err(StowawayError::Store(
                "No current version exists - nothing to unstow".to_string(),
            ));
        }

        let current_version = current_version.unwrap();
        info!(
            current_version = current_version.hash,
            target_dir = %current_version.target_dir.display(),
            "Found current version to unstow"
        );

        let current_store_path = store_manager.get_store_path(&current_version.hash);
        if !current_store_path.exists() {
            error!(
                version = current_version.hash,
                "Store version does not exist"
            );
            return Err(StowawayError::Store(format!(
                "Store version {} does not exist",
                current_version.hash
            )));
        }

        if dry_run {
            log_dryrun!("Would remove symlinks for current version");
            let mut symlink_count = 0;
            for entry in WalkDir::new(&current_store_path) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let relative_path = entry
                        .path()
                        .strip_prefix(&current_store_path)
                        .map_err(|_| StowawayError::Store("Invalid store path".to_string()))?;
                    let target_path = current_version.target_dir.join(relative_path);

                    if target_path.is_symlink() {
                        debug!(target = %target_path.display(), "Would remove symlink");
                        symlink_count += 1;
                    }
                }
            }
            log_dryrun!("Would remove {} symlinks", symlink_count);
        } else {
            self.remove_symlinks_for_store(
                &current_store_path,
                &current_version.target_dir,
                &linker,
            )?;

            let session_file = store_manager.session_file_path();
            if session_file.exists() {
                std::fs::remove_file(&session_file)?;
                debug!("Cleared current version session");
            }

            info!("Removed all symlinks and cleared current version");
        }

        let duration = start_time.elapsed();
        info!(
            duration_ms = duration.as_millis(),
            "Unstow operation completed successfully"
        );

        Ok(())
    }

    fn remove_symlinks_for_store(
        &self,
        store_path: &Path,
        target_dir: &Path,
        linker: &FileSystemLinker,
    ) -> Result<()> {
        for entry in WalkDir::new(store_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let relative_path = entry
                    .path()
                    .strip_prefix(store_path)
                    .map_err(|_| StowawayError::Store("Invalid store path".to_string()))?;
                let target_path = target_dir.join(relative_path);

                if target_path.is_symlink() {
                    linker.remove_symlink(&target_path)?;
                }
            }
        }
        Ok(())
    }
}

impl Default for StowawayEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::STOWAWAY_CONFIG;

    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config(source_dir: &Path) {
        let config_content = r#"
variables:
  user: testuser
  theme: dark

interpolation:
  include_patterns:
    - "*.conf"
    - "*.toml"
  exclude_patterns:
    - "*.tmp"
"#;
        fs::write(source_dir.join(STOWAWAY_CONFIG), config_content).unwrap();
    }

    fn create_test_files(source_dir: &Path) {
        fs::create_dir_all(source_dir.join("config")).unwrap();
        fs::write(
            source_dir.join("config/app.conf"),
            "user=@user@\ntheme=@theme@",
        )
        .unwrap();
        fs::write(
            source_dir.join("config/settings.toml"),
            "[user]\nname = \"@user@\"",
        )
        .unwrap();
        fs::write(source_dir.join("readme.txt"), "This is a readme").unwrap();
        fs::write(source_dir.join("temp.tmp"), "temporary file").unwrap();
    }

    #[test]
    #[serial]
    fn test_execute_runs_all_phases_successfully() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        create_test_config(&source_dir);
        create_test_files(&source_dir);

        let engine = StowawayEngine::new();
        let result = engine.execute(
            &source_dir,
            &target_dir,
            ExecutionFlags { is_dry_run: false },
        );

        if let Err(e) = &result {
            eprintln!("Engine execution failed: {}", e);
        }
        assert!(result.is_ok());

        let target_config_file = target_dir.join("config/app.conf");
        assert!(target_config_file.is_symlink());

        let target_settings_file = target_dir.join("config/settings.toml");
        assert!(target_settings_file.is_symlink());

        let target_readme = target_dir.join("readme.txt");
        assert!(target_readme.is_symlink());

        let target_temp = target_dir.join("temp.tmp");
        assert!(!target_temp.exists());
    }

    #[test]
    #[serial]
    fn test_execute_dry_run_creates_no_symlinks() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        create_test_config(&source_dir);
        create_test_files(&source_dir);

        let engine = StowawayEngine::new();
        let result = engine.execute(
            &source_dir,
            &target_dir,
            ExecutionFlags { is_dry_run: true },
        );

        assert!(result.is_ok());

        let target_config_file = target_dir.join("config/app.conf");
        assert!(!target_config_file.exists());

        let target_settings_file = target_dir.join("config/settings.toml");
        assert!(!target_settings_file.exists());
    }

    #[test]
    #[serial]
    fn test_execute_fails_on_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        create_test_config(&source_dir);
        create_test_files(&source_dir);

        fs::create_dir_all(target_dir.join("config")).unwrap();
        fs::write(target_dir.join("config/app.conf"), "existing content").unwrap();

        let engine = StowawayEngine::new();
        let result = engine.execute(
            &source_dir,
            &target_dir,
            ExecutionFlags { is_dry_run: false },
        );

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("conflicts"));
    }

    #[test]
    #[serial]
    fn test_execute_succeeds_with_default_config() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        create_test_files(&source_dir);

        let engine = StowawayEngine::new();
        let result = engine.execute(
            &source_dir,
            &target_dir,
            ExecutionFlags { is_dry_run: false },
        );

        assert!(result.is_ok());

        let target_config_file = target_dir.join("config/app.conf");
        assert!(target_config_file.is_symlink());

        let target_settings_file = target_dir.join("config/settings.toml");
        assert!(target_settings_file.is_symlink());

        let target_readme = target_dir.join("readme.txt");
        assert!(target_readme.is_symlink());

        let target_temp = target_dir.join("temp.tmp");
        assert!(target_temp.is_symlink());
    }

    #[test]
    #[serial]
    fn test_execute_interpolates_variables() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        create_test_config(&source_dir);
        fs::write(source_dir.join("test.conf"), "user=@user@\ntheme=@theme@").unwrap();

        let engine = StowawayEngine::new();
        let result = engine.execute(
            &source_dir,
            &target_dir,
            ExecutionFlags { is_dry_run: false },
        );

        assert!(result.is_ok());

        let target_file = target_dir.join("test.conf");
        assert!(target_file.is_symlink());

        let symlink_target = fs::read_link(&target_file).unwrap();
        let interpolated_content = fs::read_to_string(&symlink_target).unwrap();
        assert!(interpolated_content.contains("user=testuser"));
        assert!(interpolated_content.contains("theme=dark"));
    }

    #[test]
    #[serial]
    fn test_execute_handles_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        create_test_config(&source_dir);
        fs::create_dir_all(source_dir.join("deep/nested/path")).unwrap();
        fs::write(
            source_dir.join("deep/nested/path/config.conf"),
            "nested=@user@",
        )
        .unwrap();

        let engine = StowawayEngine::new();
        let result = engine.execute(
            &source_dir,
            &target_dir,
            ExecutionFlags { is_dry_run: false },
        );

        assert!(result.is_ok());

        let target_nested_file = target_dir.join("deep/nested/path/config.conf");
        assert!(target_nested_file.is_symlink());
    }

    #[test]
    #[serial]
    fn test_execute_preserves_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        create_test_config(&source_dir);
        fs::create_dir_all(source_dir.join("config/app")).unwrap();
        fs::create_dir_all(source_dir.join("scripts/utils")).unwrap();
        fs::write(source_dir.join("config/app/main.conf"), "app config").unwrap();
        fs::write(source_dir.join("scripts/utils/helper.sh"), "#!/bin/bash").unwrap();

        let engine = StowawayEngine::new();
        let result = engine.execute(
            &source_dir,
            &target_dir,
            ExecutionFlags { is_dry_run: false },
        );

        assert!(result.is_ok());

        assert!(target_dir.join("config/app").is_dir());
        assert!(target_dir.join("scripts/utils").is_dir());
        assert!(target_dir.join("config/app/main.conf").is_symlink());
        assert!(target_dir.join("scripts/utils/helper.sh").is_symlink());
    }

    #[test]
    #[serial]
    fn test_execute_respects_include_exclude_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&target_dir).unwrap();

        create_test_config(&source_dir);
        fs::write(source_dir.join("included.conf"), "user=@user@").unwrap();
        fs::write(source_dir.join("excluded.tmp"), "temp=@user@").unwrap();
        fs::write(source_dir.join("plain.txt"), "plain text").unwrap();

        let engine = StowawayEngine::new();
        let result = engine.execute(
            &source_dir,
            &target_dir,
            ExecutionFlags { is_dry_run: false },
        );

        assert!(result.is_ok());

        let included_target = target_dir.join("included.conf");
        assert!(included_target.is_symlink());
        let symlink_target = fs::read_link(&included_target).unwrap();
        let content = fs::read_to_string(&symlink_target).unwrap();
        assert!(content.contains("user=testuser"));

        let excluded_target = target_dir.join("excluded.tmp");
        assert!(!excluded_target.exists());

        let plain_target = target_dir.join("plain.txt");
        assert!(plain_target.is_symlink());
        let plain_symlink_target = fs::read_link(&plain_target).unwrap();
        let plain_content = fs::read_to_string(&plain_symlink_target).unwrap();
        assert_eq!(plain_content, "plain text");
    }
}
