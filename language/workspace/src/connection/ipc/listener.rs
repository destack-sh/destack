use std::path::Path;
use std::sync::Arc;

use crate::connection::Transport;

use super::WorkspaceIpcError;
use super::platform::{IpcListener, accept_stream, bind_listener};
use super::transport::ipc_transport;

/// Listener for workspace ipc connections.
#[derive(Debug)]
pub struct WorkspaceIpcListener {
    /// Underlying socket listener.
    listener: IpcListener,
    /// Maximum frame size for transports.
    max_frame_bytes: usize,
}

impl WorkspaceIpcListener {
    /// Bind a listener to a socket path.
    pub fn bind(path: &Path, max_frame_bytes: usize) -> Result<Self, WorkspaceIpcError> {
        // bind the listener
        let listener = bind_listener(path)?;

        Ok(Self {
            listener,
            max_frame_bytes,
        })
    }

    /// Accept an incoming connection when one is ready.
    pub fn try_accept(&self) -> Result<Option<Arc<dyn Transport>>, WorkspaceIpcError> {
        // accept the next stream
        let Some(stream) = accept_stream(&self.listener)? else {
            return Ok(None);
        };

        Ok(Some(ipc_transport(stream, self.max_frame_bytes)))
    }
}
