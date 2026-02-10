#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ipc::bindings_generated as bindings;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::ipc::{
    MessageQueueReceive, PipePair, SharedMemoryMapping, UnixReceiveAncillary,
};
use crate::platform::resource;

/// Stub for destack.ipc.message.queueClose.
pub unsafe fn destack_ipc_message_queue_close(
    context: &RuntimeCallContext,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    context.check_policy(IPC_MESSAGE_QUEUE_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueClose",
    ))
    .boxed())
}

/// Stub for destack.ipc.message.queueOpen.
pub unsafe fn destack_ipc_message_queue_open(
    context: &RuntimeCallContext,
    out: *mut resource::MessageQueueHandle,
    name: NativeStringRef,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_MESSAGE_QUEUE_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name, flags, mode, maxmessages, maxmessagebytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueOpen",
    ))
    .boxed())
}

/// Stub for destack.ipc.message.queueReceive.
pub unsafe fn destack_ipc_message_queue_receive(
    context: &RuntimeCallContext,
    out: *mut MessageQueueReceive,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(IPC_MESSAGE_QUEUE_RECEIVE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns, buffer);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueReceive",
    ))
    .boxed())
}

/// Stub for destack.ipc.message.queueSend.
pub unsafe fn destack_ipc_message_queue_send(
    context: &RuntimeCallContext,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(IPC_MESSAGE_QUEUE_SEND)?;
    let _ = (handle, priority, timeoutns, payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueSend",
    ))
    .boxed())
}

/// Stub for destack.ipc.message.queueUnlink.
pub unsafe fn destack_ipc_message_queue_unlink(
    context: &RuntimeCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(IPC_MESSAGE_QUEUE_UNLINK)?;
    let _ = name;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueUnlink",
    ))
    .boxed())
}

/// Stub for destack.ipc.pipe.close.
pub unsafe fn destack_ipc_pipe_close(
    context: &RuntimeCallContext,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    context.check_policy(IPC_PIPE_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.close")).boxed())
}

/// Stub for destack.ipc.pipe.open.
pub unsafe fn destack_ipc_pipe_open(
    context: &RuntimeCallContext,
    out: *mut PipePair,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_PIPE_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.open")).boxed())
}

/// Stub for destack.ipc.pipe.read.
pub unsafe fn destack_ipc_pipe_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::PipeHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(IPC_PIPE_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.read")).boxed())
}

/// Stub for destack.ipc.pipe.write.
pub unsafe fn destack_ipc_pipe_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::PipeHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(IPC_PIPE_WRITE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.write")).boxed())
}

/// Stub for destack.ipc.sharedMemory.close.
pub unsafe fn destack_ipc_shared_memory_close(
    context: &RuntimeCallContext,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SHARED_MEMORY_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.close",
    ))
    .boxed())
}

/// Stub for destack.ipc.sharedMemory.create.
pub unsafe fn destack_ipc_shared_memory_create(
    context: &RuntimeCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    size: u64,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SHARED_MEMORY_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name, size, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.create",
    ))
    .boxed())
}

/// Stub for destack.ipc.sharedMemory.map.
pub unsafe fn destack_ipc_shared_memory_map(
    context: &RuntimeCallContext,
    out: *mut SharedMemoryMapping,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SHARED_MEMORY_MAP)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, offset, length, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sharedMemory.map")).boxed())
}

/// Stub for destack.ipc.sharedMemory.open.
pub unsafe fn destack_ipc_shared_memory_open(
    context: &RuntimeCallContext,
    out: *mut resource::SharedMemoryHandle,
    name: NativeStringRef,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SHARED_MEMORY_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.open",
    ))
    .boxed())
}

/// Stub for destack.ipc.sharedMemory.unmap.
pub unsafe fn destack_ipc_shared_memory_unmap(
    context: &RuntimeCallContext,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SHARED_MEMORY_UNMAP)?;
    let _ = (address, length);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.unmap",
    ))
    .boxed())
}

/// Stub for destack.ipc.sync.futexWait.
pub unsafe fn destack_ipc_futex_wait(
    context: &RuntimeCallContext,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SYNC_FUTEX_WAIT)?;
    let _ = (sharedmemory, offset, expected, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sync.futexWait")).boxed())
}

/// Stub for destack.ipc.sync.futexWake.
pub unsafe fn destack_ipc_futex_wake(
    context: &RuntimeCallContext,
    out: *mut u32,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SYNC_FUTEX_WAKE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, sharedmemory, offset, count);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.sync.futexWake")).boxed())
}

/// Stub for destack.ipc.sync.semaphoreCreate.
pub unsafe fn destack_ipc_semaphore_create(
    context: &RuntimeCallContext,
    out: *mut resource::SemaphoreHandle,
    name: NativeStringRef,
    initial: u32,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SYNC_SEMAPHORE_CREATE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, name, initial, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphoreCreate",
    ))
    .boxed())
}

/// Stub for destack.ipc.sync.semaphorePost.
pub unsafe fn destack_ipc_semaphore_post(
    context: &RuntimeCallContext,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SYNC_SEMAPHORE_POST)?;
    let _ = (handle, count);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphorePost",
    ))
    .boxed())
}

/// Stub for destack.ipc.sync.semaphoreWait.
pub unsafe fn destack_ipc_semaphore_wait(
    context: &RuntimeCallContext,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(IPC_SYNC_SEMAPHORE_WAIT)?;
    let _ = (handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphoreWait",
    ))
    .boxed())
}

/// Stub for destack.ipc.unix.receive.
pub unsafe fn destack_ipc_unix_receive(
    context: &RuntimeCallContext,
    out: *mut UnixReceiveAncillary,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<()> {
    context.check_policy(IPC_UNIX_RECEIVE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, socket, maxhandles);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.unix.receive")).boxed())
}

/// Stub for destack.ipc.unix.send.
pub unsafe fn destack_ipc_unix_send(
    context: &RuntimeCallContext,
    out: *mut u64,
    socket: resource::SocketHandle,
    payload: NativeSlice<u8>,
    handles: NativeSlice<resource::TransferredHandle>,
) -> RuntimeResult<()> {
    context.check_policy(IPC_UNIX_SEND)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, socket, payload, handles);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.unix.send")).boxed())
}
