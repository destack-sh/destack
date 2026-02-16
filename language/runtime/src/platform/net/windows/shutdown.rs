use windows_sys::Win32::Networking::WinSock::{SD_BOTH, SD_RECEIVE, SD_SEND, shutdown};

use super::util::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::net::{SocketHandle, SocketShutdown};
use crate::runtime::RuntimeCallContext;

/// Shut down a socket for reads, writes, or both.
///
/// Shut down read, write, or both directions on a connected socket.
/// Half-close and peer-observed behavior follow host kernel shutdown semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses shutdown(2) on Unix and shutdown on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.close`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_shutdown(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // map the shutdown mode
    let how = match how {
        SocketShutdown::Read => SD_RECEIVE,
        SocketShutdown::Write => SD_SEND,
        SocketShutdown::ReadWrite => SD_BOTH,
    };

    // issue the shutdown
    let rc = unsafe { shutdown(socket, how) };
    if rc != 0 {
        return Err(last_net_error("shutdown"));
    }

    Ok(())
}
