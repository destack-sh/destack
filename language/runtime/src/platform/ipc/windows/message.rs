use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::ipc::MessageQueueReceive;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Message queue close operation.
const MESSAGE_QUEUE_CLOSE_OPERATION: &str = "destack.ipc.message.queueClose";
/// Message queue open operation.
const MESSAGE_QUEUE_OPEN_OPERATION: &str = "destack.ipc.message.queueOpen";
/// Message queue receive operation.
const MESSAGE_QUEUE_RECEIVE_OPERATION: &str = "destack.ipc.message.queueReceive";
/// Message queue send operation.
const MESSAGE_QUEUE_SEND_OPERATION: &str = "destack.ipc.message.queueSend";
/// Message queue unlink operation.
const MESSAGE_QUEUE_UNLINK_OPERATION: &str = "destack.ipc.message.queueUnlink";

/// Close a message queue.
pub(crate) unsafe fn destack_ipc_message_queue_close(
    _binding: &BindingCallContext,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(core_platform::not_supported(MESSAGE_QUEUE_CLOSE_OPERATION))
}

/// Open or create a message queue.
pub(crate) unsafe fn destack_ipc_message_queue_open(
    _binding: &BindingCallContext,
    out: *mut resource::MessageQueueHandle,
    name: NativeStringRef,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let _ = (name, flags, mode, maxmessages, maxmessagebytes);

    Err(core_platform::not_supported(MESSAGE_QUEUE_OPEN_OPERATION))
}

/// Receive one message from a queue.
pub(crate) unsafe fn destack_ipc_message_queue_receive(
    _binding: &BindingCallContext,
    out: *mut MessageQueueReceive,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let _ = (handle, timeoutns, buffer);

    Err(core_platform::not_supported(
        MESSAGE_QUEUE_RECEIVE_OPERATION,
    ))
}

/// Send one message to a queue.
pub(crate) unsafe fn destack_ipc_message_queue_send(
    _binding: &BindingCallContext,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, priority, timeoutns, argument_payload);

    Err(core_platform::not_supported(MESSAGE_QUEUE_SEND_OPERATION))
}

/// Remove a named message queue.
pub(crate) unsafe fn destack_ipc_message_queue_unlink(
    _binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = name;

    Err(core_platform::not_supported(MESSAGE_QUEUE_UNLINK_OPERATION))
}
