use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{SocketAddress, SocketShutdown};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Stub for destack.net.accept.
pub unsafe fn destack_net_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, listener);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.accept")).boxed())
}

/// Stub for destack.net.close.
pub unsafe fn destack_net_close(
    context: &RuntimeCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.close")).boxed())
}

/// Stub for destack.net.connect.
pub unsafe fn destack_net_connect(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    let _ = (context, out, host, port);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connect")).boxed())
}

/// Stub for destack.net.listen.
pub unsafe fn destack_net_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    host: NativeStringRef,
    port: u16,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (context, out, host, port, backlog);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listen")).boxed())
}

/// Stub for destack.net.read.
pub unsafe fn destack_net_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.read")).boxed())
}

/// Stub for destack.net.write.
pub unsafe fn destack_net_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.write")).boxed())
}

/// Stub for destack.net.shutdown.
pub unsafe fn destack_net_shutdown(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    let _ = (context, handle, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.shutdown")).boxed())
}

/// Stub for destack.net.setNonblocking.
pub unsafe fn destack_net_set_nonblocking(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNonblocking")).boxed())
}

/// Stub for destack.net.localAddress.
pub unsafe fn destack_net_local_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.localAddress")).boxed())
}

/// Stub for destack.net.peerAddress.
pub unsafe fn destack_net_peer_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.peerAddress")).boxed())
}

/// Stub for destack.net.setNoDelay.
pub unsafe fn destack_net_set_no_delay(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNoDelay")).boxed())
}

/// Stub for destack.net.setKeepAlive.
pub unsafe fn destack_net_set_keep_alive(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
    delay_seconds: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled, delay_seconds);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setKeepAlive")).boxed())
}

/// Stub for destack.net.setReuseAddr.
pub unsafe fn destack_net_set_reuse_addr(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReuseAddr")).boxed())
}

/// Stub for destack.net.setReusePort.
pub unsafe fn destack_net_set_reuse_port(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReusePort")).boxed())
}

/// Stub for destack.net.closeListener.
pub unsafe fn destack_net_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.closeListener")).boxed())
}
