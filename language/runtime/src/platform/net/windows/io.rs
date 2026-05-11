#![allow(dead_code)]

use std::mem;
use windows_sys::Win32::Networking::WinSock::{
    LPFN_WSARECVMSG, MSG_CTRUNC, MSG_TRUNC, SIO_GET_EXTENSION_FUNCTION_POINTER, SOCKADDR,
    SOCKADDR_STORAGE, SOCKET_ERROR, WSABUF, WSAEINVAL, WSAEMSGSIZE, WSAENOPROTOOPT, WSAEOPNOTSUPP,
    WSAID_WSARECVMSG, WSAIoctl, WSAMSG, WSASendMsg, recv, recvfrom, send, sendto,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{
    SocketControlBufferAbi, SocketHandle, SocketMessageFlags, SocketRecvFrom, SocketRecvMessage,
    SocketSendMessage, SocketSendTo,
};
use crate::platform::resource::TransferredHandle;
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

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
        let error_code = core_platform::last_wsa_error_code();

        // report unavailable extension providers as non-fatal fallback conditions
        if error_code == WSAEINVAL || error_code == WSAEOPNOTSUPP || error_code == WSAENOPROTOOPT {
            return Ok(None);
        }

        return Err(core_platform::net_error_with_code(
            "WSAIoctl(SIO_GET_EXTENSION_FUNCTION_POINTER)",
            error_code,
        ));
    }

    // ensure the extension pointer is available
    if receive_message.is_none() {
        return Ok(None);
    }

    Ok(receive_message)
}

/// Cast one socket-flag bitfield to WinSock i32 flags.
fn socket_flags_i32(flags: u32, label: &str) -> RuntimeResult<i32> {
    i32::try_from(flags).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "flags out of range for winsock",
        ))
        .boxed()
    })
}

/// Return whether one Winsock status reports unavailable message extensions.
fn is_message_extension_not_supported(code: i32) -> bool {
    code == WSAEINVAL || code == WSAEOPNOTSUPP || code == WSAENOPROTOOPT
}

/// Read from a socket into the provided slice.
pub(crate) unsafe fn destack_net_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "recv",
            core_platform::last_wsa_error_code(),
        ));
    }

    // write the output
    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Write to a socket from the provided slice.
pub(crate) unsafe fn destack_net_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "send",
            core_platform::last_wsa_error_code(),
        ));
    }

    // write the output
    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Read into multiple buffers.
