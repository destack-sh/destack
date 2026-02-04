use std::mem;

use windows_sys::Win32::Networking::WinSock::{
    FIONBIO, IPPROTO_TCP, SIO_KEEPALIVE_VALS, SO_KEEPALIVE, SO_REUSEADDR, SOL_SOCKET, TCP_NODELAY,
    WSAIoctl, ioctlsocket, setsockopt, tcp_keepalive,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::net::SocketHandle;
use crate::runtime::RuntimeCallContext;

/// Enable or disable non-blocking mode.
pub(crate) unsafe fn destack_net_set_nonblocking(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let socket = socket_descriptor(_context, handle)?;
    let mut value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe { ioctlsocket(socket, FIONBIO, &mut value) };
    if rc != 0 {
        return Err(last_net_error("ioctlsocket"));
    }
    Ok(())
}

/// Enable or disable TCP_NODELAY.
pub(crate) unsafe fn destack_net_set_no_delay(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let socket = socket_descriptor(_context, handle)?;
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_TCP,
            TCP_NODELAY,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Enable or disable TCP keepalive.
pub(crate) unsafe fn destack_net_set_keep_alive(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
    delay_seconds: u32,
) -> RuntimeResult<()> {
    let socket = socket_descriptor(_context, handle)?;
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_KEEPALIVE,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }

    if enabled {
        let mut settings = tcp_keepalive {
            onoff: 1,
            keepalivetime: delay_seconds * 1000,
            keepaliveinterval: 1000,
        };
        let mut returned = 0u32;
        let rc = unsafe {
            WSAIoctl(
                socket,
                SIO_KEEPALIVE_VALS,
                &mut settings as *mut _ as *mut _,
                mem::size_of::<tcp_keepalive>() as u32,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
                None,
            )
        };
        if rc != 0 {
            return Err(last_net_error("WSAIoctl"));
        }
    }

    Ok(())
}

/// Enable or disable SO_REUSEADDR.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let socket = socket_descriptor(_context, handle)?;
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_REUSEADDR,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Enable or disable SO_REUSEPORT.
pub(crate) unsafe fn destack_net_set_reuse_port(
    _context: &RuntimeCallContext,
    _handle: SocketHandle,
    _enabled: bool,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReusePort")).boxed())
}
