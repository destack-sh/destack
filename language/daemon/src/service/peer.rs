use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use tspp_rpc::{Code, Request, Response, Status};
use tspp_runtime::service::WorldId;
use tspp_workspace::Workspace;

use crate::{
    CloseWorkspaceRequest, CloseWorldRequest, ConnectionId, CreateWorldRequest, Daemon,
    DaemonService, OpenWorkspaceRequest, OpenWorkspaceResponse, RestoreWorldRequest,
};

/// Daemon operations bound to one RPC peer.
#[derive(Debug, Clone)]
pub(crate) struct DaemonPeer {
    /// Shared daemon process.
    daemon: Daemon,
    /// Calling connection.
    connection: ConnectionId,
    /// Workspace roots opened by this connection.
    workspace_roots: Arc<RwLock<HashSet<PathBuf>>>,
}

impl DaemonPeer {
    /// Bind daemon operations to one connection.
    pub(crate) fn new(daemon: Daemon, connection: ConnectionId) -> Self {
        Self {
            daemon,
            connection,
            workspace_roots: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Open one shared workspace for this connection.
    pub(crate) fn open_workspace(&self, root: &Path) -> Result<Arc<Workspace>, Status> {
        let workspace = self.daemon.workspaces().open(root)?;
        self.workspace_roots
            .write()
            .insert(workspace.root().to_path_buf());

        Ok(workspace)
    }

    /// Return one workspace by its canonical root opened on this connection.
    pub(crate) fn workspace(&self, root: &Path) -> Result<Arc<Workspace>, Status> {
        if !self.workspace_roots.read().contains(root) {
            return Err(Status::new(
                Code::NotFound,
                format!(
                    "workspace is not open on this connection: {}",
                    root.display()
                ),
            ));
        }

        self.daemon.workspaces().workspace(root)
    }

    /// Release one canonical workspace root from this connection.
    pub(crate) fn close_workspace(&self, root: &Path) {
        self.workspace_roots.write().remove(root);
    }
}

impl DaemonService for DaemonPeer {
    /// Open one workspace in this daemon.
    async fn open_workspace(
        &self,
        request: Request<OpenWorkspaceRequest>,
    ) -> Result<Response<OpenWorkspaceResponse>, Status> {
        let workspace = self.open_workspace(&request.value.root)?;
        let root = workspace.root().to_path_buf();

        Ok(Response::new(OpenWorkspaceResponse { root }))
    }

    /// Release one workspace from this connection.
    async fn close_workspace(
        &self,
        request: Request<CloseWorkspaceRequest>,
    ) -> Result<Response<()>, Status> {
        self.close_workspace(&request.value.root);

        Ok(Response::new(()))
    }

    /// Create one World in this daemon.
    async fn create_world(
        &self,
        request: Request<CreateWorldRequest>,
    ) -> Result<Response<WorldId>, Status> {
        let request = request.value;
        let options = request.options.unwrap_or_default();
        let environment = request.environment.unwrap_or_default();
        let world_id = self.daemon.worlds().create(options, environment)?;

        Ok(Response::new(world_id))
    }

    /// List the Worlds hosted by this daemon.
    async fn list_worlds(&self, _request: Request<()>) -> Result<Response<Vec<WorldId>>, Status> {
        Ok(Response::new(self.daemon.worlds().list()))
    }

    /// Restore one World from a Blob-backed Snapshot.
    async fn restore_world(
        &self,
        request: Request<RestoreWorldRequest>,
    ) -> Result<Response<WorldId>, Status> {
        let world_id = self.daemon.worlds().restore(request.value.snapshot)?;

        Ok(Response::new(world_id))
    }

    /// Close one World in this daemon.
    async fn close_world(
        &self,
        request: Request<CloseWorldRequest>,
    ) -> Result<Response<()>, Status> {
        self.daemon.worlds().close(request.value.world_id);

        Ok(Response::new(()))
    }

    /// Request orderly daemon shutdown.
    async fn shutdown(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        self.daemon.lifecycle().shutdown_after(self.connection);

        Ok(Response::new(()))
    }
}
