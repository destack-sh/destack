use super::{CloseWorkspaceRequest, OpenWorkspaceRequest, OpenWorkspaceResponse};

/// RPC operations on one Destack daemon.
#[destack_rpc::service(name = "destack.daemon.Daemon")]
pub trait DaemonService {
    /// Open one workspace in this daemon.
    #[rpc(name = "OpenWorkspace", idempotency = "idempotent")]
    fn open_workspace(request: OpenWorkspaceRequest) -> OpenWorkspaceResponse;

    /// Close one workspace in this daemon.
    #[rpc(name = "CloseWorkspace", idempotency = "idempotent")]
    fn close_workspace(request: CloseWorkspaceRequest) -> ();

    /// Request orderly daemon shutdown.
    #[rpc(name = "Shutdown", idempotency = "idempotent")]
    fn shutdown(request: ()) -> ();
}
