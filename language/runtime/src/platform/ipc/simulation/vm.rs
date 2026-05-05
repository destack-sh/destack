#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ipc::{
    MessageQueueReceiveVm, PipePairVm, SharedMemoryMappingVm, UnixReceiveAncillaryVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueClose",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<resource::MessageQueueHandle> {
    let _ = (name, flags, mode, maxmessages, maxmessagebytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueOpen",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: VmSlice<u8>,
) -> RuntimeResult<MessageQueueReceiveVm> {
    let _ = (handle, timeoutns, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueReceive",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, priority, timeoutns, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueSend",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueUnlink",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.close")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    flags: u32,
) -> RuntimeResult<PipePairVm> {
    let _ = flags;
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.open")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PipeHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.read")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PipeHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.write")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.close",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
    size: u64,
    flags: u32,
) -> RuntimeResult<resource::SharedMemoryHandle> {
    let _ = (name, size, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.create",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<SharedMemoryMappingVm> {
    let _ = (handle, offset, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sharedMemory.map")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
    flags: u32,
) -> RuntimeResult<resource::SharedMemoryHandle> {
    let _ = (name, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.open",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.unmap",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (sharedmemory, offset, expected, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sync.futexWait")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<u32> {
    let _ = (sharedmemory, offset, count);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sync.futexWake")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    name: vm::StringHandle,
    initial: u32,
    flags: u32,
) -> RuntimeResult<resource::SemaphoreHandle> {
    let _ = (name, initial, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphoreCreate",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    let _ = (handle, count);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphorePost",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphoreWait",
    ))
    .boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<UnixReceiveAncillaryVm> {
    let _ = (socket, maxhandles);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.unix.receive")).boxed())
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
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    socket: resource::SocketHandle,
    argument_payload: VmSlice<u8>,
    handles: VmSlice<resource::TransferredHandle>,
) -> RuntimeResult<u64> {
    let _ = (socket, argument_payload, handles);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.unix.send")).boxed())
}
