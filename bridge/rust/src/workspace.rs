use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{DestackLayoutOverride, Environment, Settings, open_repository_from_fs};
use destack_source::{OverlayFileSystem, PhysicalFileSystem};
use destack_workspace::{LocalWorkspace, Server};

use crate::{Error, Result};

/// In-process workspace protocol server.
#[derive(Debug, Clone)]
pub struct LocalWorkspaceServer {
    /// Protocol server backing this local server.
    server: Arc<Server>,
}

/// In-process workspace protocol server options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalWorkspaceServerOptions {
    /// Number of workers used by each opened root.
    pub worker_limit: usize,
}

impl Default for LocalWorkspaceServerOptions {
    fn default() -> Self {
        Self {
            worker_limit: LocalWorkspace::default_worker_count(),
        }
    }
}

impl LocalWorkspaceServer {
    /// Open an in-process workspace protocol server.
    pub fn open(home: impl Into<PathBuf>) -> Result<Self> {
        Self::with_options(home, LocalWorkspaceServerOptions::default())
    }

    /// Open an in-process workspace protocol server with explicit options.
    pub fn with_options(
        home: impl Into<PathBuf>,
        options: LocalWorkspaceServerOptions,
    ) -> Result<Self> {
        let home = home.into();
        let workspace = local_workspace(home, options.worker_limit)?;
        let server = Arc::new(Server::new(workspace));

        Ok(Self { server })
    }

    /// Dispatch one encoded protocol frame.
    pub fn dispatch(&self, payload: &[u8]) -> Result<Vec<Vec<u8>>> {
        self.server.dispatch(payload).map_err(Error::new)
    }
}

/// Open one local workspace.
fn local_workspace(home: PathBuf, worker_limit: usize) -> Result<Arc<LocalWorkspace>> {
    let home = workspace_path(&home)?;
    let overlay = Arc::new(OverlayFileSystem::with_inner(Arc::new(PhysicalFileSystem)));
    let repository = Arc::new(
        open_repository_from_fs(
            home,
            overlay.clone(),
            Environment::capture_process(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )
        .map_err(Error::new)?,
    );
    let workspace = LocalWorkspace::new(
        repository,
        Some(overlay),
        None,
        Vec::new(),
        worker_limit,
        None,
    )
    .map_err(Error::new)?;

    Ok(Arc::new(workspace))
}

/// Return a stable local workspace path.
fn workspace_path(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path)
        .map_err(|error| Error::new(format!("failed to canonicalize workspace path: {error}")))
}
