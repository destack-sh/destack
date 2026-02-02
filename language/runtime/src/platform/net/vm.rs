use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{SocketAddressVm, SocketShutdown};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{PlatformError, VmSlice};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.net.accept.
pub(super) fn destack_net_accept(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    listener: ListenerHandle,
) -> RuntimeResult<SocketHandle> {
    let _ = listener;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.accept is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.close.
pub(super) fn destack_net_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.connect.
pub(super) fn destack_net_connect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
    port: u16,
) -> RuntimeResult<SocketHandle> {
    let _ = (host, port);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.connect is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.listen.
pub(super) fn destack_net_listen(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
    port: u16,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    let _ = (host, port, backlog);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.listen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.read.
pub(super) fn destack_net_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.write.
pub(super) fn destack_net_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.write is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.shutdown.
pub(super) fn destack_net_shutdown(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    let _ = (handle, how);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.shutdown is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setNonblocking.
pub(super) fn destack_net_set_nonblocking(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.setNonblocking is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.localAddress.
pub(super) fn destack_net_local_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.localAddress is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.peerAddress.
pub(super) fn destack_net_peer_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.peerAddress is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setNoDelay.
pub(super) fn destack_net_set_no_delay(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.setNoDelay is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setKeepAlive.
pub(super) fn destack_net_set_keep_alive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
    delay_seconds: u32,
) -> RuntimeResult<()> {
    let _ = (handle, enabled, delay_seconds);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.setKeepAlive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setReuseAddr.
pub(super) fn destack_net_set_reuse_addr(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.setReuseAddr is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setReusePort.
pub(super) fn destack_net_set_reuse_port(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.setReusePort is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.closeListener.
pub(super) fn destack_net_close_listener(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.closeListener is not available in the VM yet",
    ))
    .boxed())
}
