use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::ipc::MessageQueueReceive;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

#[cfg(target_os = "linux")]
use super::core::{
    io_error_with_errno, message_queue_descriptor, posix_name, realtime_deadline,
    register_message_queue, timed_out, would_block,
};

/// Message queue close operation.
#[cfg(not(target_os = "linux"))]
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
    binding: &BindingCallContext,
    handle: resource::MessageQueueHandle,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        // remove one queue resource and close it through the registered finalizer
        let removed = binding.worker().resources.remove_and_finalize(
            &binding.world(),
            handle.0,
            Some(binding.engine()),
        );
        if !removed {
            return Err(core_platform::invalid_argument(
                "handle",
                "destack.ipc.message.queueClose expected one valid message-queue handle",
            ));
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (binding, handle);

    #[cfg(not(target_os = "linux"))]
    Err(core_platform::not_supported(MESSAGE_QUEUE_CLOSE_OPERATION))
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
    binding: &BindingCallContext,
    out: *mut resource::MessageQueueHandle,
    name: NativeStringRef,
    flags: u32,
    mode: u32,
    maxmessages: u32,
    maxmessagebytes: u32,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(target_os = "linux")]
    {
        /// Supported open bits for POSIX message queues.
        const SUPPORTED_OPEN_FLAGS: libc::c_int =
            libc::O_RDONLY | libc::O_WRONLY | libc::O_RDWR | libc::O_CREAT | libc::O_EXCL;
        /// Supported nonblocking bit for POSIX message queues.
        const SUPPORTED_NONBLOCK_FLAG: libc::c_int = libc::O_NONBLOCK;
        /// Access mode mask for one open call.
        const ACCESS_MODE_MASK: libc::c_int = libc::O_RDONLY | libc::O_WRONLY | libc::O_RDWR;

        // decode and normalize one queue name
        let name = posix_name(name, "name")?;

        // validate and normalize caller open flags
        let mut open_flags = libc::c_int::try_from(flags).map_err(|_| {
            core_platform::invalid_argument(
                "flags",
                "flags value exceeds host c_int range for mq_open",
            )
        })?;
        let supported_flags = SUPPORTED_OPEN_FLAGS | SUPPORTED_NONBLOCK_FLAG;
        if (open_flags & !supported_flags) != 0 {
            return Err(core_platform::invalid_argument(
                "flags",
                "flags include unsupported POSIX mq_open bits",
            ));
        }
        if (open_flags & ACCESS_MODE_MASK) == 0 {
            open_flags |= libc::O_RDWR;
        }

        // build queue attributes for create paths
        let create_requested = (open_flags & libc::O_CREAT) != 0;
        let mut attributes = unsafe { std::mem::zeroed::<libc::mq_attr>() };
        let attributes_pointer = if create_requested {
            if maxmessages == 0 {
                return Err(core_platform::invalid_argument(
                    "maxMessages",
                    "maxMessages must be greater than zero when O_CREAT is set",
                ));
            }
            if maxmessagebytes == 0 {
                return Err(core_platform::invalid_argument(
                    "maxMessageBytes",
                    "maxMessageBytes must be greater than zero when O_CREAT is set",
                ));
            }

            attributes.mq_maxmsg = i64::from(maxmessages);
            attributes.mq_msgsize = i64::from(maxmessagebytes);
            &attributes as *const libc::mq_attr as *mut libc::mq_attr
        } else {
            std::ptr::null_mut()
        };

        // open or create one queue descriptor
        let queue = unsafe {
            libc::mq_open(
                name.as_ptr(),
                open_flags,
                mode as libc::mode_t,
                attributes_pointer,
            )
        };
        if queue == -1 {
            return Err(io_error_with_errno(
                MESSAGE_QUEUE_OPEN_OPERATION,
                "mq_open",
                core_platform::get_errno(),
                "failed to open message queue",
            ));
        }

        // register queue handle and write output
        let handle = register_message_queue(binding, queue);
        unsafe {
            out.write(handle);
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (binding, name, flags, mode, maxmessages, maxmessagebytes);

    #[cfg(not(target_os = "linux"))]
    Err(core_platform::not_supported(MESSAGE_QUEUE_OPEN_OPERATION))
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
    binding: &BindingCallContext,
    out: *mut MessageQueueReceive,
    handle: resource::MessageQueueHandle,
    timeoutns: u64,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    #[cfg(target_os = "linux")]
    {
        // resolve one native queue descriptor and caller buffer
        let queue = message_queue_descriptor(binding, handle, MESSAGE_QUEUE_RECEIVE_OPERATION)?;
        let bytes = unsafe { buffer.as_mut_slice()? };

        // receive one queue message with timeout control
        let mut priority = 0u32;
        let received = if timeoutns == u64::MAX {
            unsafe {
                libc::mq_receive(
                    queue,
                    bytes.as_mut_ptr().cast::<libc::c_char>(),
                    bytes.len(),
                    &mut priority,
                )
            }
        } else {
            let deadline = realtime_deadline(timeoutns, MESSAGE_QUEUE_RECEIVE_OPERATION)?;
            unsafe {
                libc::mq_timedreceive(
                    queue,
                    bytes.as_mut_ptr().cast::<libc::c_char>(),
                    bytes.len(),
                    &mut priority,
                    &deadline,
                )
            }
        };
        if received < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::ETIMEDOUT {
                if timeoutns == 0 {
                    return Err(would_block(
                        MESSAGE_QUEUE_RECEIVE_OPERATION,
                        "failed to receive message: queue was empty",
                    ));
                }

                return Err(timed_out(
                    MESSAGE_QUEUE_RECEIVE_OPERATION,
                    "failed to receive message: timed out",
                ));
            }
            if errno == libc::EAGAIN {
                return Err(would_block(
                    MESSAGE_QUEUE_RECEIVE_OPERATION,
                    "failed to receive message: queue was empty",
                ));
            }

            return Err(io_error_with_errno(
                MESSAGE_QUEUE_RECEIVE_OPERATION,
                "mq_receive",
                errno,
                "failed to receive message",
            ));
        }

        // write message metadata output
        unsafe {
            out.write(MessageQueueReceive {
                bytes: received as u32,
                priority,
            });
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (binding, handle, timeoutns, buffer);

    #[cfg(not(target_os = "linux"))]
    Err(core_platform::not_supported(
        MESSAGE_QUEUE_RECEIVE_OPERATION,
    ))
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
    binding: &BindingCallContext,
    handle: resource::MessageQueueHandle,
    priority: u32,
    timeoutns: u64,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        // resolve one native queue descriptor and caller payload
        let queue = message_queue_descriptor(binding, handle, MESSAGE_QUEUE_SEND_OPERATION)?;
        let payload = unsafe { argument_payload.as_slice()? };

        // send one message with timeout control
        let rc = if timeoutns == u64::MAX {
            unsafe {
                libc::mq_send(
                    queue,
                    payload.as_ptr().cast::<libc::c_char>(),
                    payload.len(),
                    priority,
                )
            }
        } else {
            let deadline = realtime_deadline(timeoutns, MESSAGE_QUEUE_SEND_OPERATION)?;
            unsafe {
                libc::mq_timedsend(
                    queue,
                    payload.as_ptr().cast::<libc::c_char>(),
                    payload.len(),
                    priority,
                    &deadline,
                )
            }
        };
        if rc != 0 {
            let errno = core_platform::get_errno();
            if errno == libc::ETIMEDOUT {
                if timeoutns == 0 {
                    return Err(would_block(
                        MESSAGE_QUEUE_SEND_OPERATION,
                        "failed to send message: queue was full",
                    ));
                }

                return Err(timed_out(
                    MESSAGE_QUEUE_SEND_OPERATION,
                    "failed to send message: timed out",
                ));
            }
            if errno == libc::EAGAIN {
                return Err(would_block(
                    MESSAGE_QUEUE_SEND_OPERATION,
                    "failed to send message: queue was full",
                ));
            }

            return Err(io_error_with_errno(
                MESSAGE_QUEUE_SEND_OPERATION,
                "mq_send",
                errno,
                "failed to send message",
            ));
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (binding, handle, priority, timeoutns, argument_payload);

    #[cfg(not(target_os = "linux"))]
    Err(core_platform::not_supported(MESSAGE_QUEUE_SEND_OPERATION))
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
    binding: &BindingCallContext,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        let _ = binding;

        // decode and normalize one queue name
        let name = posix_name(name, "name")?;

        // unlink one queue namespace entry
        let rc = unsafe { libc::mq_unlink(name.as_ptr()) };
        if rc != 0 {
            return Err(io_error_with_errno(
                MESSAGE_QUEUE_UNLINK_OPERATION,
                "mq_unlink",
                core_platform::get_errno(),
                "failed to unlink message queue",
            ));
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (binding, name);

    #[cfg(not(target_os = "linux"))]
    Err(core_platform::not_supported(MESSAGE_QUEUE_UNLINK_OPERATION))
}
