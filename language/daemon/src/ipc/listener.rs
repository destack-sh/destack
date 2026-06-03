use std::path::Path;
use std::sync::Arc;

use crate::protocol::Transport;

use super::DaemonIpcError;
use super::platform::{IpcListener, accept_stream, bind_listener};
use super::transport::ipc_transport;

/// Listener for daemon ipc connections.
#[derive(Debug)]
pub struct DaemonIpcListener {
    /// Underlying socket listener.
    listener: IpcListener,
    /// Maximum frame size for transports.
    max_frame_bytes: usize,
}

impl DaemonIpcListener {
    /// Bind a listener to a socket path.
    pub fn bind(path: &Path, max_frame_bytes: usize) -> Result<Self, DaemonIpcError> {
        // bind the listener
        let listener = bind_listener(path)?;

        Ok(Self {
            listener,
            max_frame_bytes,
        })
    }

    /// Accept an incoming connection.
    pub fn accept(&self) -> Result<Arc<dyn Transport>, DaemonIpcError> {
        // accept the next stream
        let stream = accept_stream(&self.listener)?;

        Ok(ipc_transport(stream, self.max_frame_bytes))
    }
}
