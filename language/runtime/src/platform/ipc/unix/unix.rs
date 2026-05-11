use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::ipc::{UnixPeerCredentials, UnixReceiveAncillary};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    io_error, register_transferred_descriptor, socket_descriptor, transferable_descriptor,
};

/// Unix ancillary receive operation.
const UNIX_RECEIVE_OPERATION: &str = "destack.ipc.unix.receive";
/// Unix ancillary send operation.
const UNIX_SEND_OPERATION: &str = "destack.ipc.unix.send";
/// Internal receive payload budget when the API does not expose bytes directly.
const RECEIVE_PAYLOAD_BUDGET: usize = 65_536;

/// Resolve one peer-credential payload from one unix socket descriptor.
fn peer_credentials(socket: libc::c_int) -> RuntimeResult<Option<UnixPeerCredentials>> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // query peer credentials through SO_PEERCRED
        let mut peer = std::mem::MaybeUninit::<libc::ucred>::uninit();
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let status = unsafe {
            libc::getsockopt(
                socket,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                peer.as_mut_ptr().cast::<libc::c_void>(),
                &mut length,
            )
        };
        if status != 0 {
            let errno = core_platform::get_errno();
            if errno == libc::ENOTCONN || errno == libc::EINVAL || errno == libc::ENOTSUP {
                return Ok(None);
            }

            return Err(io_error(
                UNIX_RECEIVE_OPERATION,
                "getsockopt",
                "failed to resolve unix peer credentials",
            ));
        }

        let peer = unsafe { peer.assume_init() };
        return Ok(Some(UnixPeerCredentials {
            pid: Some(peer.pid as u32),
            uid: peer.uid,
            gid: peer.gid,
        }));
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        // query peer credentials through getpeereid
        let mut uid: libc::uid_t = 0;
        let mut gid: libc::gid_t = 0;
        let status = unsafe { libc::getpeereid(socket, &mut uid, &mut gid) };
        if status != 0 {
            let errno = core_platform::get_errno();
            if errno == libc::ENOTCONN || errno == libc::EINVAL || errno == libc::ENOTSUP {
                return Ok(None);
            }

            return Err(io_error(
                UNIX_RECEIVE_OPERATION,
                "getpeereid",
                "failed to resolve unix peer credentials",
            ));
        }

        return Ok(Some(UnixPeerCredentials {
            pid: None,
            uid,
            gid,
        }));
    }

    #[allow(unreachable_code)]
    Ok(None)
}

/// Receive payload and transferred handles.
pub(crate) unsafe fn destack_ipc_unix_receive(
    binding: &BindingCallContext,
    out: *mut UnixReceiveAncillary,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve one unix socket descriptor
    let descriptor = socket_descriptor(binding, socket, UNIX_RECEIVE_OPERATION)?;

    // prepare one bounded payload receive buffer
    let mut payload = vec![0u8; RECEIVE_PAYLOAD_BUDGET];
    let mut iovec = libc::iovec {
        iov_base: payload.as_mut_ptr().cast::<libc::c_void>(),
        iov_len: payload.len(),
    };

    // allocate one ancillary control buffer for transferred descriptors
    let descriptor_capacity = usize::try_from(maxhandles).map_err(|_| {
        core_platform::invalid_argument("maxHandles", "maxHandles exceeds host usize range")
    })?;
    let descriptor_bytes = descriptor_capacity
        .checked_mul(std::mem::size_of::<libc::c_int>())
        .ok_or_else(|| {
            core_platform::invalid_argument("maxHandles", "maxHandles overflowed descriptor bytes")
        })?;
    if descriptor_bytes > u32::MAX as usize {
        return Err(core_platform::invalid_argument(
            "maxHandles",
            "maxHandles descriptor bytes exceed host cmsg range",
        ));
    }

    let control_length = if descriptor_bytes == 0 {
        0
    } else {
        unsafe { libc::CMSG_SPACE(descriptor_bytes as u32) as usize }
    };
    let mut control = vec![0u8; control_length];

    // build one recvmsg header for payload and descriptor control
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iovec;
    message.msg_iovlen = 1;
    if !control.is_empty() {
        message.msg_control = control.as_mut_ptr().cast::<libc::c_void>();
        message.msg_controllen = control.len() as _;
    }

    // receive one ancillary message payload
    let bytes = unsafe { libc::recvmsg(descriptor, &mut message, 0) };
    if bytes < 0 {
        return Err(io_error(
            UNIX_RECEIVE_OPERATION,
            "recvmsg",
            "failed to receive unix ancillary message",
        ));
    }

    // decode transferred descriptors from ancillary control messages
    let mut handles = Vec::new();
    let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(&message) };
    while !cmsg.is_null() {
        let header = unsafe { &*cmsg };
        if header.cmsg_level == libc::SOL_SOCKET && header.cmsg_type == libc::SCM_RIGHTS {
            let base_length = unsafe { libc::CMSG_LEN(0) as usize };
            if (header.cmsg_len as usize) < base_length {
                cmsg = unsafe { libc::CMSG_NXTHDR(&message, cmsg) };
                continue;
            }

            let data_length = header.cmsg_len as usize - base_length;
            let descriptor_count = data_length / std::mem::size_of::<libc::c_int>();
            let data = unsafe { libc::CMSG_DATA(cmsg).cast::<libc::c_int>() };

            for index in 0..descriptor_count {
                let received = unsafe { *data.add(index) };
                if handles.len() >= descriptor_capacity {
                    unsafe {
                        libc::close(received);
                    }
                    continue;
                }

                let transferred = register_transferred_descriptor(binding, received);
                handles.push(transferred);
            }
        }

        cmsg = unsafe { libc::CMSG_NXTHDR(&message, cmsg) };
    }

    // resolve peer credentials when available
    let credentials = peer_credentials(descriptor)?;

    // build and write one receive result payload
    unsafe {
        out.write(UnixReceiveAncillary {
            bytes: bytes as u64,
            handles: binding.store_array(handles),
            credentials,
        });
    }

    Ok(())
}

