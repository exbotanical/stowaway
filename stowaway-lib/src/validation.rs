use crate::config::STOWAWAY_STORE_PATH;
use crate::error::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub path: std::path::PathBuf,
    pub conflict_type: ConflictType,
    pub is_managed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    None,
    FileExists,
    DirectoryExists,
    SymlinkExists,
    PermissionDenied,
}

pub trait Validator {
    fn validate_path(&self, path: &Path) -> Result<ValidationResult>;
    fn is_managed_by_stowaway(&self, path: &Path) -> Result<bool>;
}

pub struct FileSystemValidator;

impl Validator for FileSystemValidator {
    fn validate_path(&self, path: &Path) -> Result<ValidationResult> {
        let conflict_type = if path.is_symlink() {
            ConflictType::SymlinkExists
        } else if !path.exists() {
            ConflictType::None
        } else if path.is_dir() {
            ConflictType::DirectoryExists
        } else if path.is_file() {
            ConflictType::FileExists
        } else {
            ConflictType::None
        };

        let is_managed = if path.is_symlink() {
            self.is_managed_by_stowaway(path)?
        } else {
            false
        };

        Ok(ValidationResult {
            path: path.to_path_buf(),
            conflict_type,
            is_managed,
        })
    }

    fn is_managed_by_stowaway(&self, path: &Path) -> Result<bool> {
        if !path.is_symlink() {
            return Ok(false);
        }

        match std::fs::read_link(path) {
            Ok(target) => {
                let target_str = target.to_string_lossy();
                Ok(target_str.contains(STOWAWAY_STORE_PATH))
            }
            Err(_) => Ok(false),
        }
    }
}

impl Default for FileSystemValidator {
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
    fn test_validate_nonexistent_path() {
        let validator = FileSystemValidator;
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("nonexistent");

        let result = validator.validate_path(&test_path).unwrap();

        assert_eq!(result.conflict_type, ConflictType::None);
        assert!(!result.is_managed);
        assert_eq!(result.path, test_path);
    }

    #[test]
    fn test_validate_existing_file() {
        let validator = FileSystemValidator;
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "content").unwrap();

        let result = validator.validate_path(&test_file).unwrap();

        assert_eq!(result.conflict_type, ConflictType::FileExists);
        assert!(!result.is_managed);
    }

    #[test]
    fn test_validate_existing_directory() {
        let validator = FileSystemValidator;
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("testdir");
        fs::create_dir(&test_dir).unwrap();

        let result = validator.validate_path(&test_dir).unwrap();

        assert_eq!(result.conflict_type, ConflictType::DirectoryExists);
        assert!(!result.is_managed);
    }

    #[test]
    fn test_is_managed_by_stowaway_false_for_file() {
        let validator = FileSystemValidator;
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "content").unwrap();

        let result = validator.is_managed_by_stowaway(&test_file).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_validation_result_creation() {
        let path = std::path::PathBuf::from("/test/path");
        let result = ValidationResult {
            path: path.clone(),
            conflict_type: ConflictType::FileExists,
            is_managed: true,
        };

        assert_eq!(result.path, path);
        assert_eq!(result.conflict_type, ConflictType::FileExists);
        assert!(result.is_managed);
    }
}
