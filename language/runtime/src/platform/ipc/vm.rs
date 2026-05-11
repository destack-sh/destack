use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::core::{call_out, store_string_from_vm};
use crate::platform::ipc::{
    MessageQueueReceive, MessageQueueReceiveVm, PipePairVm, SharedMemoryMappingVm,
    UnixReceiveAncillaryVm,
};
use crate::platform::{VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm;

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
pub(crate) fn destack_ipc_message_queue_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_message_queue_close(binding, handle) }
}

/// Open or create a message queue.
pub(crate) fn destack_ipc_message_queue_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<resource::MessageQueueHandle> {
    let name = store_string_from_vm(binding, context, name)?;
    call_out(|out| unsafe {
        super::host::destack_ipc_message_queue_open(
            binding,
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: VmSlice<u8>,
) -> RuntimeResult<MessageQueueReceiveVm> {
    // copy the VM buffer into host memory for the receive call
    let read = context.read();
    let mut bytes = buffer.read_bytes(&read)?;

    // invoke host receive and copy buffer contents back into VM memory
    let receive: MessageQueueReceive = call_out(|out| {
        let buffer = native_bytes_from_vec(&mut bytes);
        unsafe {
            super::host::destack_ipc_message_queue_receive(binding, out, handle, timeoutns, buffer)
        }
    })?;
    let mut write = context.write();
    buffer.write_bytes(&mut write, &bytes)?;

    Ok(receive)
}

/// Send one message to a queue.
pub(crate) fn destack_ipc_message_queue_send(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<()> {
    let read = context.read();
    let mut bytes = argument_payload.read_bytes(&read)?;
    let payload = native_bytes_from_vec(&mut bytes);
    unsafe {
        super::host::destack_ipc_message_queue_send(binding, handle, priority, timeoutns, payload)
    }
}

/// Remove a named message queue.
pub(crate) fn destack_ipc_message_queue_unlink(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let name = store_string_from_vm(binding, context, name)?;
    unsafe { super::host::destack_ipc_message_queue_unlink(binding, name) }
}

/// Close one pipe endpoint.
pub(crate) fn destack_ipc_pipe_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_pipe_close(binding, handle) }
}

/// Create one unnamed pipe pair.
pub(crate) fn destack_ipc_pipe_open(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    flags: u32,
) -> RuntimeResult<PipePairVm> {
    call_out(|out| unsafe { super::host::destack_ipc_pipe_open(binding, out, flags) })
}

/// Read bytes from a pipe endpoint.
pub(crate) fn destack_ipc_pipe_read(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::PipeHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // copy the VM buffer into host memory for the read call
    let read = context.read();
    let mut bytes = buffer.read_bytes(&read)?;

    // invoke host read and copy buffer contents back into VM memory
    let read = call_out(|out| {
        let buffer = native_bytes_from_vec(&mut bytes);
        unsafe { super::host::destack_ipc_pipe_read(binding, out, handle, buffer) }
    })?;
    let mut write = context.write();
    buffer.write_bytes(&mut write, &bytes)?;

    Ok(read)
}

/// Write bytes to a pipe endpoint.
pub(crate) fn destack_ipc_pipe_write(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: resource::PipeHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let read = context.read();
    let mut bytes = buffer.read_bytes(&read)?;
    let payload = native_bytes_from_vec(&mut bytes);
    call_out(|out| unsafe { super::host::destack_ipc_pipe_write(binding, out, handle, payload) })
}

/// Close one shared memory object handle.
pub(crate) fn destack_ipc_shared_memory_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_shared_memory_close(binding, handle) }
}

/// Create one named shared memory object.
pub(crate) fn destack_ipc_shared_memory_create(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
    size: u64,
    flags: u32,
) -> RuntimeResult<resource::SharedMemoryHandle> {
    let name = store_string_from_vm(binding, context, name)?;
    call_out(|out| unsafe {
        super::host::destack_ipc_shared_memory_create(binding, out, name, size, flags)
    })
}

/// Map one shared memory range.
pub(crate) fn destack_ipc_shared_memory_map(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<SharedMemoryMappingVm> {
    call_out(|out| unsafe {
        super::host::destack_ipc_shared_memory_map(binding, out, handle, offset, length, flags)
    })
}

/// Open one named shared memory object.
pub(crate) fn destack_ipc_shared_memory_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
    flags: u32,
) -> RuntimeResult<resource::SharedMemoryHandle> {
    let name = store_string_from_vm(binding, context, name)?;
    call_out(|out| unsafe {
        super::host::destack_ipc_shared_memory_open(binding, out, name, flags)
    })
}

/// Unmap one shared memory range.
pub(crate) fn destack_ipc_shared_memory_unmap(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_shared_memory_unmap(binding, address, length) }
}

/// Wait on one shared-memory futex word.
pub(crate) fn destack_ipc_futex_wait(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe {
        super::host::destack_ipc_futex_wait(binding, sharedmemory, offset, expected, timeoutns)
    }
}

/// Wake futex waiters for one shared-memory word.
pub(crate) fn destack_ipc_futex_wake(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe {
        super::host::destack_ipc_futex_wake(binding, out, sharedmemory, offset, count)
    })
}

/// Create one named semaphore.
pub(crate) fn destack_ipc_semaphore_create(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
    initial: u32,
    flags: u32,
) -> RuntimeResult<resource::SemaphoreHandle> {
    let name = store_string_from_vm(binding, context, name)?;
    call_out(|out| unsafe {
        super::host::destack_ipc_semaphore_create(binding, out, name, initial, flags)
    })
}

/// Increment one semaphore count.
pub(crate) fn destack_ipc_semaphore_post(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_semaphore_post(binding, handle, count) }
}

/// Wait one semaphore count.
pub(crate) fn destack_ipc_semaphore_wait(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_semaphore_wait(binding, handle, timeoutns) }
}

/// Receive payload and transferred handles.
pub(crate) fn destack_ipc_unix_receive(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<UnixReceiveAncillaryVm> {
    let receive = call_out(|out| unsafe {
        super::host::destack_ipc_unix_receive(binding, out, socket, maxhandles)
    })?;

    let handles = unsafe { receive.handles.as_slice()? };
    let mut write = context.write();

    Ok(UnixReceiveAncillaryVm {
        bytes: receive.bytes,
        handles: VmArray::from_values(&mut write, handles)?,
        credentials: receive.credentials,
    })
}

/// Send payload and transferred handles.
pub(crate) fn destack_ipc_unix_send(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    socket: resource::SocketHandle,
    argument_payload: VmSlice<u8>,
    handles: VmSlice<resource::TransferredHandle>,
) -> RuntimeResult<u64> {
    let read = context.read();
    let mut payload = argument_payload.read_bytes(&read)?;
    let mut handles = handles.read_values(&read)?;

    call_out(|out| unsafe {
        super::host::destack_ipc_unix_send(
            binding,
            out,
            socket,
            native_bytes_from_vec(&mut payload),
            native_handles_from_vec(&mut handles),
        )
    })
}
