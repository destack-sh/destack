#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeError;
use crate::platform::net::{SocketAddress, SocketShutdown};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, RuntimeStatus};

/// Stub for destack.net.accept.
#[unsafe(export_name = "destack.net.accept")]
pub unsafe extern "C" fn destack_net_accept(
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeStatus {
    let _ = (out, listener);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.accept")).boxed(),
        None,
    )
}

/// Stub for destack.net.close.
#[unsafe(export_name = "destack.net.close")]
pub unsafe extern "C" fn destack_net_close(handle: SocketHandle) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.close")).boxed(),
        None,
    )
}

/// Stub for destack.net.connect.
#[unsafe(export_name = "destack.net.connect")]
pub unsafe extern "C" fn destack_net_connect(
    out: *mut SocketHandle,
    host: NativeStringRef,
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
    out: *mut ListenerHandle,
    host: NativeStringRef,
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
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
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
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeStatus {
    let _ = (out, handle, buffer);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.write")).boxed(),
        None,
    )
}

/// Stub for destack.net.shutdown.
#[unsafe(export_name = "destack.net.shutdown")]
pub unsafe extern "C" fn destack_net_shutdown(
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeStatus {
    let _ = (handle, how);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.shutdown")).boxed(),
        None,
    )
}

/// Stub for destack.net.setNonblocking.
#[unsafe(export_name = "destack.net.setNonblocking")]
pub unsafe extern "C" fn destack_net_set_nonblocking(
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeStatus {
    let _ = (handle, enabled);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.setNonblocking")).boxed(),
        None,
    )
}

/// Stub for destack.net.localAddress.
#[unsafe(export_name = "destack.net.localAddress")]
pub unsafe extern "C" fn destack_net_local_address(
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeStatus {
    let _ = (out, handle);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.localAddress")).boxed(),
        None,
    )
}

/// Stub for destack.net.peerAddress.
#[unsafe(export_name = "destack.net.peerAddress")]
pub unsafe extern "C" fn destack_net_peer_address(
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeStatus {
    let _ = (out, handle);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.peerAddress")).boxed(),
        None,
    )
}

/// Stub for destack.net.setNoDelay.
#[unsafe(export_name = "destack.net.setNoDelay")]
pub unsafe extern "C" fn destack_net_set_no_delay(
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeStatus {
    let _ = (handle, enabled);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.setNoDelay")).boxed(),
        None,
    )
}

/// Stub for destack.net.setKeepAlive.
#[unsafe(export_name = "destack.net.setKeepAlive")]
pub unsafe extern "C" fn destack_net_set_keep_alive(
    handle: SocketHandle,
    enabled: bool,
    delay_seconds: u32,
) -> RuntimeStatus {
    let _ = (handle, enabled, delay_seconds);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.setKeepAlive")).boxed(),
        None,
    )
}

/// Stub for destack.net.setReuseAddr.
#[unsafe(export_name = "destack.net.setReuseAddr")]
pub unsafe extern "C" fn destack_net_set_reuse_addr(
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeStatus {
    let _ = (handle, enabled);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.setReuseAddr")).boxed(),
        None,
    )
}

/// Stub for destack.net.setReusePort.
#[unsafe(export_name = "destack.net.setReusePort")]
pub unsafe extern "C" fn destack_net_set_reuse_port(
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeStatus {
    let _ = (handle, enabled);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.setReusePort")).boxed(),
        None,
    )
}

/// Stub for destack.net.closeListener.
#[unsafe(export_name = "destack.net.closeListener")]
pub unsafe extern "C" fn destack_net_close_listener(handle: ListenerHandle) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.net.closeListener")).boxed(),
        None,
    )
}
