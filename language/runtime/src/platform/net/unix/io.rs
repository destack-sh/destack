use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::net::*;
use crate::platform::resource::{ResourceEntry, TransferredHandle};
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::os::unix::io::RawFd;

/// Read from a socket into the provided slice.
pub(crate) unsafe fn destack_net_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read on unix platforms
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

    // decode the buffer
    let buffer = unsafe { buffer.as_mut_slice()? };

    // read from the socket
    let rc = unsafe {
        libc::recv(
            fd,
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
            0,
        )
    };
    if rc < 0 {
        return Err(core_platform::net_error("recv"));
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
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // write on unix platforms
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

    // decode the buffer
    let buffer = unsafe { buffer.as_slice()? };

    // write to the socket
    let flags = os::send_flags();
    let rc = unsafe {
        libc::send(
            fd,
            buffer.as_ptr() as *const libc::c_void,
            buffer.len(),
            flags,
        )
    };
    if rc < 0 {
        return Err(core_platform::net_error("send"));
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

    // resolve the socket descriptor and buffer list
    let fd = socket_descriptor(binding, handle)?;
    let buffers = unsafe { buffers.as_slice()? };
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_mut_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }

    // read from the socket
    let rc = unsafe { libc::readv(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::net_error("readv"));
    }

    unsafe {
        *out = rc as u64;
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

    // resolve the socket descriptor and buffer list
    let fd = socket_descriptor(binding, handle)?;
    let buffers = unsafe { buffers.as_slice()? };
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }

    // write to the socket
    let rc = unsafe { libc::writev(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::net_error("writev"));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Receive multiple datagrams.
#[allow(dead_code)]
pub(crate) unsafe fn destack_net_recv_mmsg(
    binding: &BindingCallContext,
    out: *mut NativeArray<u64>,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate receive flags
    if recv_flags.0 > i32::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "recvFlags",
            "flags value is out of range",
        ))
        .boxed());
    }

    // resolve socket and buffers
    let fd = socket_descriptor(binding, handle)?;
    let buffers = unsafe { buffers.as_slice()? };
    let mut counts = Vec::with_capacity(buffers.len());

    // receive into each buffer
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        let rc = unsafe {
            libc::recv(
                fd,
                slice.as_mut_ptr() as *mut libc::c_void,
                slice.len(),
                recv_flags.0 as libc::c_int,
            )
        };

        // stop on would-block once at least one packet is received
        if rc < 0 {
            let errno = core_platform::get_errno();
            if !counts.is_empty() && (errno == libc::EAGAIN || errno == libc::EWOULDBLOCK) {
                break;
            }

            return Err(core_platform::net_error("recv"));
        }

        // stop on orderly shutdown
        if rc == 0 {
            break;
        }

        counts.push(rc as u64);
    }

    // write the output array
    unsafe {
        *out = binding.store_array(counts);
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

    // resolve the socket descriptor and buffer
    let fd = socket_descriptor(binding, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };

    // build the iovec
    let mut iovec = libc::iovec {
        iov_base: buffer.as_mut_ptr() as *mut libc::c_void,
        iov_len: buffer.len(),
    };

    // reserve source-address storage for recvmsg
    let mut address_storage = std::mem::MaybeUninit::<libc::sockaddr_storage>::zeroed();

    // compute control buffer length
    let mut control_len = 0usize;
    if max_fds > 0 {
        let fd_bytes = (max_fds as usize)
            .checked_mul(std::mem::size_of::<RawFd>())
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "maxFds",
                    "fd count is too large",
                ))
                .boxed()
            })?;
        if fd_bytes > u32::MAX as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "maxFds",
                "fd count is too large",
            ))
            .boxed());
        }
        control_len =
            control_len.saturating_add(unsafe { libc::CMSG_SPACE(fd_bytes as u32) } as usize);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if want_credentials {
        control_len = control_len.saturating_add(unsafe {
            libc::CMSG_SPACE(std::mem::size_of::<libc::ucred>() as u32)
        } as usize);
    }

    // apply caller control-budget semantics
    if max_control_bytes > 0 {
        let budget = max_control_bytes as usize;
        if control_len == 0 {
            control_len = budget;
        } else {
            control_len = control_len.min(budget);
        }
    }

    // allocate the control buffer
    let mut control = vec![0u8; control_len];

    // build the message header
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_name = address_storage.as_mut_ptr() as *mut libc::c_void;
    message.msg_namelen = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    message.msg_iov = &mut iovec;
    message.msg_iovlen = 1;
    if !control.is_empty() {
        let control_len = control.len();
        if control_len > u32::MAX as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "buffer",
                "control buffer too large",
            ))
            .boxed());
        }
        message.msg_control = control.as_mut_ptr() as *mut libc::c_void;
        message.msg_controllen = control_len as _;
    }

    // validate receive flags
    if recv_flags.0 > i32::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "recvFlags",
            "flags value is out of range",
        ))
        .boxed());
    }

    // receive the message
    let rc = unsafe { libc::recvmsg(fd, &mut message, recv_flags.0 as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::net_error("recvmsg"));
    }

    // decode recvmsg flags
    let recv_flags = SocketMessageFlags(message.msg_flags as u32);
    let payload_truncated = (message.msg_flags & libc::MSG_TRUNC) != 0;
    let control_truncated = (message.msg_flags & libc::MSG_CTRUNC) != 0;

    // decode ancillary data
    let mut received_fds: Vec<RawFd> = Vec::new();
    let mut credentials: Option<SocketCredentials> = None;
    let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(&message) };
    while !cmsg.is_null() {
        let header = unsafe { &*cmsg };
        if header.cmsg_level == libc::SOL_SOCKET && header.cmsg_type == libc::SCM_RIGHTS {
            let data_len = header.cmsg_len as usize - unsafe { libc::CMSG_LEN(0) as usize };
            let count = data_len / std::mem::size_of::<RawFd>();
            let data = unsafe { libc::CMSG_DATA(cmsg) as *const RawFd };
            for index in 0..count {
                received_fds.push(unsafe { *data.add(index) });
            }
        }
        if header.cmsg_level == libc::SOL_SOCKET {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            if header.cmsg_type == libc::SCM_CREDENTIALS {
                let data = unsafe { libc::CMSG_DATA(cmsg) as *const libc::ucred };
                let ucred = unsafe { *data };
                if want_credentials {
                    credentials = Some(SocketCredentials {
                        pid: ucred.pid as u32,
                        uid: ucred.uid,
                        gid: ucred.gid,
                    });
                }
            }
        }
        cmsg = unsafe { libc::CMSG_NXTHDR(&message, cmsg) };
    }

    // register received file descriptors
    let mut handles = Vec::with_capacity(received_fds.len().min(max_fds as usize));
    for (index, descriptor) in received_fds.into_iter().enumerate() {
        if index >= max_fds as usize {
            unsafe {
                libc::close(descriptor);
            }
            continue;
        }
        let kind = kind_from_received_descriptor(descriptor)?;
        let entry = ResourceEntry::new(kind)
            .with_fd(descriptor)
            .with_finalizer(FileFinalizer { fd: descriptor });
        let id = binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
        handles.push(TransferredHandle(id));
    }

    // resolve missing credentials through peer socket metadata
    if want_credentials && credentials.is_none() {
        credentials = Some(peer_socket_credentials(fd)?);
    }

    // decode source address payload
    let address = if message.msg_namelen > 0 {
        let storage = unsafe { address_storage.assume_init() };
        Some(socket_address_raw_from_storage(
            binding,
            &storage,
            message.msg_namelen,
        )?)
    } else {
        None
    };

    // decode raw control payload bytes
    let control_len =
        core_platform::u64_to_usize(message.msg_controllen as u64, "message.msg_controllen")?;
    let control_len = control_len.min(control.len());
    let control_bytes = if control_len == 0 {
        Vec::new()
    } else {
        control[..control_len].to_vec()
    };
    let control = binding.store_array(control_bytes);

    // build the response payload
    let fds = binding.store_array(handles);
    unsafe {
        *out = SocketRecvMessage {
            bytes: rc as u64,
            address,
            recv_flags,
            payload_truncated,
            control_truncated,
            control: SocketControlBufferAbi(control),
            fds,
            credentials,
        };
    }

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

    // resolve the socket descriptor and buffer
    let fd = socket_descriptor(binding, handle)?;
    let buffer = unsafe { buffer.as_slice()? };

    // resolve file descriptors to send
    let fds = unsafe { message.fds.as_slice()? };
    let mut raw_fds = Vec::with_capacity(fds.len());
    for handle in fds {
        raw_fds.push(transferable_descriptor(binding, *handle)?);
    }

    // decode raw control bytes
    let raw_control = unsafe { message.control.0.as_slice()? };
    if !raw_control.is_empty() && (!raw_fds.is_empty() || message.credentials.is_some()) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "message.control",
            "raw control cannot be combined with fds or explicit credentials",
        ))
        .boxed());
    }

    // validate credentials support
    if message.credentials.is_some() {
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            return Err(
                RuntimeError::from(PlatformError::not_supported("destack.net.sendMsg")).boxed(),
            );
        }
    }

    // validate send flags before dispatch
    if message.flags.0 > i32::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "message.flags",
            "flags value is out of range",
        ))
        .boxed());
    }

    // build the iovec
    let mut iovec = libc::iovec {
        iov_base: buffer.as_ptr() as *mut libc::c_void,
        iov_len: buffer.len(),
    };

    // compute typed control buffer length
    let mut typed_control_len = 0usize;
    if !raw_control.is_empty() {
        typed_control_len = raw_control.len();
    } else if !raw_fds.is_empty() {
        let fd_bytes = raw_fds
            .len()
            .checked_mul(std::mem::size_of::<RawFd>())
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "message.fds",
                    "fd count is too large",
                ))
                .boxed()
            })?;
        if fd_bytes > u32::MAX as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "message.fds",
                "fd count is too large",
            ))
            .boxed());
        }
        typed_control_len =
            typed_control_len.saturating_add(unsafe { libc::CMSG_SPACE(fd_bytes as u32) } as usize);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if raw_control.is_empty() && message.credentials.is_some() {
        typed_control_len = typed_control_len.saturating_add(unsafe {
            libc::CMSG_SPACE(std::mem::size_of::<libc::ucred>() as u32)
        } as usize);
    }

    // allocate the control buffer
    let mut control = vec![0u8; typed_control_len];
    if !raw_control.is_empty() {
        control.copy_from_slice(raw_control);
    }

    // build the message header
    let mut hdr: libc::msghdr = unsafe { std::mem::zeroed() };
    hdr.msg_iov = &mut iovec;
    hdr.msg_iovlen = 1;
    if !control.is_empty() {
        let control_len = control.len();
        if control_len > u32::MAX as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "buffer",
                "control buffer too large",
            ))
            .boxed());
        }
        hdr.msg_control = control.as_mut_ptr() as *mut libc::c_void;
        hdr.msg_controllen = control_len as _;
    }

    // fill typed control messages when raw control is absent
    if raw_control.is_empty() {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(&hdr) };
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        let cmsg = unsafe { libc::CMSG_FIRSTHDR(&hdr) };
        if !raw_fds.is_empty() {
            let header = unsafe { &mut *cmsg };
            header.cmsg_level = libc::SOL_SOCKET;
            header.cmsg_type = libc::SCM_RIGHTS;
            let cmsg_len =
                unsafe { libc::CMSG_LEN((raw_fds.len() * std::mem::size_of::<RawFd>()) as u32) }
                    as usize;
            header.cmsg_len = cmsg_len as _;
            let data = unsafe { libc::CMSG_DATA(cmsg) as *mut RawFd };
            unsafe {
                std::ptr::copy_nonoverlapping(raw_fds.as_ptr(), data, raw_fds.len());
            }
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                cmsg = unsafe { libc::CMSG_NXTHDR(&hdr, cmsg) };
            }
            #[cfg(not(any(target_os = "linux", target_os = "android")))]
            {
                let _ = cmsg;
            }
        }

        #[cfg(any(target_os = "linux", target_os = "android"))]
        if let Some(credentials) = message.credentials {
            if cmsg.is_null() {
                return Err(
                    RuntimeError::from(PlatformError::io("missing control buffer")).boxed(),
                );
            }
            let header = unsafe { &mut *cmsg };
            header.cmsg_level = libc::SOL_SOCKET;
            header.cmsg_type = libc::SCM_CREDENTIALS;
            let cmsg_len =
                unsafe { libc::CMSG_LEN(std::mem::size_of::<libc::ucred>() as u32) } as usize;
            header.cmsg_len = cmsg_len as _;
            let data = unsafe { libc::CMSG_DATA(cmsg) as *mut libc::ucred };
            unsafe {
                *data = libc::ucred {
                    pid: credentials.pid as libc::pid_t,
                    uid: credentials.uid,
                    gid: credentials.gid,
                };
            }
        }
    }

    // send the message with optional explicit address routing
    let rc = if let Some(address) = message.address {
        with_socket_address_raw(address, |sockaddr, length| {
            hdr.msg_name = sockaddr as *mut libc::c_void;
            hdr.msg_namelen = length;

            let rc = unsafe { libc::sendmsg(fd, &hdr, message.flags.0 as libc::c_int) };
            if rc < 0 {
                return Err(core_platform::net_error("sendmsg"));
            }

            Ok(rc)
        })?
    } else {
        let rc = unsafe { libc::sendmsg(fd, &hdr, message.flags.0 as libc::c_int) };
        if rc < 0 {
            return Err(core_platform::net_error("sendmsg"));
        }

        rc
    };

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Send multiple datagrams.
#[allow(dead_code)]
pub(crate) unsafe fn destack_net_send_mmsg(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    send_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate send flags
    if send_flags.0 > i32::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "sendFlags",
            "flags value is out of range",
        ))
        .boxed());
    }

    // resolve socket and buffers
    let fd = socket_descriptor(binding, handle)?;
    let buffers = unsafe { buffers.as_slice()? };
    let mut sent_count = 0u64;

    // send each buffer as an individual message
    for buffer in buffers {
        let slice = unsafe { buffer.as_slice()? };
        let rc = unsafe {
            libc::send(
                fd,
                slice.as_ptr() as *const libc::c_void,
                slice.len(),
                send_flags.0 as libc::c_int,
            )
        };

        // stop after partial progress on transient send errors
        if rc < 0 {
            let errno = core_platform::get_errno();
            if sent_count > 0 && (errno == libc::EAGAIN || errno == libc::EWOULDBLOCK) {
                break;
            }

            return Err(core_platform::net_error("send"));
        }

        sent_count = sent_count.saturating_add(1);
    }

    // write the output count
    unsafe {
        *out = sent_count;
    }

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

    // resolve runtime values
    let fd = socket_descriptor(binding, handle)?;
    let buffer = unsafe { buffer.as_slice()? };

    // send the datagram to the raw destination
    with_socket_address_raw(message.address, |sockaddr, length| {
        let bytes = unsafe {
            libc::sendto(
                fd,
                buffer.as_ptr() as *const libc::c_void,
                buffer.len(),
                message.flags.0 as libc::c_int,
                sockaddr,
                length,
            )
        };
        if bytes < 0 {
            return Err(core_platform::net_error("sendto"));
        }

        unsafe {
            *out = bytes as u64;
        }

        Ok(())
    })
}
