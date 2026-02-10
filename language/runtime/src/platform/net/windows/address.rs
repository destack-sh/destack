use std::mem;
use windows_sys::Win32::Networking::WinSock::{
    SOCKADDR, SOCKADDR_STORAGE, getpeername, getsockname,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::net::{SocketAddress, SocketHandle};
use crate::runtime::RuntimeCallContext;

/// Return the local address bytes for a socket.
pub(crate) unsafe fn destack_net_local_address_raw(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // query the local address
    let mut storage = mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getsockname(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(last_net_error("getsockname"));
    }

    // decode and write the output
    let storage = unsafe { storage.assume_init() };
    let bytes = unsafe {
        let pointer = &storage as *const _ as *const u8;
        std::slice::from_raw_parts(pointer, length as usize)
    };
    let address = SocketAddress {
        family: storage.ss_family,
        length: length as u32,
        bytes: context.store_array(bytes.to_vec()),
    };
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Return the peer address bytes for a socket.
pub(crate) unsafe fn destack_net_peer_address_raw(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // query the peer address
    let mut storage = mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getpeername(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(last_net_error("getpeername"));
    }

    // decode and write the output
    let storage = unsafe { storage.assume_init() };
    let bytes = unsafe {
        let pointer = &storage as *const _ as *const u8;
        std::slice::from_raw_parts(pointer, length as usize)
    };
    let address = SocketAddress {
        family: storage.ss_family,
        length: length as u32,
        bytes: context.store_array(bytes.to_vec()),
    };
    unsafe {
        *out = address;
    }

    Ok(())
}
