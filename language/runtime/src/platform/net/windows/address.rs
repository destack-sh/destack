use std::mem;

use windows_sys::Win32::Networking::WinSock::{
    SOCKADDR, SOCKADDR_STORAGE, getpeername, getsockname,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::net::{SocketAddress, SocketHandle};
use crate::runtime::RuntimeCallContext;

/// Return the local address for a socket.
pub(crate) unsafe fn destack_net_local_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let socket = socket_descriptor(context, handle)?;
    let mut storage = mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getsockname(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(last_net_error("getsockname"));
    }
    let storage = unsafe { storage.assume_init() };
    let address = socket_address_from_storage(context, &storage)?;
    unsafe {
        *out = address;
    }
    Ok(())
}

/// Return the peer address for a socket.
pub(crate) unsafe fn destack_net_peer_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let socket = socket_descriptor(context, handle)?;
    let mut storage = mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getpeername(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(last_net_error("getpeername"));
    }
    let storage = unsafe { storage.assume_init() };
    let address = socket_address_from_storage(context, &storage)?;
    unsafe {
        *out = address;
    }
    Ok(())
}
