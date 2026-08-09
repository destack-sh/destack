use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::IpcError;
use super::platform::{PlatformIpcListener, accept_stream, bind_listener, connect_stream};
use super::transport::IpcTransport;
use crate::{Listener, Transport};

/// Listener for framed local RPC connections.
#[derive(Debug)]
pub struct IpcListener {
    /// Platform listener.
    listener: PlatformIpcListener,
    /// Bound local connection path.
    path: PathBuf,
    /// Largest accepted transport message.
    max_message_bytes: usize,
    /// Whether closure was requested.
    is_closed: AtomicBool,
}

impl IpcListener {
    /// Bind one local RPC listener.
    pub fn bind(path: &Path, max_message_bytes: usize) -> Result<Self, IpcError> {
        let listener = bind_listener(path)?;

        Ok(Self {
            listener,
            path: path.to_path_buf(),
            max_message_bytes,
            is_closed: AtomicBool::new(false),
        })
    }
}

impl Listener for IpcListener {
    type Error = IpcError;

    /// Accept one framed local connection, or return `None` after closure.
    fn accept(&self) -> Result<Option<Arc<dyn Transport>>, Self::Error> {
        if self.is_closed.load(Ordering::Acquire) {
            return Ok(None);
        }

        let stream = accept_stream(&self.listener)?;
        if self.is_closed.load(Ordering::Acquire) {
            drop(stream);

            return Ok(None);
        }
        let transport: Arc<dyn Transport> = IpcTransport::new(stream, self.max_message_bytes)?;

        Ok(Some(transport))
    }

    /// Close this listener and interrupt a pending accept.
    fn close(&self) -> Result<(), Self::Error> {
        if self.is_closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let stream = connect_stream(&self.path)?;
        drop(stream);

        Ok(())
    }
}
