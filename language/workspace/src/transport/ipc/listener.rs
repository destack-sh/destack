use std::path::Path;
use std::sync::Arc;

use crate::Transport;

use super::IpcError;
use super::platform::{PlatformIpcListener, accept_stream, bind_listener};
use super::transport::ipc_transport;

/// Listener for workspace ipc connections.
#[derive(Debug)]
pub struct IpcListener {
    /// Underlying socket listener.
    listener: PlatformIpcListener,
    /// Maximum frame size for transports.
    max_frame_bytes: usize,
}

impl IpcListener {
    /// Bind a listener to a socket path.
    pub fn bind(path: &Path, max_frame_bytes: usize) -> Result<Self, IpcError> {
        // bind the listener
        let listener = bind_listener(path)?;

        Ok(Self {
            listener,
            max_frame_bytes,
        })
    }

    /// Accept an incoming connection when one is ready.
    pub fn try_accept(&self) -> Result<Option<Arc<dyn Transport>>, IpcError> {
        // accept the next stream
        let Some(stream) = accept_stream(&self.listener)? else {
            return Ok(None);
        };

        Ok(Some(ipc_transport(stream, self.max_frame_bytes)))
    }
}
