use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::io::{CompletionEvent, CompletionOperation, core as io_core};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

/// Cancel queued operations for one target.
pub(crate) unsafe fn destack_io_completion_cancel(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_cancel(binding, handle, target)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Close a completion queue.
pub(crate) unsafe fn destack_io_completion_close(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
) -> RuntimeResult<()> {
    io_core::completion_close(binding, handle)
}

/// Enter the completion backend with submit and wait hints.
pub(crate) unsafe fn destack_io_completion_enter(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    mincomplete: u32,
    timeoutns: u64,
    flags: u32,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_enter(binding, handle, mincomplete, timeoutns, flags)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Open a completion queue.
pub(crate) unsafe fn destack_io_completion_open(
    binding: &BindingCallContext,
    out: *mut resource::CompletionHandle,
    entries: u32,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_open(binding, entries)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Submit one completion operation.
pub(crate) unsafe fn destack_io_completion_submit(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
    operation: CompletionOperation,
) -> RuntimeResult<()> {
    io_core::completion_submit(binding, handle, operation)
}

/// Submit a batch of completion operations.
pub(crate) unsafe fn destack_io_completion_submit_batch(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    operationwords: NativeSlice<u64>,
    operationcount: u32,
    operationwordstride: u32,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_submit_batch(
        binding,
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
pub(crate) unsafe fn destack_io_completion_wait(
    binding: &BindingCallContext,
    out: *mut NativeArray<CompletionEvent>,
    handle: resource::CompletionHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::completion_wait(binding, handle, timeoutns, maxevents)?;
    let value = binding.store_array(value);

    unsafe {
        out.write(value);
    }

    Ok(())
}
