use windows_sys::Win32::Networking::WinSock::{SD_BOTH, SD_RECEIVE, SD_SEND, shutdown};

use super::util::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::net::{SocketHandle, SocketShutdown};
use crate::runtime::BindingCallContext;

/// Shut down a socket for reads, writes, or both.
pub(crate) unsafe fn destack_net_shutdown(
    binding: &BindingCallContext,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // map the shutdown mode
    let how = match how {
        SocketShutdown::Read => SD_RECEIVE,
        SocketShutdown::Write => SD_SEND,
        SocketShutdown::ReadWrite => SD_BOTH,
    };

    // issue the shutdown
    let rc = unsafe { shutdown(socket, how) };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "shutdown",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}
