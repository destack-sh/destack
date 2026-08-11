use destack_runtime::service::WorldId;

use super::{
    CloseWorkspaceRequest, CloseWorldRequest, CreateWorldRequest, OpenWorkspaceRequest,
    OpenWorkspaceResponse,
};

/// RPC operations on one Destack daemon.
#[destack_rpc::service(name = "destack.daemon.Daemon")]
pub trait DaemonService {
    // =============================================================================
    // Workspace
    // =============================================================================

    /// Open one workspace in this daemon.
    #[rpc(name = "OpenWorkspace", idempotency = "idempotent")]
    fn open_workspace(request: OpenWorkspaceRequest) -> OpenWorkspaceResponse;

    /// Close one workspace in this daemon.
    #[rpc(name = "CloseWorkspace", idempotency = "idempotent")]
    fn close_workspace(request: CloseWorkspaceRequest) -> ();

    // =============================================================================
    // World
    // =============================================================================

    /// Create one World in this daemon.
    #[rpc(name = "CreateWorld")]
    fn create_world(request: CreateWorldRequest) -> WorldId;

    /// Close one World in this daemon.
    #[rpc(name = "CloseWorld", idempotency = "idempotent")]
    fn close_world(request: CloseWorldRequest) -> ();

    // =============================================================================
    // Process
    // =============================================================================

    /// Request orderly daemon shutdown.
    #[rpc(name = "Shutdown", idempotency = "idempotent")]
    fn shutdown(request: ()) -> ();
}
