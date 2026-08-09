use std::path::Path;
use std::sync::Arc;

use super::IpcError;
use super::platform::{PlatformIpcListener, accept_stream, bind_listener};
use super::transport::IpcTransport;
use crate::Transport;

/// Listener for framed local RPC connections.
#[derive(Debug)]
pub struct IpcListener {
    /// Platform listener.
    listener: PlatformIpcListener,
    /// Largest accepted transport message.
    max_message_bytes: usize,
}

impl IpcListener {
    /// Bind one local RPC listener.
    pub fn bind(path: &Path, max_message_bytes: usize) -> Result<Self, IpcError> {
        let listener = bind_listener(path)?;

        Ok(Self {
            listener,
            max_message_bytes,
        })
    }

    /// Accept one connection when ready.
    pub fn try_accept(&self) -> Result<Option<Arc<dyn Transport>>, IpcError> {
        let Some(stream) = accept_stream(&self.listener)? else {
            return Ok(None);
        };
        let transport: Arc<dyn Transport> = IpcTransport::new(stream, self.max_message_bytes)?;

        Ok(Some(transport))
    }
}
