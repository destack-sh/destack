use crate::diagnostic::RuntimeResult;
use crate::platform::ipc::{
    MessageQueueReceive, MessageQueueReceiveVm, PipePairVm, SharedMemoryMappingVm,
    UnixReceiveAncillary, UnixReceiveAncillaryVm,
};
use crate::platform::{NativeSlice, NativeStringRef, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

use super::host as host_ipc;

/// Invoke one host call that writes through an out pointer.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate one uninitialized output slot for the host call
    let mut out = std::mem::MaybeUninit::<T>::uninit();

    // execute the host call and assume initialization on success
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Convert one VM string into one runtime native string.
fn native_string_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let value = context.string_ref(value)?;
    Ok(runtime.store_string(value.as_str()))
}

/// Build one mutable native byte slice from one vec.
fn native_bytes_from_vec(bytes: &mut Vec<u8>) -> NativeSlice<u8> {
    NativeSlice {
        data: bytes.as_mut_ptr(),
        len: bytes.len() as u32,
    }
}

/// Build one mutable native transferred-handle slice from one vec.
fn native_handles_from_vec(
    handles: &mut Vec<resource::TransferredHandle>,
) -> NativeSlice<resource::TransferredHandle> {
    NativeSlice {
        data: handles.as_mut_ptr(),
        len: handles.len() as u32,
    }
}

/// Convert one native unix ancillary payload into its VM shape.
fn unix_receive_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: UnixReceiveAncillary,
) -> RuntimeResult<UnixReceiveAncillaryVm> {
    let handles = unsafe { value.handles.as_slice()? };

    Ok(UnixReceiveAncillaryVm {
        bytes: value.bytes,
        handles: VmArray::from_values(context, handles)?,
        credentials: value.credentials,
    })
}

/// Close one message queue.
pub(crate) fn destack_ipc_message_queue_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    unsafe { host_ipc::destack_ipc_message_queue_close(runtime, handle) }
}

/// Open or create a message queue.
pub(crate) fn destack_ipc_message_queue_open(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<resource::MessageQueueHandle> {
    let name = native_string_from_vm(runtime, context, name)?;
    call_out(|out| unsafe {
        host_ipc::destack_ipc_message_queue_open(
            runtime,
            out,
            name,
            flags,
            mode,
            maxmessages,
            maxmessagebytes,
        )
    })
}

/// Receive one message from a queue.
pub(crate) fn destack_ipc_message_queue_receive(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: VmSlice<u8>,
) -> RuntimeResult<MessageQueueReceiveVm> {
    // copy the VM buffer into host memory for the receive call
    let mut bytes = buffer.read_bytes(context)?;

    // invoke host receive and copy buffer contents back into VM memory
    let receive: MessageQueueReceive = call_out(|out| {
        let buffer = native_bytes_from_vec(&mut bytes);
        unsafe {
            host_ipc::destack_ipc_message_queue_receive(runtime, out, handle, timeoutns, buffer)
        }
    })?;
    buffer.write_bytes(context, &bytes)?;

    Ok(receive)
}

/// Send one message to a queue.
pub(crate) fn destack_ipc_message_queue_send(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<()> {
    let mut bytes = argument_payload.read_bytes(context)?;
    let payload = native_bytes_from_vec(&mut bytes);
    unsafe {
        host_ipc::destack_ipc_message_queue_send(runtime, handle, priority, timeoutns, payload)
    }
}

/// Remove a named message queue.
pub(crate) fn destack_ipc_message_queue_unlink(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let name = native_string_from_vm(runtime, context, name)?;
    unsafe { host_ipc::destack_ipc_message_queue_unlink(runtime, name) }
}

/// Close one pipe endpoint.
pub(crate) fn destack_ipc_pipe_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    unsafe { host_ipc::destack_ipc_pipe_close(runtime, handle) }
}

/// Create one unnamed pipe pair.
pub(crate) fn destack_ipc_pipe_open(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    flags: u32,
) -> RuntimeResult<PipePairVm> {
    call_out(|out| unsafe { host_ipc::destack_ipc_pipe_open(runtime, out, flags) })
}

/// Read bytes from a pipe endpoint.
pub(crate) fn destack_ipc_pipe_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PipeHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // copy the VM buffer into host memory for the read call
    let mut bytes = buffer.read_bytes(context)?;

    // invoke host read and copy buffer contents back into VM memory
    let read = call_out(|out| {
        let buffer = native_bytes_from_vec(&mut bytes);
        unsafe { host_ipc::destack_ipc_pipe_read(runtime, out, handle, buffer) }
    })?;
    buffer.write_bytes(context, &bytes)?;

    Ok(read)
}

