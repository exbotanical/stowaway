use crate::error::{Result, StowawayError};
use std::path::Path;

pub trait Linker {
    fn create_symlink(&self, source: &Path, target: &Path) -> Result<()>;
    fn remove_symlink(&self, target: &Path) -> Result<()>;
    fn is_symlink_valid(&self, target: &Path, expected_source: &Path) -> Result<bool>;
}

pub struct FileSystemLinker;

impl Linker for FileSystemLinker {
    fn create_symlink(&self, source: &Path, target: &Path) -> Result<()> {
        if let Some(parent) = target.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        if target.exists() || target.is_symlink() {
            return Err(StowawayError::Linking(format!(
                "Target already exists: {}",
                target.display()
            )));
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(source, target)?;
        }

        #[cfg(windows)]
        {
            if source.is_dir() {
                std::os::windows::fs::symlink_dir(source, target)?;
            } else {
                std::os::windows::fs::symlink_file(source, target)?;
            }
        }

        Ok(())
    }

    fn remove_symlink(&self, target: &Path) -> Result<()> {
        if !target.is_symlink() {
            return Err(StowawayError::Linking(format!(
                "Target is not a symlink: {}",
                target.display()
            )));
        }

        std::fs::remove_file(target)?;
        Ok(())
    }

    fn is_symlink_valid(&self, target: &Path, expected_source: &Path) -> Result<bool> {
        if !target.is_symlink() {
            return Ok(false);
        }

        match std::fs::read_link(target) {
            Ok(actual_source) => Ok(actual_source == expected_source),
            Err(_) => Ok(false),
        }
    }
}

impl Default for FileSystemLinker {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_symlink() {
        let linker = FileSystemLinker;
        let temp_dir = TempDir::new().unwrap();

        let source_file = temp_dir.path().join("source.txt");
        let target_file = temp_dir.path().join("target.txt");

        fs::write(&source_file, "content").unwrap();

        linker.create_symlink(&source_file, &target_file).unwrap();

        assert!(target_file.is_symlink());
        assert_eq!(fs::read_to_string(&target_file).unwrap(), "content");
    }

    #[test]
    fn test_create_symlink_with_directories() {
        let linker = FileSystemLinker;
        let temp_dir = TempDir::new().unwrap();

        let source_file = temp_dir.path().join("source.txt");
        let target_file = temp_dir.path().join("subdir").join("target.txt");

        fs::write(&source_file, "content").unwrap();

        linker.create_symlink(&source_file, &target_file).unwrap();

        assert!(target_file.is_symlink());
        assert_eq!(fs::read_to_string(&target_file).unwrap(), "content");
    }

    #[test]
    fn test_remove_symlink() {
        let linker = FileSystemLinker;
        let temp_dir = TempDir::new().unwrap();

        let source_file = temp_dir.path().join("source.txt");
        let target_file = temp_dir.path().join("target.txt");

        fs::write(&source_file, "content").unwrap();
        linker.create_symlink(&source_file, &target_file).unwrap();

        assert!(target_file.is_symlink());

        linker.remove_symlink(&target_file).unwrap();

        assert!(!target_file.exists());
    }

    #[test]
    fn test_is_symlink_valid() {
        let linker = FileSystemLinker;
        let temp_dir = TempDir::new().unwrap();

        let source_file = temp_dir.path().join("source.txt");
        let target_file = temp_dir.path().join("target.txt");

        fs::write(&source_file, "content").unwrap();
        linker.create_symlink(&source_file, &target_file).unwrap();

        assert!(linker.is_symlink_valid(&target_file, &source_file).unwrap());

        let wrong_source = temp_dir.path().join("wrong.txt");
        assert!(!linker
            .is_symlink_valid(&target_file, &wrong_source)
            .unwrap());
    }

    #[test]
    fn test_create_symlink_target_exists() {
        let linker = FileSystemLinker;
        let temp_dir = TempDir::new().unwrap();

        let source_file = temp_dir.path().join("source.txt");
        let target_file = temp_dir.path().join("target.txt");

        fs::write(&source_file, "content").unwrap();
        fs::write(&target_file, "existing").unwrap();

        let result = linker.create_symlink(&source_file, &target_file);
        assert!(result.is_err());
    }
}
