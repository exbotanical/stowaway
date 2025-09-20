use thiserror::Error;

pub type Result<T> = std::result::Result<T, StowawayError>;

#[derive(Error, Debug)]
pub enum StowawayError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Interpolation error: {0}")]
    Interpolation(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Linking error: {0}")]
    Linking(String),

    #[error("Conflict detected: {0}")]
    Conflict(String),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),

    #[error("Walk directory error: {0}")]
    WalkDir(#[from] walkdir::Error),
}
