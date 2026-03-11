use super::core::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::net::*;
use crate::platform::resource::*;
use crate::runtime::BindingCallContext;

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
    binding: &BindingCallContext,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    // shutdown sockets on unix platforms
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

    // issue the shutdown
    let rc = unsafe { libc::shutdown(fd, shutdown_how(how)) };
    if rc != 0 {
        return Err(core_platform::net_error("shutdown"));
    }

    Ok(())
}