pub(crate) unsafe fn destack_net_readv(
    binding: &BindingCallContext,
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
        unsafe { destack_net_read(binding, &mut local, handle, *buffer) }?;
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

/// Write from multiple buffers.
pub(crate) unsafe fn destack_net_writev(
    binding: &BindingCallContext,
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
        unsafe { destack_net_write(binding, &mut local, handle, *buffer) }?;
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
    binding: &BindingCallContext,
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

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // decode and validate the buffer
    let buffer = unsafe { buffer.as_mut_slice()? };
    let buffer_len = u32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;
    let socket_buffer_length = i32::try_from(buffer_len).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large for winsock",
        ))
        .boxed()
    })?;

    // resolve the recvmsg extension entrypoint when available
    let receive_message = receive_message_extension(socket)?;

    // fall back to recvfrom when winsock does not expose recvmsg support
    if receive_message.is_none() {
        let mut address = unsafe { std::mem::zeroed::<SOCKADDR_STORAGE>() };
        let mut address_length = std::mem::size_of::<SOCKADDR_STORAGE>() as i32;
        let recv_flags_i32 = socket_flags_i32(recv_flags.0, "recvFlags")?;

        let rc = unsafe {
            recvfrom(
                socket,
                buffer.as_mut_ptr() as *mut _,
                socket_buffer_length,
                recv_flags_i32,
                &mut address as *mut _ as *mut SOCKADDR,
                &mut address_length,
            )
        };

        let (bytes_received, payload_truncated) = if rc >= 0 {
            (rc as u64, false)
        } else {
            let error_code = core_platform::last_wsa_error_code();
            if error_code == WSAEMSGSIZE {
                (buffer_len as u64, true)
            } else {
                return Err(core_platform::net_error_with_code(
                    "recvfrom",
                    core_platform::last_wsa_error_code(),
                ));
            }
        };

        let address = if address_length > 0 {
            Some(socket_address_raw_from_storage(
                binding,
                &address,
                address_length,
            )?)
        } else {
            None
        };
        let recv_flags_value = if payload_truncated {
            SocketMessageFlags(recv_flags.0 | MSG_TRUNC)
        } else {
            recv_flags
        };
        let empty_control = binding.store_array(Vec::<u8>::new());
        let empty_fds = binding.store_array(Vec::<TransferredHandle>::new());
        unsafe {
            *out = SocketRecvMessage {
                bytes: bytes_received,
                address,
                recv_flags: recv_flags_value,
                payload_truncated,
                control_truncated: false,
                control: SocketControlBufferAbi(empty_control),
                fds: empty_fds,
                credentials: None,
            };
        }

        return Ok(());
    }

    // reject ancillary-only calls when recvmsg extension is unavailable
    let receive_message = receive_message.ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported("destack.net.recvMsg")).boxed()
    })?;

    // allocate source-address and control storage
    let mut address = unsafe { std::mem::zeroed::<SOCKADDR_STORAGE>() };
    let mut control = vec![0u8; max_control_bytes as usize];
    let control_len = u32::try_from(control.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "maxControlBytes",
            "control buffer too large",
        ))
        .boxed()
    })?;

    // prepare the winsock message payload
    let mut data = WSABUF {
        len: buffer_len,
        buf: buffer.as_mut_ptr(),
    };
    let mut message = WSAMSG {
        name: &mut address as *mut _ as *mut SOCKADDR,
        namelen: std::mem::size_of::<SOCKADDR_STORAGE>() as i32,
        lpBuffers: &mut data,
        dwBufferCount: 1,
        Control: WSABUF {
            len: control_len,
            buf: if control.is_empty() {
                std::ptr::null_mut()
            } else {
                control.as_mut_ptr()
            },
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
            return Err(core_platform::net_error_with_code(
                "WSARecvMsg",
                core_platform::last_wsa_error_code(),
            ));
        }
    }

    // decode source-address payload
    let address = if message.namelen > 0 {
        Some(socket_address_raw_from_storage(
            binding,
            &address,
            message.namelen,
        )?)
    } else {
        None
    };

    // decode raw ancillary payload
    let control_len = (message.Control.len as usize).min(control.len());
    let control = binding.store_array_copy(&control[..control_len]);
    let fds: Vec<TransferredHandle> = Vec::new();
    let fds = binding.store_array(fds);

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
            address,
            recv_flags,
            payload_truncated,
            control_truncated,
            control: SocketControlBufferAbi(control),
            fds,
            credentials: None,
        };
    }

    Ok(())
}

/// Receive multiple datagrams.
pub(crate) unsafe fn destack_net_recv_mmsg(
    binding: &BindingCallContext,
    out: *mut NativeArray<u64>,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    _recv_flags: SocketMessageFlags,
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
        unsafe { destack_net_read(binding, &mut bytes, handle, *buffer) }?;

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
        *out = binding.store_array(counts);
    }

    // preserve the explicit flags parameter for future Winsock support
    Ok(())
}

