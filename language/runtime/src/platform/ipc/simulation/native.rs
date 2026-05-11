#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::ipc::{
    MessageQueueReceive, PipePair, SharedMemoryMapping, UnixReceiveAncillary,
};
use crate::platform::resource;

/// Close a message queue.
pub(crate) unsafe fn destack_ipc_message_queue_close(
    binding: &BindingCallContext,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueClose",
    ))
    .boxed())
}

/// Open or create a message queue.
pub(crate) unsafe fn destack_ipc_message_queue_open(
    binding: &BindingCallContext,
    out: *mut resource::MessageQueueHandle,
    name: NativeStringRef,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, name, flags, mode, maxmessages, maxmessagebytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueOpen",
    ))
    .boxed())
}

/// Receive one message from a queue.
pub(crate) unsafe fn destack_ipc_message_queue_receive(
    binding: &BindingCallContext,
    out: *mut MessageQueueReceive,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, handle, timeoutns, buffer);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueReceive",
    ))
    .boxed())
}

/// Send one message to a queue.
pub(crate) unsafe fn destack_ipc_message_queue_send(
    binding: &BindingCallContext,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, priority, timeoutns, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueSend",
    ))
    .boxed())
}

/// Remove a named message queue.
pub(crate) unsafe fn destack_ipc_message_queue_unlink(
    binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = name;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueUnlink",
    ))
    .boxed())
}

/// Close one pipe endpoint.
pub(crate) unsafe fn destack_ipc_pipe_close(
    binding: &BindingCallContext,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.close")).boxed())
}

/// Create one unnamed pipe pair.
pub(crate) unsafe fn destack_ipc_pipe_open(
    binding: &BindingCallContext,
    out: *mut PipePair,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.open")).boxed())
}

/// Read bytes from a pipe endpoint.
pub(crate) unsafe fn destack_ipc_pipe_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::PipeHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.read")).boxed())
}

/// Write bytes to a pipe endpoint.
pub(crate) unsafe fn destack_ipc_pipe_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::PipeHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.write")).boxed())
}

/// Close one shared memory object handle.
pub(crate) unsafe fn destack_ipc_shared_memory_close(
    binding: &BindingCallContext,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.close",
    ))
    .boxed())
}

/// Create one named shared memory object.
pub(crate) unsafe fn destack_ipc_shared_memory_create(
    binding: &BindingCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    size: u64,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, name, size, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.create",
    ))
    .boxed())
}

/// Map one shared memory range.
pub(crate) unsafe fn destack_ipc_shared_memory_map(
    binding: &BindingCallContext,
    out: *mut SharedMemoryMapping,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, handle, offset, length, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sharedMemory.map")).boxed())
}

/// Open one named shared memory object.
pub(crate) unsafe fn destack_ipc_shared_memory_open(
    binding: &BindingCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, name, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.open",
    ))
    .boxed())
}

/// Unmap one shared memory range.
pub(crate) unsafe fn destack_ipc_shared_memory_unmap(
    binding: &BindingCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (address, length);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.unmap",
    ))
    .boxed())
}

/// Wait on one shared-memory futex word.
pub(crate) unsafe fn destack_ipc_futex_wait(
    binding: &BindingCallContext,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (sharedmemory, offset, expected, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sync.futexWait")).boxed())
}

/// Wake futex waiters for one shared-memory word.
pub(crate) unsafe fn destack_ipc_futex_wake(
    binding: &BindingCallContext,
    out: *mut u32,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, sharedmemory, offset, count);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sync.futexWake")).boxed())
}

/// Create one named semaphore.
pub(crate) unsafe fn destack_ipc_semaphore_create(
    binding: &BindingCallContext,
    out: *mut resource::SemaphoreHandle,
    name: NativeStringRef,
    initial: u32,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, name, initial, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphoreCreate",
    ))
    .boxed())
}

/// Increment one semaphore count.
pub(crate) unsafe fn destack_ipc_semaphore_post(
    binding: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, count);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphorePost",
    ))
    .boxed())
}

/// Wait one semaphore count.
pub(crate) unsafe fn destack_ipc_semaphore_wait(
    binding: &BindingCallContext,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphoreWait",
    ))
    .boxed())
}

/// Receive payload and transferred handles.
pub(crate) unsafe fn destack_ipc_unix_receive(
    binding: &BindingCallContext,
    out: *mut UnixReceiveAncillary,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, socket, maxhandles);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.unix.receive")).boxed())
}

/// Send payload and transferred handles.
pub(crate) unsafe fn destack_ipc_unix_send(
    binding: &BindingCallContext,
    out: *mut u64,
    socket: resource::SocketHandle,
    argument_payload: NativeSlice<u8>,
    handles: NativeSlice<resource::TransferredHandle>,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, socket, argument_payload, handles);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.unix.send")).boxed())
}
