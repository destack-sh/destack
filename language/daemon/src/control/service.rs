/// RPC operations controlling one Destack daemon process.
#[destack_rpc::service(name = "destack.daemon.Control")]
pub trait ControlService {
    /// Request orderly daemon shutdown.
    #[rpc(name = "Shutdown")]
    fn shutdown(request: ()) -> ();
}
