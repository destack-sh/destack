#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::ipc::{
    MessageQueueReceive, PipePair, SharedMemoryMapping, UnixReceiveAncillary,
};
use crate::platform::resource;

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
pub(crate) unsafe fn destack_ipc_message_queue_close(
    context: &BindingCallContext,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_message_queue_open(
    context: &BindingCallContext,
    out: *mut resource::MessageQueueHandle,
    name: NativeStringRef,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, name, flags, mode, maxmessages, maxmessagebytes);

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
pub(crate) unsafe fn destack_ipc_message_queue_receive(
    context: &BindingCallContext,
    out: *mut MessageQueueReceive,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, handle, timeoutns, buffer);

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
pub(crate) unsafe fn destack_ipc_message_queue_send(
    context: &BindingCallContext,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_message_queue_unlink(
    context: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_pipe_close(
    context: &BindingCallContext,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_pipe_open(
    context: &BindingCallContext,
    out: *mut PipePair,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, flags);

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
pub(crate) unsafe fn destack_ipc_pipe_read(
    context: &BindingCallContext,
    out: *mut u64,
    handle: resource::PipeHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, handle, buffer);

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
pub(crate) unsafe fn destack_ipc_pipe_write(
    context: &BindingCallContext,
    out: *mut u64,
    handle: resource::PipeHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, handle, buffer);

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
pub(crate) unsafe fn destack_ipc_shared_memory_close(
    context: &BindingCallContext,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_shared_memory_create(
    context: &BindingCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    size: u64,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, name, size, flags);

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
pub(crate) unsafe fn destack_ipc_shared_memory_map(
    context: &BindingCallContext,
    out: *mut SharedMemoryMapping,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, handle, offset, length, flags);

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
pub(crate) unsafe fn destack_ipc_shared_memory_open(
    context: &BindingCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, name, flags);

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
pub(crate) unsafe fn destack_ipc_shared_memory_unmap(
    context: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_futex_wait(
    context: &BindingCallContext,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_futex_wake(
    context: &BindingCallContext,
    out: *mut u32,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, sharedmemory, offset, count);

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
pub(crate) unsafe fn destack_ipc_semaphore_create(
    context: &BindingCallContext,
    out: *mut resource::SemaphoreHandle,
    name: NativeStringRef,
    initial: u32,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, name, initial, flags);

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
pub(crate) unsafe fn destack_ipc_semaphore_post(
    context: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_semaphore_wait(
    context: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = context;
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
pub(crate) unsafe fn destack_ipc_unix_receive(
    context: &BindingCallContext,
    out: *mut UnixReceiveAncillary,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, socket, maxhandles);

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
pub(crate) unsafe fn destack_ipc_unix_send(
    context: &BindingCallContext,
    out: *mut u64,
    socket: resource::SocketHandle,
    argument_payload: NativeSlice<u8>,
    handles: NativeSlice<resource::TransferredHandle>,
) -> RuntimeResult<()> {
    let _ = context;
    let _ = (out, socket, argument_payload, handles);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.unix.send")).boxed())
}
