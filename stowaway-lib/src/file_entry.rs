#[derive(Debug, Clone)]
pub struct FileEntry {
    pub source_path: std::path::PathBuf,
    pub relative_path: std::path::PathBuf,
    pub target_path: std::path::PathBuf,
    pub should_interpolate: bool,
}
