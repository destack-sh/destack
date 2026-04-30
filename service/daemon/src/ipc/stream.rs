use std::io;
use std::path::Path;

use super::DaemonIpcError;

#[cfg(windows)]
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

/// Listener type for daemon ipc connections.
#[cfg(unix)]
pub(super) type IpcListener = UnixListener;
/// Stream type for daemon ipc connections.
#[cfg(unix)]
pub(super) type IpcStream = UnixStream;

/// Listener type for daemon ipc connections.
#[cfg(windows)]
pub(super) type IpcListener = LocalSocketListener;
/// Stream type for daemon ipc connections.
#[cfg(windows)]
pub(super) type IpcStream = LocalSocketStream;

/// Bind a listener to a socket path.
#[cfg(unix)]
pub(super) fn bind_listener(path: &Path) -> Result<IpcListener, DaemonIpcError> {
    // bind the unix socket
    let listener = UnixListener::bind(path).map_err(DaemonIpcError::Io)?;

    // enable nonblocking accepts
    listener.set_nonblocking(true).map_err(DaemonIpcError::Io)?;

    Ok(listener)
}

/// Bind a listener to a socket path.
#[cfg(windows)]
pub(super) fn bind_listener(path: &Path) -> Result<IpcListener, DaemonIpcError> {
    // resolve the pipe name
    let name = path_to_pipe_name(path);

    // bind the listener
    LocalSocketListener::bind(name).map_err(DaemonIpcError::Io)
}

/// Accept a single stream from the listener.
#[cfg(unix)]
pub(super) fn accept_stream(listener: &IpcListener) -> Result<IpcStream, DaemonIpcError> {
    // accept a new connection
    match listener.accept() {
        Ok((stream, _)) => {
            // reset to blocking mode for protocol IO
            stream.set_nonblocking(false).map_err(DaemonIpcError::Io)?;
            Ok(stream)
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Err(DaemonIpcError::WouldBlock),
        Err(error) => Err(DaemonIpcError::Io(error)),
    }
}

/// Accept a single stream from the listener.
#[cfg(windows)]
pub(super) fn accept_stream(listener: &IpcListener) -> Result<IpcStream, DaemonIpcError> {
    listener.accept().map_err(DaemonIpcError::Io)
}

/// Connect to a socket path and return a stream.
#[cfg(unix)]
pub(super) fn connect_stream(path: &Path) -> Result<IpcStream, DaemonIpcError> {
    // connect to the unix socket
    UnixStream::connect(path).map_err(DaemonIpcError::Io)
}

/// Connect to a socket path and return a stream.
#[cfg(windows)]
pub(super) fn connect_stream(path: &Path) -> Result<IpcStream, DaemonIpcError> {
    // resolve the pipe name
    let name = path_to_pipe_name(path);

    // connect to the pipe
    LocalSocketStream::connect(name).map_err(DaemonIpcError::Io)
}

/// Convert a socket path to a local pipe name.
#[cfg(windows)]
fn path_to_pipe_name(path: &Path) -> String {
    // derive a pipe name from the socket path
    let root_id = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("root");
    format!(r"\\.\pipe\destack-{root_id}")
}
