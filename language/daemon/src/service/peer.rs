use destack_rpc::{Request, Response, Status};
use destack_runtime::service::WorldId;

use crate::{
    CloseWorkspaceRequest, CloseWorldRequest, ConnectionId, CreateWorldRequest, Daemon,
    DaemonService, OpenWorkspaceRequest, OpenWorkspaceResponse,
};

/// Daemon operations bound to one RPC peer.
#[derive(Debug, Clone)]
pub(crate) struct DaemonPeer {
    /// Shared daemon process.
    daemon: Daemon,
    /// Calling connection.
    connection: ConnectionId,
}

impl DaemonPeer {
    /// Bind daemon operations to one connection.
    pub(crate) const fn new(daemon: Daemon, connection: ConnectionId) -> Self {
        Self { daemon, connection }
    }
}

impl DaemonService for DaemonPeer {
    /// Open one workspace in this daemon.
    async fn open_workspace(
        &self,
        request: Request<OpenWorkspaceRequest>,
    ) -> Result<Response<OpenWorkspaceResponse>, Status> {
        let workspace = self.daemon.workspaces().open(&request.value.root)?;
        let root = workspace.root().to_path_buf();
        let revision = workspace.revision()?;

        Ok(Response::new(OpenWorkspaceResponse { root, revision }))
    }

    /// Close one workspace in this daemon.
    async fn close_workspace(
        &self,
        request: Request<CloseWorkspaceRequest>,
    ) -> Result<Response<()>, Status> {
        self.daemon.workspaces().close(&request.value.root)?;

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
