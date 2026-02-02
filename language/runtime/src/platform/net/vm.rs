use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, VmSlice};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.net.accept.
pub(super) fn destack_net_accept(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    listener: crate::platform::resource::ListenerHandle,
) -> RuntimeResult<crate::platform::resource::SocketHandle> {
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
    handle: crate::platform::resource::SocketHandle,
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
) -> RuntimeResult<crate::platform::resource::SocketHandle> {
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
) -> RuntimeResult<crate::platform::resource::ListenerHandle> {
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
    handle: crate::platform::resource::SocketHandle,
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
    handle: crate::platform::resource::SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.net.write is not available in the VM yet",
    ))
    .boxed())
}
