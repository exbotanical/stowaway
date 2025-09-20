use crate::{config::StowawayConfig, file_entry::FileEntry};

pub struct StowawayContext {
    pub config: StowawayConfig,
    pub source_dir: std::path::PathBuf,
    pub target_dir: std::path::PathBuf,
    pub dry_run: bool,
    pub files: Vec<FileEntry>,
    pub store_hash: Option<String>,
}
