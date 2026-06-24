use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{
    DestackLayoutOverride, Environment, Settings, open_repository_from_fs,
    open_repository_from_memory,
};
use destack_source::{Edit, OverlayFileSystem, PhysicalFileSystem};

use crate::{Error, LocalWorkspace};

use super::Server;

impl Server {
    /// Open a protocol server for one local workspace path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, Error> {
        Self::open_with_worker_limit(path, LocalWorkspace::default_worker_count())
    }

    /// Open a protocol server for one local workspace path with an explicit worker limit.
    pub fn open_with_worker_limit(
        path: impl Into<PathBuf>,
        worker_limit: usize,
    ) -> Result<Self, Error> {
        let workspace = LocalWorkspace::open(path, worker_limit)?;

        Ok(Self::new(Arc::new(workspace)))
    }

    /// Open a protocol server from in-memory source edits.
    pub fn memory(root: impl Into<PathBuf>, edits: Vec<Edit>) -> Result<Self, Error> {
        Self::memory_with_worker_limit(root, edits, LocalWorkspace::default_worker_count())
    }

    /// Open a protocol server from in-memory source edits with an explicit worker limit.
    pub fn memory_with_worker_limit(
        root: impl Into<PathBuf>,
        edits: Vec<Edit>,
        worker_limit: usize,
    ) -> Result<Self, Error> {
        let workspace = LocalWorkspace::memory(root, edits, worker_limit)?;

        Ok(Self::new(Arc::new(workspace)))
    }
}

impl LocalWorkspace {
    /// Open a local workspace from one filesystem path.
    pub fn open(path: impl Into<PathBuf>, worker_limit: usize) -> Result<Self, Error> {
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

        Self::new(
            repository,
            Some(file_system),
            None,
            Vec::new(),
            worker_limit,
            None,
        )
    }

    /// Open a local workspace from in-memory source edits.
    pub fn memory(
        root: impl Into<PathBuf>,
        edits: Vec<Edit>,
        worker_limit: usize,
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

        Self::new(repository, None, None, vec![root], worker_limit, None)
    }
}

/// Return a stable local workspace path.
fn workspace_path(path: &Path) -> Result<PathBuf, Error> {
    std::fs::canonicalize(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}
