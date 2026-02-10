use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ipc::{
    MessageQueueReceiveVm, PipePairVm, SharedMemoryMappingVm, UnixReceiveAncillaryVm,
};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.ipc.message.queueClose.
pub(super) fn destack_ipc_message_queue_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueClose is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.message.queueOpen.
pub(super) fn destack_ipc_message_queue_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<resource::MessageQueueHandle> {
    let _ = (name, flags, mode, maxmessages, maxmessagebytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.message.queueReceive.
pub(super) fn destack_ipc_message_queue_receive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: VmSlice<u8>,
) -> RuntimeResult<MessageQueueReceiveVm> {
    let _ = (handle, timeoutns, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueReceive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.message.queueSend.
pub(super) fn destack_ipc_message_queue_send(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    payload: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, priority, timeoutns, payload);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueSend is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.message.queueUnlink.
pub(super) fn destack_ipc_message_queue_unlink(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.message.queueUnlink is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.pipe.close.
pub(super) fn destack_ipc_pipe_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.pipe.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.pipe.open.
pub(super) fn destack_ipc_pipe_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    flags: u32,
) -> RuntimeResult<PipePairVm> {
    let _ = flags;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.pipe.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.pipe.read.
pub(super) fn destack_ipc_pipe_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PipeHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.pipe.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.pipe.write.
pub(super) fn destack_ipc_pipe_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PipeHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.pipe.write is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sharedMemory.close.
pub(super) fn destack_ipc_shared_memory_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SharedMemoryHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sharedMemory.create.
pub(super) fn destack_ipc_shared_memory_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    size: u64,
    flags: u32,
) -> RuntimeResult<resource::SharedMemoryHandle> {
    let _ = (name, size, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.create is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sharedMemory.map.
pub(super) fn destack_ipc_shared_memory_map(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SharedMemoryHandle,
    offset: u64,
    length: u64,
    flags: u32,
) -> RuntimeResult<SharedMemoryMappingVm> {
    let _ = (handle, offset, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.map is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sharedMemory.open.
pub(super) fn destack_ipc_shared_memory_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    flags: u32,
) -> RuntimeResult<resource::SharedMemoryHandle> {
    let _ = (name, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sharedMemory.unmap.
pub(super) fn destack_ipc_shared_memory_unmap(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
    length: u64,
) -> RuntimeResult<()> {
    let _ = (address, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sharedMemory.unmap is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sync.futexWait.
pub(super) fn destack_ipc_futex_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    expected: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (sharedmemory, offset, expected, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.futexWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sync.futexWake.
pub(super) fn destack_ipc_futex_wake(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    sharedmemory: resource::SharedMemoryHandle,
    offset: u64,
    count: u32,
) -> RuntimeResult<u32> {
    let _ = (sharedmemory, offset, count);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.futexWake is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sync.semaphoreCreate.
pub(super) fn destack_ipc_semaphore_create(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    name: vm::StringHandle,
    initial: u32,
    flags: u32,
) -> RuntimeResult<resource::SemaphoreHandle> {
    let _ = (name, initial, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphoreCreate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sync.semaphorePost.
pub(super) fn destack_ipc_semaphore_post(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SemaphoreHandle,
    count: u32,
) -> RuntimeResult<()> {
    let _ = (handle, count);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphorePost is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.sync.semaphoreWait.
pub(super) fn destack_ipc_semaphore_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::SemaphoreHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.sync.semaphoreWait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.unix.receive.
pub(super) fn destack_ipc_unix_receive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    socket: resource::SocketHandle,
    maxhandles: u32,
) -> RuntimeResult<UnixReceiveAncillaryVm> {
    let _ = (socket, maxhandles);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.unix.receive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ipc.unix.send.
pub(super) fn destack_ipc_unix_send(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    socket: resource::SocketHandle,
    payload: VmSlice<u8>,
    handles: VmSlice<resource::TransferredHandle>,
) -> RuntimeResult<u64> {
    let _ = (socket, payload, handles);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ipc.unix.send is not available in the VM yet",
    ))
    .boxed())
}
