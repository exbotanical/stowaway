use crate::{config::StowawayConfig, file_entry::FileEntry, store::StoreVersion};

#[derive(Debug, Clone, PartialEq)]
pub enum OperationMode {
    /// Same content as current version, verify existing symlinks
    Verify,
    /// New installation, create everything from scratch
    Create,
    /// Content changed, replace existing symlinks with new ones
    Replace,
    /// Switch to existing store version (rollback-like operation)
    Switch(StoreVersion),
}

impl Default for OperationMode {
    fn default() -> Self {
        OperationMode::Create
    }
}

pub struct StowawayContext {
    /// The user-provided configuration
    pub config: StowawayConfig,
    /// The directory which houses the source-tree i.e. files we will scan and transform, and ultimately place
    pub source_dir: std::path::PathBuf,
    pub target_dir: std::path::PathBuf,
    pub dry_run: bool,
    pub files: Vec<FileEntry>,
    pub store_hash: Option<String>,
    pub operation_mode: OperationMode,
    pub current_version: Option<StoreVersion>,
    pub target_hash: Option<String>,
}
