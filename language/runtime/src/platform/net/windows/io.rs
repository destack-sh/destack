#![allow(dead_code)]

use std::mem;
use windows_sys::Win32::Networking::WinSock::{
    LPFN_WSARECVMSG, MSG_CTRUNC, MSG_TRUNC, SIO_GET_EXTENSION_FUNCTION_POINTER, SOCKET_ERROR,
    WSABUF, WSAEMSGSIZE, WSAID_WSARECVMSG, WSAIoctl, WSAMSG, WSASendMsg, recv, send,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{
    SocketAddress, SocketControlBufferAbi, SocketCredentials, SocketHandle, SocketMessageFlags,
    SocketRecvMessage, SocketSendMessage,
};
use crate::platform::resource::TransferredHandle;
use crate::platform::{NativeArray, NativeSlice, PlatformError, core as core_platform};
use crate::runtime::RuntimeCallContext;

/// Resolve the `WSARecvMsg` extension pointer for a socket.
fn receive_message_extension(socket: usize) -> RuntimeResult<LPFN_WSARECVMSG> {
    // allocate extension query payload
    let mut receive_message: LPFN_WSARECVMSG = None;
    let mut bytes_returned = 0u32;

    // resolve `WSARecvMsg` through winsock extension dispatch
    let rc = unsafe {
        WSAIoctl(
            socket,
            SIO_GET_EXTENSION_FUNCTION_POINTER,
            &WSAID_WSARECVMSG as *const _ as *mut _,
            mem::size_of_val(&WSAID_WSARECVMSG) as u32,
            &mut receive_message as *mut _ as *mut _,
            mem::size_of::<LPFN_WSARECVMSG>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
            None,
        )
    };
    if rc != 0 {
        return Err(last_net_error(
            "WSAIoctl(SIO_GET_EXTENSION_FUNCTION_POINTER)",
        ));
    }

    // ensure the extension pointer is available
    if receive_message.is_none() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.net.recvMsg")).boxed(),
        );
    }

    Ok(receive_message)
}

/// Read from a socket into a buffer.
pub(crate) unsafe fn destack_net_read(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // decode and validate the buffer
    let buffer = unsafe { buffer.as_mut_slice()? };
    let buffer_len = i32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;

    // read from the socket
    let rc = unsafe { recv(socket, buffer.as_mut_ptr() as *mut _, buffer_len, 0) };
    if rc < 0 {
        return Err(last_net_error("recv"));
    }

    // write the output
    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Write to a socket from a buffer.
pub(crate) unsafe fn destack_net_write(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // decode and validate the buffer
    let buffer = unsafe { buffer.as_slice()? };
    let buffer_len = i32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;

    // write to the socket
    let rc = unsafe { send(socket, buffer.as_ptr() as *const _, buffer_len, 0) };
    if rc < 0 {
        return Err(last_net_error("send"));
    }

    // write the output
    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Read from a socket into multiple buffers.
pub(crate) unsafe fn destack_net_readv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the buffer list
    let buffers = unsafe { buffers.as_slice()? };
    let mut total = 0u64;
    for buffer in buffers {
        let mut local = 0u64;
        unsafe { destack_net_read(context, &mut local, handle, *buffer) }?;
        total = total.saturating_add(local);
        if local < buffer.len as u64 {
            break;
        }
    }

    unsafe {
        *out = total;
    }

    Ok(())
}

/// Write to a socket from multiple buffers.
pub(crate) unsafe fn destack_net_writev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the buffer list
    let buffers = unsafe { buffers.as_slice()? };
    let mut total = 0u64;
    for buffer in buffers {
        let mut local = 0u64;
        unsafe { destack_net_write(context, &mut local, handle, *buffer) }?;
        total = total.saturating_add(local);
        if local < buffer.len as u64 {
            break;
        }
    }

    unsafe {
        *out = total;
    }

    Ok(())
}

