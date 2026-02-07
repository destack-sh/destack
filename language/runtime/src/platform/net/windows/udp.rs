use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, INVALID_SOCKET, IPPROTO_UDP, SOCK_DGRAM, socket,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::net::{SocketFamily, SocketHandle};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::runtime::RuntimeCallContext;

/// Create a UDP socket.
pub(crate) unsafe fn destack_net_udp_socket(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // select the socket family
    let family = match family {
        SocketFamily::IPv4 => AF_INET as i32,
        SocketFamily::IPv6 => AF_INET6 as i32,
        SocketFamily::Unspecified => AF_INET as i32,
    };

    // create the socket
    let socket = unsafe { socket(family, SOCK_DGRAM, IPPROTO_UDP) };
    if socket == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    // register the socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
}