/// Write bytes to a pipe endpoint.
pub(crate) fn destack_ipc_pipe_write(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PipeHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let mut bytes = buffer.read_bytes(context)?;
    let payload = native_bytes_from_vec(&mut bytes);
    call_out(|out| unsafe { host_ipc::destack_ipc_pipe_write(runtime, out, handle, payload) })
}

/// Close one shared-memory object handle.
pub(crate) fn destack_ipc_shared_memory_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    unsafe { host_ipc::destack_ipc_shared_memory_close(runtime, handle) }
}

/// Create one named shared-memory object.
pub(crate) fn destack_ipc_shared_memory_create(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    size: u64,
    flags: u32,
) -> RuntimeResult<resource::SharedMemoryHandle> {
    let name = native_string_from_vm(runtime, context, name)?;
    call_out(|out| unsafe {
        host_ipc::destack_ipc_shared_memory_create(runtime, out, name, size, flags)
    })
}

/// Map one shared-memory range.
pub(crate) fn destack_ipc_shared_memory_map(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<SharedMemoryMappingVm> {
    call_out(|out| unsafe {
        host_ipc::destack_ipc_shared_memory_map(runtime, out, handle, offset, length, flags)
    })
}

/// Open one named shared-memory object.
pub(crate) fn destack_ipc_shared_memory_open(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    flags: u32,
) -> RuntimeResult<resource::SharedMemoryHandle> {
    let name = native_string_from_vm(runtime, context, name)?;
    call_out(|out| unsafe { host_ipc::destack_ipc_shared_memory_open(runtime, out, name, flags) })
}

/// Unmap one shared-memory range.
pub(crate) fn destack_ipc_shared_memory_unmap(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { host_ipc::destack_ipc_shared_memory_unmap(runtime, address, length) }
}

/// Wait on one shared-memory futex word.
pub(crate) fn destack_ipc_futex_wait(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_ipc::destack_ipc_futex_wait(runtime, sharedmemory, offset, expected, timeoutns) }
}

/// Wake futex waiters for one shared-memory word.
pub(crate) fn destack_ipc_futex_wake(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe {
        host_ipc::destack_ipc_futex_wake(runtime, out, sharedmemory, offset, count)
    })
}

/// Create one named semaphore.
pub(crate) fn destack_ipc_semaphore_create(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    initial: u32,
    flags: u32,
) -> RuntimeResult<resource::SemaphoreHandle> {
    let name = native_string_from_vm(runtime, context, name)?;
    call_out(|out| unsafe {
        host_ipc::destack_ipc_semaphore_create(runtime, out, name, initial, flags)
    })
}

/// Increment one semaphore count.
pub(crate) fn destack_ipc_semaphore_post(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    unsafe { host_ipc::destack_ipc_semaphore_post(runtime, handle, count) }
}

/// Wait one semaphore count.
pub(crate) fn destack_ipc_semaphore_wait(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_ipc::destack_ipc_semaphore_wait(runtime, handle, timeoutns) }
}

/// Receive payload and transferred handles.
pub(crate) fn destack_ipc_unix_receive(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<UnixReceiveAncillaryVm> {
    let receive = call_out(|out| unsafe {
        host_ipc::destack_ipc_unix_receive(runtime, out, socket, maxhandles)
    })?;

    unix_receive_to_vm(context, receive)
}

/// Send payload and transferred handles.
pub(crate) fn destack_ipc_unix_send(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    socket: resource::SocketHandle,
    argument_payload: VmSlice<u8>,
    handles: VmSlice<resource::TransferredHandle>,
) -> RuntimeResult<u64> {
    let mut payload = argument_payload.read_bytes(context)?;
    let mut handles = handles.read_values(context)?;

    call_out(|out| unsafe {
        host_ipc::destack_ipc_unix_send(
            runtime,
            out,
            socket,
            native_bytes_from_vec(&mut payload),
            native_handles_from_vec(&mut handles),
        )
    })
}