/// Receive a message with ancillary data.
pub(crate) unsafe fn destack_net_recv_msg(
    context: &RuntimeCallContext,
    out: *mut SocketRecvMessage,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recv_flags: SocketMessageFlags,
    max_fds: u32,
    want_credentials: bool,
    max_control_bytes: u32,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject unsupported ancillary receive features
    if max_fds > 0 || want_credentials {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.net.recvMsg")).boxed(),
        );
    }

    // reserve explicit control size support for future winsock paths
    let _ = max_control_bytes;

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // decode and validate the buffer
    let buffer = unsafe { buffer.as_mut_slice()? };
    let buffer_len = u32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;

    // resolve the recvmsg extension entrypoint
    let receive_message = receive_message_extension(socket)?;
    let receive_message = receive_message.unwrap();

    // prepare the winsock message payload
    let mut data = WSABUF {
        len: buffer_len,
        buf: buffer.as_mut_ptr(),
    };
    let mut message = WSAMSG {
        name: std::ptr::null_mut(),
        namelen: 0,
        lpBuffers: &mut data,
        dwBufferCount: 1,
        Control: WSABUF {
            len: 0,
            buf: std::ptr::null_mut(),
        },
        dwFlags: recv_flags.0,
    };
    let mut bytes_received = 0u32;

    // receive payload bytes and map truncation behavior
    let rc = unsafe {
        receive_message(
            socket,
            &mut message,
            &mut bytes_received,
            std::ptr::null_mut(),
            None,
        )
    };
    let mut payload_truncated = false;
    if rc == SOCKET_ERROR {
        let error_code = core_platform::last_wsa_error_code();
        if error_code == WSAEMSGSIZE {
            payload_truncated = true;
        } else {
            return Err(last_net_error("WSARecvMsg"));
        }
    }

    // build an empty ancillary payload
    let address = SocketAddress {
        family: 0,
        length: 0,
        bytes: context.store_array(Vec::new()),
    };
    let control = context.store_array(Vec::new());
    let fds: Vec<TransferredHandle> = Vec::new();
    let fds = context.store_array(fds);

    // map receive flags to truncation metadata
    let recv_flags = SocketMessageFlags(message.dwFlags);
    if (recv_flags.0 & MSG_TRUNC) != 0 {
        payload_truncated = true;
    }
    let control_truncated = (recv_flags.0 & MSG_CTRUNC) != 0;

    // write the output payload
    unsafe {
        *out = SocketRecvMessage {
            bytes: bytes_received as u64,
            has_address: false,
            address,
            recv_flags,
            payload_truncated,
            control_truncated,
            control: SocketControlBufferAbi(control),
            fds,
            has_credentials: false,
            credentials: SocketCredentials {
                pid: 0,
                uid: 0,
                gid: 0,
            },
        };
    }

    Ok(())
}

/// Receive multiple messages into multiple buffers.
pub(crate) unsafe fn destack_net_recv_mmsg(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u64>,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the buffer list
    let buffers = unsafe { buffers.as_slice()? };
    let mut counts = Vec::with_capacity(buffers.len());

    // receive into each buffer in sequence
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        let mut bytes = 0u64;
        unsafe { destack_net_read(context, &mut bytes, handle, *buffer) }?;

        // stop on orderly shutdown
        if bytes == 0 {
            break;
        }

        // cap to the target buffer length for safety
        let capped = bytes.min(slice.len() as u64);
        counts.push(capped);

        // stop when the read does not fill the buffer
        if capped < slice.len() as u64 {
            break;
        }
    }

    // write the output array
    unsafe {
        *out = context.store_array(counts);
    }

    // preserve the explicit flags parameter for future Winsock support
    let _ = recv_flags;

    Ok(())
}

/// Send a message with ancillary data.
pub(crate) unsafe fn destack_net_send_msg(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendMessage,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject unsupported ancillary send features
    let fds = unsafe { message.fds.as_slice()? };
    if !fds.is_empty() || message.has_credentials {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.net.sendMsg")).boxed(),
        );
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // decode and validate the buffer
    let buffer = unsafe { buffer.as_slice()? };
    let buffer_len = u32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;

    // prepare the winsock message payload
    let mut data = WSABUF {
        len: buffer_len,
        buf: buffer.as_ptr() as *mut _,
    };
    let send_message = WSAMSG {
        name: std::ptr::null_mut(),
        namelen: 0,
        lpBuffers: &mut data,
        dwBufferCount: 1,
        Control: WSABUF {
            len: 0,
            buf: std::ptr::null_mut(),
        },
        dwFlags: 0,
    };
    let mut bytes_sent = 0u32;

    // send payload bytes
    let rc = unsafe {
        WSASendMsg(
            socket,
            &send_message,
            message.flags.0,
            &mut bytes_sent,
            std::ptr::null_mut(),
            None,
        )
    };
    if rc == SOCKET_ERROR {
        return Err(last_net_error("WSASendMsg"));
    }

    // write the output count
    unsafe {
        *out = bytes_sent as u64;
    }

    Ok(())
}

/// Send multiple messages from multiple buffers.
pub(crate) unsafe fn destack_net_send_mmsg(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    send_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the buffer list
    let buffers = unsafe { buffers.as_slice()? };
    let mut sent_count = 0u64;

    // send each buffer in sequence
    for buffer in buffers {
        let mut bytes = 0u64;
        unsafe { destack_net_write(context, &mut bytes, handle, *buffer) }?;
        sent_count = sent_count.saturating_add(1);
    }

    // write the output count
    unsafe {
        *out = sent_count;
    }

    // preserve the explicit flags parameter for future Winsock support
    let _ = send_flags;

    Ok(())
}
