use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const STOWAWAY_DIR: &str = ".stowaway";
// TODO: Allow both yaml/yml
pub const STOWAWAY_CONFIG: &str = "stowaway.yaml";
pub const STOWAWAY_STORE_PATH: &str = ".stowaway/store/";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StowawayConfig {
    pub variables: HashMap<String, String>,
    pub interpolation: InterpolationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpolationConfig {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
}

impl Default for InterpolationConfig {
    fn default() -> Self {
        Self {
            include_patterns: vec!["**/*".to_string()],
            exclude_patterns: vec![format!("**/{}", STOWAWAY_CONFIG)],
        }
    }
}

pub trait ConfigLoader {
    fn load_config(&self, source_dir: &Path) -> Result<StowawayConfig>;
}

pub struct YamlConfigLoader;

impl ConfigLoader for YamlConfigLoader {
    fn load_config(&self, source_dir: &Path) -> Result<StowawayConfig> {
        let config_path = source_dir.join(STOWAWAY_CONFIG);

        if !config_path.exists() {
            return Ok(StowawayConfig::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: StowawayConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = StowawayConfig::default();
        assert!(config.variables.is_empty());
        assert_eq!(config.interpolation.include_patterns, vec!["**/*"]);
        assert_eq!(
            config.interpolation.exclude_patterns,
            vec![format!("**/{}", STOWAWAY_CONFIG)]
        );
    }

    #[test]
    fn test_load_missing_config() {
        let loader = YamlConfigLoader;
        let temp_dir = TempDir::new().unwrap();

        let config = loader.load_config(temp_dir.path()).unwrap();
        assert!(config.variables.is_empty());
    }

    #[test]
    fn test_load_valid_config() {
        let loader = YamlConfigLoader;
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(STOWAWAY_CONFIG);

        let yaml_content = r#"
variables:
  username: "testuser"
  editor: "vim"
interpolation:
  include_patterns:
    - "**/*.conf"
    - "**/*.toml"
  exclude_patterns:
    - "**/*.bin"
"#;

        std::fs::write(&config_path, yaml_content).unwrap();

        let config = loader.load_config(temp_dir.path()).unwrap();
        assert_eq!(
            config.variables.get("username"),
            Some(&"testuser".to_string())
        );
        assert_eq!(config.variables.get("editor"), Some(&"vim".to_string()));
        assert_eq!(
            config.interpolation.include_patterns,
            vec!["**/*.conf", "**/*.toml"]
        );
        assert_eq!(config.interpolation.exclude_patterns, vec!["**/*.bin"]);
    }
}
