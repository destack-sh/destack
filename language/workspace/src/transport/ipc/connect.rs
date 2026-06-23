use std::path::Path;
use std::sync::Arc;

use crate::Transport;

use super::IpcError;
use super::platform::connect_stream;
use super::transport::ipc_transport;

/// Connect to a workspace ipc socket.
pub(crate) fn connect_ipc(
    path: &Path,
    max_frame_bytes: usize,
) -> Result<Arc<dyn Transport>, IpcError> {
    // connect to the socket
    let stream = connect_stream(path)?;

    Ok(ipc_transport(stream, max_frame_bytes))
}
