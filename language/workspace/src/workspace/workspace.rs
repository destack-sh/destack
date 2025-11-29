use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{FileRegistry, FileSystem};

#[derive(Debug, Clone)]
pub struct Workspace {
    /// The current working directory.
    pub cwd: PathBuf,
    /// The file system.
    pub fs: Arc<dyn FileSystem>,
    /// The files in the workspace.
    pub files: Arc<FileRegistry>,
}

impl Workspace {}