/// Send payload and transferred handles.
pub(crate) unsafe fn destack_ipc_unix_send(
    binding: &BindingCallContext,
    out: *mut u64,
    socket: resource::SocketHandle,
    argument_payload: NativeSlice<u8>,
    handles: NativeSlice<resource::TransferredHandle>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve one unix socket descriptor
    let descriptor = socket_descriptor(binding, socket, UNIX_SEND_OPERATION)?;

    // decode payload and transferred handle descriptors
    let payload = unsafe { argument_payload.as_slice()? };
    let handles = unsafe { handles.as_slice()? };
    let mut descriptors = Vec::with_capacity(handles.len());
    for handle in handles {
        let descriptor = transferable_descriptor(binding, *handle, "handles")?;
        descriptors.push(descriptor);
    }

    // build one iovec for payload bytes
    let mut iovec = libc::iovec {
        iov_base: payload.as_ptr().cast::<libc::c_void>() as *mut libc::c_void,
        iov_len: payload.len(),
    };

    // allocate one control buffer when descriptors are requested
    let descriptor_bytes = descriptors
        .len()
        .checked_mul(std::mem::size_of::<libc::c_int>())
        .ok_or_else(|| {
            core_platform::invalid_argument("handles", "handle list overflowed descriptor bytes")
        })?;
    if descriptor_bytes > u32::MAX as usize {
        return Err(core_platform::invalid_argument(
            "handles",
            "handle list exceeds host cmsg range",
        ));
    }

    let control_length = if descriptor_bytes == 0 {
        0
    } else {
        unsafe { libc::CMSG_SPACE(descriptor_bytes as u32) as usize }
    };
    let mut control = vec![0u8; control_length];

    // build one sendmsg header for payload and descriptor control
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iovec;
    message.msg_iovlen = 1;
    if !control.is_empty() {
        message.msg_control = control.as_mut_ptr().cast::<libc::c_void>();
        message.msg_controllen = control.len() as _;

        let cmsg = unsafe { libc::CMSG_FIRSTHDR(&message) };
        if cmsg.is_null() {
            return Err(core_platform::invalid_argument(
                "handles",
                "failed to allocate ancillary descriptor header",
            ));
        }

        let header = unsafe { &mut *cmsg };
        header.cmsg_level = libc::SOL_SOCKET;
        header.cmsg_type = libc::SCM_RIGHTS;
        header.cmsg_len = unsafe { libc::CMSG_LEN(descriptor_bytes as u32) as _ };

        let data = unsafe { libc::CMSG_DATA(cmsg).cast::<libc::c_int>() };
        for (index, descriptor) in descriptors.into_iter().enumerate() {
            unsafe {
                *data.add(index) = descriptor;
            }
        }
    }

    // send one ancillary payload over the unix socket
    let bytes = unsafe { libc::sendmsg(descriptor, &message, 0) };
    if bytes < 0 {
        return Err(io_error(
            UNIX_SEND_OPERATION,
            "sendmsg",
            "failed to send unix ancillary message",
        ));
    }

    unsafe {
        out.write(bytes as u64);
    }

    Ok(())
}
