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
pub(crate) fn destack_ipc_pipe_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PipeHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.close")).boxed())
}

/// Create one unnamed pipe pair.
pub(crate) fn destack_ipc_pipe_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    flags: u32,
) -> RuntimeResult<PipePairVm> {
    let _ = flags;
    Err(RuntimeError::from(PlatformError::not_supported("destack.ipc.pipe.open")).boxed())
}

/// Read bytes from a pipe endpoint.
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
