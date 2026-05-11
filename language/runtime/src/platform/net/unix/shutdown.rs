use super::core::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::net::*;
use crate::platform::resource::*;
use crate::runtime::BindingCallContext;

/// Shut down a socket for reads, writes, or both.
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
