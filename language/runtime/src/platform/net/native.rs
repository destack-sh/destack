#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeError;
use crate::platform::{PlatformError, PlatformSlice, PlatformStringRef, RuntimeStatus};

/// Stub for destack.net.accept.
#[unsafe(export_name = "destack.net.accept")]
pub unsafe extern "C" fn destack_net_accept(
    out: *mut crate::platform::resource::SocketHandle,
    listener: crate::platform::resource::ListenerHandle,
) -> RuntimeStatus {
    let _ = (out, listener);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.accept")).boxed(),
        None,
    )
}

/// Stub for destack.net.close.
#[unsafe(export_name = "destack.net.close")]
pub unsafe extern "C" fn destack_net_close(
    handle: crate::platform::resource::SocketHandle,
) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.close")).boxed(),
        None,
    )
}

/// Stub for destack.net.connect.
#[unsafe(export_name = "destack.net.connect")]
pub unsafe extern "C" fn destack_net_connect(
    out: *mut crate::platform::resource::SocketHandle,
    host: PlatformStringRef,
    port: u16,
) -> RuntimeStatus {
    let _ = (out, host, port);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.connect")).boxed(),
        None,
    )
}

/// Stub for destack.net.listen.
#[unsafe(export_name = "destack.net.listen")]
pub unsafe extern "C" fn destack_net_listen(
    out: *mut crate::platform::resource::ListenerHandle,
    host: PlatformStringRef,
    port: u16,
    backlog: u32,
) -> RuntimeStatus {
    let _ = (out, host, port, backlog);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.listen")).boxed(),
        None,
    )
}

/// Stub for destack.net.read.
#[unsafe(export_name = "destack.net.read")]
pub unsafe extern "C" fn destack_net_read(
    out: *mut u64,
    handle: crate::platform::resource::SocketHandle,
    buffer: PlatformSlice<u8>,
) -> RuntimeStatus {
    let _ = (out, handle, buffer);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.read")).boxed(),
        None,
    )
}

/// Stub for destack.net.write.
#[unsafe(export_name = "destack.net.write")]
pub unsafe extern "C" fn destack_net_write(
    out: *mut u64,
    handle: crate::platform::resource::SocketHandle,
    buffer: PlatformSlice<u8>,
) -> RuntimeStatus {
    let _ = (out, handle, buffer);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.write")).boxed(),
        None,
    )
}
