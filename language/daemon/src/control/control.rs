use destack_rpc::{Request, Response, Status};

use super::ControlService;
use crate::DaemonLifecycle;

/// Control operations for one running daemon.
#[derive(Debug, Clone)]
pub struct Control {
    /// Daemon lifecycle to control.
    lifecycle: DaemonLifecycle,
}

impl Control {
    /// Create daemon control operations.
    pub fn new(lifecycle: DaemonLifecycle) -> Self {
        Self { lifecycle }
    }
}

impl ControlService for Control {
    /// Request orderly daemon shutdown.
    async fn shutdown(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        self.lifecycle.shutdown();

        Ok(Response::new(()))
    }
}
