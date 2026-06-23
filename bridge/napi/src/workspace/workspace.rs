use napi::Result;
use napi_derive::napi;

use crate::core::to_error;

/// In-process workspace protocol server exposed to Node.
#[derive(Debug)]
#[napi]
pub struct LocalWorkspaceServer {
    /// Local Rust server.
    server: destack::LocalWorkspaceServer,
}

#[napi]
impl LocalWorkspaceServer {
    /// Open an in-process workspace protocol server.
    #[napi(factory)]
    pub fn open(home: String) -> Result<Self> {
        let server = destack::LocalWorkspaceServer::open(home).map_err(to_error)?;

        Ok(Self { server })
    }

    /// Dispatch one encoded protocol message payload.
    #[napi]
    pub fn dispatch(&self, payload: Vec<u8>) -> Result<Vec<Vec<u8>>> {
        self.server.dispatch(&payload).map_err(to_error)
    }
}
