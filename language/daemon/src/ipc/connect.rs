use std::path::Path;
use std::sync::Arc;

use crate::protocol::Transport;

use super::DaemonIpcError;
use super::platform::connect_stream;
use super::transport::ipc_transport;

/// Connect to a daemon ipc socket.
pub fn connect_ipc(
    path: &Path,
    max_frame_bytes: usize,
) -> Result<Arc<dyn Transport>, DaemonIpcError> {
    // connect to the socket
    let stream = connect_stream(path)?;

    Ok(ipc_transport(stream, max_frame_bytes))
}
