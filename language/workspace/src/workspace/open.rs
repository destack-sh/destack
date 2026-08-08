use std::path::PathBuf;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use destack_repository::{
    DestackLayoutOverride, Environment, Settings, open_repository_from_memory,
};
use destack_source::Edit;

#[cfg(not(target_arch = "wasm32"))]
use destack_repository::open_repository_from_fs;
#[cfg(not(target_arch = "wasm32"))]
use destack_source::{OverlayFileSystem, PhysicalFileSystem};

use super::LocalWorkspace;
use crate::Error;

impl LocalWorkspace {
    /// Open a local workspace from one filesystem path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(path: impl Into<PathBuf>, worker_count: usize) -> Result<Self, Error> {
        let path = workspace_path(&path.into())?;
        let file_system = Arc::new(OverlayFileSystem::with_inner(Arc::new(PhysicalFileSystem)));
        let repository = Arc::new(
            open_repository_from_fs(
                path,
                file_system.clone(),
                Environment::capture_process(),
                Settings::default(),
                DestackLayoutOverride::default(),
            )
            .map_err(Error::from)?,
        );

        Self::new(repository, Some(file_system), Vec::new(), worker_count)
    }

    /// Open a local workspace from in-memory source edits.
    pub fn memory(
        root: impl Into<PathBuf>,
        edits: Vec<Edit>,
        worker_count: usize,
    ) -> Result<Self, Error> {
        let root = root.into();
        let repository = Arc::new(
            open_repository_from_memory(
                root.clone(),
                edits,
                Environment::default(),
                Settings::default(),
                DestackLayoutOverride::default(),
            )
            .map_err(Error::from)?,
        );

        Self::new(repository, None, vec![root], worker_count)
    }
}

/// Return one stable local workspace path.
#[cfg(not(target_arch = "wasm32"))]
fn workspace_path(path: &Path) -> Result<PathBuf, Error> {
    std::fs::canonicalize(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}
