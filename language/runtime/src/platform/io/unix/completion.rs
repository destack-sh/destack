use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::io::{CompletionEvent, CompletionOperation, core as io_core};
use crate::platform::{NativeArray, resource};
use crate::runtime::{BindingCallContext, NativeSlice};

/// Cancel queued operations for one target.
///
/// Cancel queued completion operations associated with one runtime resource target.
/// Cancellation count reflects host backend cancellation behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses io_uring cancel requests on Unix and CancelIoEx style APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.completion`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_completion_cancel(
    context: &BindingCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_cancel(context, handle, target)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Close a completion queue.
///
/// Close one completion queue and release host queue resources.
/// Pending operations are canceled or drained by host policy.
///
/// # Platform
/// Unix and Windows.
/// Uses host completion backend teardown semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.completion`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_completion_close(
    context: &BindingCallContext,
    handle: resource::CompletionHandle,
) -> RuntimeResult<()> {
    io_core::completion_close(context, handle)
}

/// Enter the completion backend with submit and wait hints.
///
/// Ask the backend to flush pending submissions and optionally wait for completions.
/// Enter semantics and wake behavior follow host backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses io_uring_enter on Unix and runtime-entered wait-and-drain loop on Windows.
///
/// # Errors
/// Returns invalidArgument, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.submit`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_completion_enter(
    context: &BindingCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    mincomplete: u32,
    timeoutns: u64,
    flags: u32,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_enter(context, handle, mincomplete, timeoutns, flags)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Open a completion queue.
///
/// Create one completion queue instance with backend-defined capacity.
/// Queue behavior and worker-thread integration follow host completion APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses io_uring or AIO style completion backends on Unix and IOCP on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.completion`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_completion_open(
    context: &BindingCallContext,
    out: *mut resource::CompletionHandle,
    entries: u32,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_open(context, entries)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Submit one completion operation.
///
/// Submit one operation descriptor into the completion backend queue.
/// Submission semantics follow host backend operation encoding rules.
///
/// # Platform
/// Unix and Windows.
/// Uses io_uring SQE submission on Unix and overlapped I/O submission on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.submit`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_completion_submit(
    context: &BindingCallContext,
    handle: resource::CompletionHandle,
    operation: CompletionOperation,
) -> RuntimeResult<()> {
    io_core::completion_submit(context, handle, operation)
}

/// Submit a batch of completion operations.
///
/// Submit multiple operation descriptors in one backend transaction.
/// Batch ordering is preserved as provided by the caller.
///
/// # Platform
/// Unix and Windows.
/// Uses io_uring SQE batch submission on Unix and runtime batched overlapped submission on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.submit`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_completion_submit_batch(
    context: &BindingCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    operationwords: NativeSlice<u64>,
    operationcount: u32,
    operationwordstride: u32,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_submit_batch(
        context,
        handle,
        operationwords,
        operationcount,
        operationwordstride,
    )?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Wait for completion events.
///
/// Wait for one batch of completion events and return normalized completion records.
/// Timeout units are nanoseconds and follow host completion wait semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses io_uring CQ waits on Unix and IOCP dequeue waits on Windows.
///
/// # Errors
/// Returns invalidArgument, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.completion`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_completion_wait(
    context: &BindingCallContext,
    out: *mut NativeArray<CompletionEvent>,
    handle: resource::CompletionHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_wait(context, handle, timeoutns, maxevents)?;
    let value = context.store_array(value);

    unsafe {
        out.write(value);
    }

    Ok(())
}
