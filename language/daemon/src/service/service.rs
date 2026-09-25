use tspp_runtime::service::WorldId;

use super::{
    CloseWorkspaceRequest, CloseWorldRequest, CreateWorldRequest, OpenWorkspaceRequest,
    OpenWorkspaceResponse, RestoreWorldRequest,
};

/// RPC operations on one TS++ daemon.
#[tspp_rpc::service(name = "tspp.daemon.Daemon")]
pub trait DaemonService {
    // =============================================================================
    // Workspace
    // =============================================================================

    /// Open one workspace in this daemon.
    #[rpc(name = "OpenWorkspace", idempotency = "idempotent")]
    fn open_workspace(request: OpenWorkspaceRequest) -> OpenWorkspaceResponse;

    /// Release one workspace from the calling connection.
    #[rpc(name = "CloseWorkspace", idempotency = "idempotent")]
    fn close_workspace(request: CloseWorkspaceRequest) -> ();

    // =============================================================================
    // World
    // =============================================================================

    /// Create one World in this daemon.
    #[rpc(name = "CreateWorld")]
    fn create_world(request: CreateWorldRequest) -> WorldId;

    /// List the Worlds hosted by this daemon.
    #[rpc(name = "ListWorlds", idempotency = "no_side_effects")]
    fn list_worlds(request: ()) -> Vec<WorldId>;

    /// Restore one World from a Blob-backed Snapshot.
    #[rpc(name = "RestoreWorld")]
    fn restore_world(request: RestoreWorldRequest) -> WorldId;

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
