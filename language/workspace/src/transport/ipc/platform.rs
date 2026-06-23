use std::io;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;

use super::IpcError;

#[cfg(windows)]
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

/// Platform listener type for workspace ipc connections.
#[cfg(unix)]
pub(super) type PlatformIpcListener = UnixListener;
/// Stream type for workspace ipc connections.
#[cfg(unix)]
pub(super) type IpcStream = UnixStream;

/// Platform listener type for workspace ipc connections.
#[cfg(windows)]
pub(super) type PlatformIpcListener = LocalSocketListener;
/// Stream type for workspace ipc connections.
#[cfg(windows)]
pub(super) type IpcStream = LocalSocketStream;

/// Bind a listener to a socket path.
#[cfg(unix)]
pub(super) fn bind_listener(path: &Path) -> Result<PlatformIpcListener, IpcError> {
    // bind the unix socket
    let listener = UnixListener::bind(path).map_err(IpcError::Io)?;

    // enable nonblocking accepts
    listener.set_nonblocking(true).map_err(IpcError::Io)?;

    Ok(listener)
}

/// Bind a listener to a socket path.
#[cfg(windows)]
pub(super) fn bind_listener(path: &Path) -> Result<PlatformIpcListener, IpcError> {
    // resolve the pipe name
    let name = path_to_pipe_name(path)?;

    // bind the listener
    LocalSocketListener::bind(name).map_err(IpcError::Io)
}

/// Accept a single stream from the listener.
#[cfg(unix)]
pub(super) fn accept_stream(listener: &PlatformIpcListener) -> Result<Option<IpcStream>, IpcError> {
    // accept a new connection
    match listener.accept() {
        Ok((stream, _)) => {
            // reset to blocking mode for protocol IO
            stream.set_nonblocking(false).map_err(IpcError::Io)?;
            Ok(Some(stream))
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(IpcError::Io(error)),
    }
}

/// Accept a single stream from the listener.
#[cfg(windows)]
pub(super) fn accept_stream(listener: &PlatformIpcListener) -> Result<Option<IpcStream>, IpcError> {
    listener.accept().map(Some).map_err(IpcError::Io)
}

/// Connect to a socket path and return a stream.
#[cfg(unix)]
pub(super) fn connect_stream(path: &Path) -> Result<IpcStream, IpcError> {
    // connect to the unix socket
    UnixStream::connect(path).map_err(IpcError::Io)
}

/// Connect to a socket path and return a stream.
#[cfg(windows)]
pub(super) fn connect_stream(path: &Path) -> Result<IpcStream, IpcError> {
    // resolve the pipe name
    let name = path_to_pipe_name(path)?;

    // connect to the pipe
    LocalSocketStream::connect(name).map_err(IpcError::Io)
}

/// Convert a socket path to a local pipe name.
#[cfg(windows)]
fn path_to_pipe_name(path: &Path) -> Result<String, IpcError> {
    // derive a pipe name from the socket path
    let Some(root_id) = path.file_stem().and_then(|name| name.to_str()) else {
        return Err(IpcError::InvalidPath(PathBuf::from(path)));
    };

    Ok(format!(r"\\.\pipe\destack-{root_id}"))
}
