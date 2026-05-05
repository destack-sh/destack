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
/// Close a message queue.
///
/// Close one message queue handle while keeping queue lifetime semantics explicit.
/// Queue destruction remains host-policy and may require explicit unlink operations.
///
/// # Platform
/// Unix and Windows.
/// Uses mq_close on Unix and runtime queue-handle close on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.message`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_ipc_message_queue_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_message_queue_close(binding, handle) }
}

/// Open or create a message queue.
///
/// Open one named message queue with explicit queue limits and open flags.
/// Name visibility and queue semantics follow host queue namespace rules.
///
/// # Platform
/// Unix and Windows.
/// Uses POSIX mqueue APIs on Unix and runtime emulation over named pipes or completion queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioAlreadyExists, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.message`.
///
/// # Replay
/// External, recordable.
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
///
/// Dequeue one message into caller memory with timeout control.
/// Payload truncation behavior follows host message queue semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses mq_timedreceive on Unix and runtime queue receive on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.message`.
///
/// # Replay
/// External, recordable.
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
///
/// Enqueue one payload with an explicit priority and timeout.
/// Priority ordering and wakeup semantics follow host queue behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses mq_timedsend on Unix and runtime queue send on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.message`.
///
/// # Replay
/// External, recordable.
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
///
/// Remove one message queue name from the host namespace.
/// Queue objects with live handles remain valid until final close per host semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses mq_unlink on Unix and runtime namespace removal on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.message`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_ipc_message_queue_unlink(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let name = store_string_from_vm(binding, context, name)?;
    unsafe { super::host::destack_ipc_message_queue_unlink(binding, name) }
}

/// Close one pipe endpoint.
///
/// Close one endpoint of a pipe pair.
/// Pending readers and writers observe host close semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.pipe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_ipc_pipe_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_pipe_close(binding, handle) }
}

/// Create one unnamed pipe pair.
///
/// Create one local pipe with read and write endpoints.
/// Endpoint inheritance and blocking mode follow host pipe semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses pipe2 or pipe on Unix and CreatePipe on Windows.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ipc.pipe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_ipc_pipe_open(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    flags: u32,
) -> RuntimeResult<PipePairVm> {
    call_out(|out| unsafe { super::host::destack_ipc_pipe_open(binding, out, flags) })
}

/// Read bytes from a pipe endpoint.
///
/// Read bytes into caller-provided memory from one pipe endpoint.
/// Partial reads are preserved exactly as reported by the host.
///
/// # Platform
/// Unix and Windows.
/// Uses read(2) on Unix and ReadFile on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ipc.pipe`.
///
/// # Replay
/// External, recordable.
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
///
/// Write bytes from caller-provided memory to one pipe endpoint.
/// Partial writes are preserved exactly as reported by the host.
///
/// # Platform
/// Unix and Windows.
/// Uses write(2) on Unix and WriteFile on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ipc.pipe`.
///
/// # Replay
/// External, recordable.
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
///
/// Close one shared memory handle without unmapping process mappings.
/// Mapping lifetime remains independent until explicit unmap calls.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_ipc_shared_memory_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_shared_memory_close(binding, handle) }
}

/// Create one named shared memory object.
///
/// Create one shared memory object with explicit size and creation flags.
/// Name namespace and visibility follow host object manager semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses shm_open or memfd-style APIs on Unix and file mapping objects on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioAlreadyExists, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
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
///
/// Map one region of a shared memory object into the current process address space.
/// Mapping protection and coherence follow host virtual-memory semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses mmap family on Unix and MapViewOfFile on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
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
///
/// Open one existing shared memory object by name.
/// Access rights and visibility follow host object manager semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses shm_open-style APIs on Unix and OpenFileMapping on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
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
///
/// Unmap one previously mapped memory range from the process address space.
/// Unmap operation does not destroy the underlying shared memory object.
///
/// # Platform
/// Unix and Windows.
/// Uses munmap on Unix and UnmapViewOfFile on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.shared.memory`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_ipc_shared_memory_unmap(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_shared_memory_unmap(binding, address, length) }
}

/// Wait on one shared-memory futex word.
///
/// Wait while one futex word matches the expected value.
/// Offset is byte-based within the mapped shared memory object.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wait on Linux and WaitOnAddress-style primitives on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.futex`.
///
/// # Replay
/// External, recordable.
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
///
/// Wake up to count waiters blocked on one futex word.
/// Wake ordering follows host scheduler semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses futex wake on Linux and WakeByAddress-style primitives on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.futex`.
///
/// # Replay
/// External, recordable.
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
///
/// Create one named interprocess semaphore with an initial count.
/// Name visibility and ownership follow host semaphore namespace rules.
///
/// # Platform
/// Unix and Windows.
/// Uses POSIX semaphores on Unix and named semaphore objects on Windows.
///
/// # Errors
/// Returns invalidArgument, ioAlreadyExists, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.semaphore`.
///
/// # Replay
/// External, recordable.
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
///
/// Release one waiting semaphore acquisition by incrementing the count.
/// Wakeup ordering follows host semaphore scheduling behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses sem_post on Unix and ReleaseSemaphore on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.semaphore`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_ipc_semaphore_post(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_semaphore_post(binding, handle, count) }
}

/// Wait one semaphore count.
///
/// Decrement one semaphore count, waiting up to the provided timeout.
/// Timeout units are nanoseconds and follow host wait semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses sem_timedwait or sem_wait on Unix and WaitForSingleObject on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.semaphore`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_ipc_semaphore_wait(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { super::host::destack_ipc_semaphore_wait(binding, handle, timeoutns) }
}

/// Receive payload and transferred handles.
///
/// Receive one ancillary message payload with transferred handles and credentials.
/// Handle ownership transfer is explicit and host-limited.
///
/// # Platform
/// Unix.
/// Uses recvmsg with SCM_RIGHTS and peer credential control messages.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.unix`, `ipc.fd.pass`.
///
/// # Replay
/// External, recordable.
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
///
/// Send one payload and optional transferred handles over a unix-domain socket.
/// Handle transfer semantics follow host ancillary message ownership rules.
///
/// # Platform
/// Unix.
/// Uses sendmsg with SCM_RIGHTS and optional credential control messages.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `ipc.unix`, `ipc.fd.pass`.
///
/// # Replay
/// External, recordable.
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
