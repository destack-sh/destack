use std::path::PathBuf;

/// Physical paths whose repository state may have changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWatchEvent {
    /// Distinct paths accumulated before delivery.
    pub paths: Vec<PathBuf>,
    /// Whether the host requires an authoritative rescan.
    pub is_rescan: bool,
}
