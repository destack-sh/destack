use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{SocketAddress, SocketAddressVm, SocketShutdown, core as core_net};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeSlice, NativeStringRef, VmSlice};
use crate::runtime::RuntimeCallContext;

/// Accept a new connection from a listener.
pub fn destack_net_accept(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    listener: ListenerHandle,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { core_net::destack_net_accept(runtime, out, listener) })
}

/// Close a socket handle.
pub fn destack_net_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_close(runtime, handle) }
}

/// Connect to a remote host and return a socket handle.
pub fn destack_net_connect(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
    port: u16,
) -> RuntimeResult<SocketHandle> {
    let host = host_from_vm(runtime, context, host)?;

    call_out(|out| unsafe { core_net::destack_net_connect(runtime, out, host, port) })
}

/// Start listening on the given address.
pub fn destack_net_listen(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
    port: u16,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    let host = host_from_vm(runtime, context, host)?;

    call_out(|out| unsafe { core_net::destack_net_listen(runtime, out, host, port, backlog) })
}

/// Read from a socket into the provided slice.
pub fn destack_net_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native_buffer = allocate_read_buffer(runtime, buffer);
    let bytes_read =
        call_out(|out| unsafe { core_net::destack_net_read(runtime, out, handle, native_buffer) })?;
    write_read_buffer(context, buffer, native_buffer)?;

    Ok(bytes_read)
}

/// Write to a socket from the provided slice.
pub fn destack_net_write(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native_buffer = buffer_from_vm(runtime, context, buffer)?;

    call_out(|out| unsafe { core_net::destack_net_write(runtime, out, handle, native_buffer) })
}

/// Shut down a socket for reads, writes, or both.
pub fn destack_net_shutdown(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_shutdown(runtime, handle, how) }
}

/// Enable or disable nonblocking mode on a socket.
pub fn destack_net_set_nonblocking(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_nonblocking(runtime, handle, enabled) }
}

/// Read the local socket address.
pub fn destack_net_local_address(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    let address =
        call_out(|out| unsafe { core_net::destack_net_local_address(runtime, out, handle) })?;
    socket_address_to_vm(context, address)
}

/// Read the remote socket address.
pub fn destack_net_peer_address(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    let address =
        call_out(|out| unsafe { core_net::destack_net_peer_address(runtime, out, handle) })?;
    socket_address_to_vm(context, address)
}

/// Enable or disable TCP_NODELAY.
pub fn destack_net_set_no_delay(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_no_delay(runtime, handle, enabled) }
}

/// Enable or disable TCP keepalive.
pub fn destack_net_set_keep_alive(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
    delay_seconds: u32,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_keep_alive(runtime, handle, enabled, delay_seconds) }
}

/// Enable or disable SO_REUSEADDR.
pub fn destack_net_set_reuse_addr(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_reuse_addr(runtime, handle, enabled) }
}

/// Enable or disable SO_REUSEPORT.
pub fn destack_net_set_reuse_port(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_reuse_port(runtime, handle, enabled) }
}

/// Close a listener handle.
pub fn destack_net_close_listener(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_close_listener(runtime, handle) }
}

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    call(value.as_mut_ptr())?;
    Ok(unsafe { value.assume_init() })
}

fn host_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let host_ref = context
        .string_ref(host)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(runtime.store_string(host_ref.as_str()))
}

fn buffer_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    let bytes = buffer.read_bytes(context)?;
    Ok(runtime.store_slice(bytes))
}

fn allocate_read_buffer(runtime: &RuntimeCallContext, buffer: VmSlice<u8>) -> NativeSlice<u8> {
    let length = buffer.len as usize;
    runtime.store_slice(vec![0u8; length])
}

fn write_read_buffer(
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
    native: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let bytes = unsafe { native.as_slice()? };
    buffer.write_bytes(context, bytes)
}

fn socket_address_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    address: SocketAddress,
) -> RuntimeResult<SocketAddressVm> {
    let host = unsafe { address.host.as_str()? };
    let host_value = context.intern_string(host);
    let host_handle = vm::StringHandle::new(host_value);
    Ok(SocketAddressVm {
        host: host_handle,
        port: address.port,
        family: address.family,
    })
}
