use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{SocketAddress, SocketShutdown};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Reject unsupported net accept.
pub(crate) unsafe fn destack_net_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, listener);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.accept")).boxed())
}

/// Reject unsupported net close.
pub(crate) unsafe fn destack_net_close(
    context: &RuntimeCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.close")).boxed())
}

/// Reject unsupported net connect.
pub(crate) unsafe fn destack_net_connect(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    let _ = (context, out, host, port);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connect")).boxed())
}

/// Reject unsupported net listen.
pub(crate) unsafe fn destack_net_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    host: NativeStringRef,
    port: u16,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (context, out, host, port, backlog);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listen")).boxed())
}

/// Reject unsupported net read.
pub(crate) unsafe fn destack_net_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.read")).boxed())
}

/// Reject unsupported net write.
pub(crate) unsafe fn destack_net_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.write")).boxed())
}

/// Reject unsupported net shutdown.
pub(crate) unsafe fn destack_net_shutdown(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    let _ = (context, handle, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.shutdown")).boxed())
}

/// Reject unsupported net setNonblocking.
pub(crate) unsafe fn destack_net_set_nonblocking(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNonblocking")).boxed())
}

/// Reject unsupported net localAddress.
pub(crate) unsafe fn destack_net_local_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.localAddress")).boxed())
}

/// Reject unsupported net peerAddress.
pub(crate) unsafe fn destack_net_peer_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.peerAddress")).boxed())
}

/// Reject unsupported net setNoDelay.
pub(crate) unsafe fn destack_net_set_no_delay(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNoDelay")).boxed())
}

/// Reject unsupported net setKeepAlive.
pub(crate) unsafe fn destack_net_set_keep_alive(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
    delay_seconds: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled, delay_seconds);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setKeepAlive")).boxed())
}

/// Reject unsupported net setReuseAddr.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReuseAddr")).boxed())
}

/// Reject unsupported net setReusePort.
pub(crate) unsafe fn destack_net_set_reuse_port(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReusePort")).boxed())
}

/// Reject unsupported net closeListener.
pub(crate) unsafe fn destack_net_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.closeListener")).boxed())
}
