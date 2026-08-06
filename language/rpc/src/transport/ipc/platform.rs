use std::io;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;
use std::time::Duration;

#[cfg(windows)]
use interprocess::TryClone;
#[cfg(windows)]
use interprocess::local_socket::{
    ListenerNonblockingMode, LocalSocketListener, LocalSocketStream,
    traits::{Listener as _, Stream as _},
};
#[cfg(unix)]
use std::net::Shutdown;
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use super::IpcError;

/// Platform listener for local RPC connections.
#[cfg(unix)]
pub(super) type PlatformIpcListener = UnixListener;
/// Platform stream for local RPC connections.
#[cfg(unix)]
pub(super) type IpcStream = UnixStream;

/// Platform listener for local RPC connections.
#[cfg(windows)]
pub(super) type PlatformIpcListener = LocalSocketListener;
/// Platform stream for local RPC connections.
#[cfg(windows)]
pub(super) type IpcStream = LocalSocketStream;

/// Bind one local listener.
#[cfg(unix)]
pub(super) fn bind_listener(path: &Path) -> Result<PlatformIpcListener, IpcError> {
    let listener = UnixListener::bind(path)?;
    listener.set_nonblocking(true)?;

    Ok(listener)
}

/// Bind one local listener.
#[cfg(windows)]
pub(super) fn bind_listener(path: &Path) -> Result<PlatformIpcListener, IpcError> {
    let name = path_to_pipe_name(path)?;
    let listener = LocalSocketListener::bind(name).map_err(IpcError::Io)?;
    listener
        .set_nonblocking(ListenerNonblockingMode::Accept)
        .map_err(IpcError::Io)?;

    Ok(listener)
}

/// Accept one local stream when ready.
#[cfg(unix)]
pub(super) fn accept_stream(listener: &PlatformIpcListener) -> Result<Option<IpcStream>, IpcError> {
    match listener.accept() {
        Ok((stream, _)) => {
            stream.set_nonblocking(false)?;

            Ok(Some(stream))
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Accept one local stream when ready.
#[cfg(windows)]
pub(super) fn accept_stream(listener: &PlatformIpcListener) -> Result<Option<IpcStream>, IpcError> {
    match listener.accept() {
        Ok(stream) => Ok(Some(stream)),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(IpcError::Io(error)),
    }
}

/// Connect one local stream.
#[cfg(unix)]
pub(super) fn connect_stream(path: &Path) -> Result<IpcStream, IpcError> {
    UnixStream::connect(path).map_err(Into::into)
}

/// Connect one local stream.
#[cfg(windows)]
pub(super) fn connect_stream(path: &Path) -> Result<IpcStream, IpcError> {
    let name = path_to_pipe_name(path)?;

    LocalSocketStream::connect(name).map_err(IpcError::Io)
}

/// Duplicate one local stream for independent reads and writes.
#[cfg(unix)]
pub(super) fn clone_stream(stream: &IpcStream) -> Result<IpcStream, IpcError> {
    stream.try_clone().map_err(Into::into)
}

/// Duplicate one local stream for independent reads and writes.
#[cfg(windows)]
pub(super) fn clone_stream(stream: &IpcStream) -> Result<IpcStream, IpcError> {
    stream.try_clone().map_err(IpcError::Io)
}

/// Bound receive blocking so close can interrupt this stream.
#[cfg(unix)]
pub(super) fn configure_stream(stream: &IpcStream, timeout: Duration) -> Result<(), IpcError> {
    stream.set_read_timeout(Some(timeout)).map_err(Into::into)
}

/// Bound receive blocking so close can interrupt this stream.
#[cfg(windows)]
pub(super) fn configure_stream(stream: &IpcStream, timeout: Duration) -> Result<(), IpcError> {
    stream.set_recv_timeout(Some(timeout)).map_err(IpcError::Io)
}

/// Interrupt all pending input and output.
#[cfg(unix)]
pub(super) fn interrupt_stream(stream: &IpcStream) -> Result<(), IpcError> {
    stream.shutdown(Shutdown::Both).map_err(Into::into)
}

/// Let the configured receive timeout observe connection closure.
#[cfg(windows)]
pub(super) fn interrupt_stream(_stream: &IpcStream) -> Result<(), IpcError> {
    Ok(())
}

/// Convert a filesystem path into a named pipe.
#[cfg(windows)]
fn path_to_pipe_name(path: &Path) -> Result<String, IpcError> {
    let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
        return Err(IpcError::InvalidPath(PathBuf::from(path)));
    };

    Ok(format!(r"\\.\pipe\destack-{name}"))
}