/// Send a message with ancillary data.
pub(crate) unsafe fn destack_net_send_msg(
    binding: &BindingCallContext,
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
    if !fds.is_empty() || message.credentials.is_some() {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.net.sendMsg")).boxed(),
        );
    }

    // decode raw control bytes
    let control = unsafe { message.control.0.as_slice()? };
    let control_len = u32::try_from(control.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "message.control",
            "control buffer too large",
        ))
        .boxed()
    })?;

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // decode and validate the buffer
    let buffer = unsafe { buffer.as_slice()? };
    let buffer_len = u32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;
    let socket_buffer_length = i32::try_from(buffer_len).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large for winsock",
        ))
        .boxed()
    })?;

    // use send or sendto when no ancillary control payload is present
    if control.is_empty() {
        let flags = socket_flags_i32(message.flags.0, "message.flags")?;
        let bytes_sent = if let Some(address) = message.address {
            with_socket_address_raw(address, |sockaddr, length| {
                let rc = unsafe {
                    sendto(
                        socket,
                        buffer.as_ptr() as *const _,
                        socket_buffer_length,
                        flags,
                        sockaddr,
                        length,
                    )
                };
                if rc == SOCKET_ERROR {
                    return Err(core_platform::net_error_with_code(
                        "sendto",
                        core_platform::last_wsa_error_code(),
                    ));
                }

                Ok(rc as u64)
            })?
        } else {
            let rc = unsafe {
                send(
                    socket,
                    buffer.as_ptr() as *const _,
                    socket_buffer_length,
                    flags,
                )
            };
            if rc == SOCKET_ERROR {
                return Err(core_platform::net_error_with_code(
                    "send",
                    core_platform::last_wsa_error_code(),
                ));
            }
            rc as u64
        };

        unsafe {
            *out = bytes_sent;
        }

        return Ok(());
    }

    // prepare the winsock message payload
    let mut data = WSABUF {
        len: buffer_len,
        buf: buffer.as_ptr() as *mut _,
    };
    let mut send_message = WSAMSG {
        name: std::ptr::null_mut(),
        namelen: 0,
        lpBuffers: &mut data,
        dwBufferCount: 1,
        Control: WSABUF {
            len: control_len,
            buf: if control.is_empty() {
                std::ptr::null_mut()
            } else {
                control.as_ptr() as *mut u8
            },
        },
        dwFlags: 0,
    };
    let mut bytes_sent = 0u32;

    // send payload bytes with optional explicit destination
    if let Some(address) = message.address {
        with_socket_address_raw(address, |sockaddr, length| {
            send_message.name = sockaddr as *mut SOCKADDR;
            send_message.namelen = length;

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
                let error_code = core_platform::last_wsa_error_code();
                if is_message_extension_not_supported(error_code) {
                    return Err(RuntimeError::from(PlatformError::not_supported(
                        "destack.net.sendMsg",
                    ))
                    .boxed());
                }

                return Err(core_platform::net_error_with_code("WSASendMsg", error_code));
            }

            Ok(())
        })?;
    } else {
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
            let error_code = core_platform::last_wsa_error_code();
            if is_message_extension_not_supported(error_code) {
                return Err(RuntimeError::from(PlatformError::not_supported(
                    "destack.net.sendMsg",
                ))
                .boxed());
            }

            return Err(core_platform::net_error_with_code("WSASendMsg", error_code));
        }
    }

    // write the output count
    unsafe {
        *out = bytes_sent as u64;
    }

    Ok(())
}

/// Send multiple datagrams.
pub(crate) unsafe fn destack_net_send_mmsg(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    _send_flags: SocketMessageFlags,
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
        unsafe { destack_net_write(binding, &mut bytes, handle, *buffer) }?;
        sent_count = sent_count.saturating_add(1);
    }

    // write the output count
    unsafe {
        *out = sent_count;
    }

    // preserve the explicit flags parameter for future Winsock support
    Ok(())
}

/// Receive a packet from a remote socket address.
pub(crate) unsafe fn destack_net_recv_from(
    binding: &BindingCallContext,
    out: *mut SocketRecvFrom,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reuse recvmsg so recvFlags and truncation metadata stay honest
    let mut message = std::mem::MaybeUninit::<SocketRecvMessage>::uninit();
    unsafe {
        destack_net_recv_msg(
            binding,
            message.as_mut_ptr(),
            handle,
            buffer,
            recv_flags,
            0,
            false,
            0,
        )
    }?;
    let message = unsafe { message.assume_init() };
    let address = message.address.ok_or_else(|| {
        core_platform::io_operation_error(
            "destack.net.recvFrom",
            Some(PlatformErrorCode::IoInvalidData),
            "recvmsg did not report a source address",
        )
    })?;

    // write the recvfrom projection
    unsafe {
        *out = SocketRecvFrom {
            bytes: message.bytes,
            address,
            recv_flags: message.recv_flags,
        };
    }

    Ok(())
}

/// Send a packet to a remote socket address.
pub(crate) unsafe fn destack_net_send_to(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendTo,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // resolve runtime values
    let socket = socket_descriptor(binding, handle)?;
    let buffer = unsafe { buffer.as_slice()? };
    let buffer_len = i32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;

    // send one datagram
    with_socket_address_raw(message.address, |sockaddr, length| {
        let bytes = unsafe {
            sendto(
                socket,
                buffer.as_ptr() as *const _,
                buffer_len,
                message.flags.0 as i32,
                sockaddr,
                length,
            )
        };
        if bytes == SOCKET_ERROR {
            return Err(core_platform::net_error_with_code(
                "sendto",
                core_platform::last_wsa_error_code(),
            ));
        }

        unsafe {
            *out = bytes as u64;
        }

        Ok(())
    })
}
