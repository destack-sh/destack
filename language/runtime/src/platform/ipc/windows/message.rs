use crate::diagnostic::RuntimeResult;
use crate::platform::ipc::MessageQueueReceive;
use crate::platform::{NativeSlice, NativeStringRef, resource};
use crate::runtime::BindingCallContext;

use super::core::{ensure_out, not_supported};

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
    _context: &BindingCallContext,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(not_supported(MESSAGE_QUEUE_CLOSE_OPERATION))
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
    _context: &BindingCallContext,
    out: *mut resource::MessageQueueHandle,
    name: NativeStringRef,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    let _ = (name, flags, mode, maxmessages, maxmessagebytes);

    Err(not_supported(MESSAGE_QUEUE_OPEN_OPERATION))
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
    _context: &BindingCallContext,
    out: *mut MessageQueueReceive,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    let _ = (handle, timeoutns, buffer);

    Err(not_supported(MESSAGE_QUEUE_RECEIVE_OPERATION))
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
    _context: &BindingCallContext,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, priority, timeoutns, argument_payload);

    Err(not_supported(MESSAGE_QUEUE_SEND_OPERATION))
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
    _context: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = name;

    Err(not_supported(MESSAGE_QUEUE_UNLINK_OPERATION))
}
