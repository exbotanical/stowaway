use crate::{
    config::StowawayConfig, engine::ExecutionFlags, file_entry::FileEntry, store::StoreVersion,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum OperationMode {
    /// Same content as current version, verify existing symlinks
    Verify,
    /// New installation, create everything from scratch
    #[default]
    Create,
    /// Content changed, replace existing symlinks with new ones
    Replace,
}

pub struct StowawayContext {
    /// The user-provided configuration
    pub config: StowawayConfig,
    /// The directory which houses the source-tree i.e. files we will scan and transform, and ultimately place
    pub source_dir: std::path::PathBuf,
    /// The target of the eventually linked files i.e. where we are installing to
    pub target_dir: std::path::PathBuf,
    /// Whether stowaway should actually perform ANY mutative action
    pub dry_run: bool,
    /// The list of files eligible for processing
    pub files: Vec<FileEntry>,
    /// Version hash of this current iteration of the store
    pub store_hash: Option<String>,
    pub target_hash: Option<String>,
    pub operation_mode: OperationMode,
    pub current_version: Option<StoreVersion>,
}

impl StowawayContext {
    pub fn default(
        config: StowawayConfig,
        source: std::path::PathBuf,
        target: std::path::PathBuf,
        flags: ExecutionFlags,
    ) -> Self {
        StowawayContext {
            config,
            source_dir: source,
            target_dir: target,
            dry_run: flags.is_dry_run,
            files: Vec::new(),
            store_hash: None,
            operation_mode: crate::context::OperationMode::Create,
            current_version: None,
            target_hash: None,
        }
    }
}
