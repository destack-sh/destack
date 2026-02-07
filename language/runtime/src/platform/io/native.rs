#![allow(clippy::missing_safety_doc)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::io::{
    CompletionEvent, CompletionOperation, EventToken, PollBackend, PollEvent, PollInterest,
    UringFeatures, UringParameters, bindings_generated as bindings,
};
use crate::platform::{NativeArray, NativeSlice, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

use bindings::*;

/// Return a not supported error for an io binding.
fn not_supported(binding_name: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(binding_name)).boxed())
}

/// Open a completion queue.
pub unsafe fn destack_io_completion_open(
    context: &RuntimeCallContext,
    out: *mut resource::CompletionHandle,
    entries: u32,
) -> RuntimeResult<()> {
    context.check_policy(COMPLETION_OPEN)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, entries);

    // NOTE #Incomplete: implement completion queue open
    not_supported("destack.io.completionOpen")
}

/// Cancel a completion operation.
pub unsafe fn destack_io_completion_cancel(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    context.check_policy(COMPLETION_CANCEL)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, target);

    // NOTE #Incomplete: implement completion queue cancel
    not_supported("destack.io.completionCancel")
}

/// Close a completion queue.
pub unsafe fn destack_io_completion_close(
    context: &RuntimeCallContext,
    handle: resource::CompletionHandle,
) -> RuntimeResult<()> {
    context.check_policy(COMPLETION_CLOSE)?;

    // NOTE #Incomplete: implement completion queue close
    let _ = handle;
    not_supported("destack.io.completionClose")
}

/// Enter a completion queue wait loop.
pub unsafe fn destack_io_completion_enter(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    mincomplete: u32,
    timeoutns: u64,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(COMPLETION_ENTER)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, mincomplete, timeoutns, flags);

    // NOTE #Incomplete: implement completion queue enter
    not_supported("destack.io.completionEnter")
}

/// Submit a single completion operation.
pub unsafe fn destack_io_completion_submit(
    context: &RuntimeCallContext,
    handle: resource::CompletionHandle,
    operation: CompletionOperation,
) -> RuntimeResult<()> {
    context.check_policy(COMPLETION_SUBMIT)?;

    // NOTE #Incomplete: implement completion queue submit
    let _ = (handle, operation);
    not_supported("destack.io.completionSubmit")
}

/// Submit a batch of completion operations.
pub unsafe fn destack_io_completion_submit_batch(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    operationwords: NativeSlice<u64>,
    operationcount: u32,
    operationwordstride: u32,
) -> RuntimeResult<()> {
    context.check_policy(COMPLETION_SUBMIT_BATCH)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (
        out,
        handle,
        operationwords,
        operationcount,
        operationwordstride,
    );

    // NOTE #Incomplete: implement completion queue batch submit
    not_supported("destack.io.completionSubmitBatch")
}

/// Wait for completion events.
pub unsafe fn destack_io_completion_wait(
    context: &RuntimeCallContext,
    out: *mut NativeArray<CompletionEvent>,
    handle: resource::CompletionHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<()> {
    context.check_policy(COMPLETION_WAIT)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns, maxevents);

    // NOTE #Incomplete: implement completion queue wait
    not_supported("destack.io.completionWait")
}

/// Attach a resource to an event token.
pub unsafe fn destack_io_event_attach(
    context: &RuntimeCallContext,
    token: EventToken,
    target: resource::ResourceId,
    key: u64,
) -> RuntimeResult<()> {
    context.check_policy(EVENT_ATTACH)?;

    // NOTE #Incomplete: implement event attach
    let _ = (token, target, key);
    not_supported("destack.io.eventAttach")
}

/// Close an event token.
pub unsafe fn destack_io_event_close(
    context: &RuntimeCallContext,
    token: EventToken,
) -> RuntimeResult<()> {
    context.check_policy(EVENT_CLOSE)?;

    // NOTE #Incomplete: implement event close
    let _ = token;
    not_supported("destack.io.eventClose")
}

/// Open an event token.
pub unsafe fn destack_io_event_open(
    context: &RuntimeCallContext,
    out: *mut EventToken,
    initial: u64,
) -> RuntimeResult<()> {
    context.check_policy(EVENT_OPEN)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, initial);

    // NOTE #Incomplete: implement event open
    not_supported("destack.io.eventOpen")
}

/// Signal an event token.
pub unsafe fn destack_io_event_signal(
    context: &RuntimeCallContext,
    token: EventToken,
    value: u64,
) -> RuntimeResult<()> {
    context.check_policy(EVENT_SIGNAL)?;

    // NOTE #Incomplete: implement event signal
    let _ = (token, value);
    not_supported("destack.io.eventSignal")
}

/// Open a poll handle.
pub unsafe fn destack_io_poll_open(
    context: &RuntimeCallContext,
    out: *mut resource::PollHandle,
    backend: PollBackend,
) -> RuntimeResult<()> {
    context.check_policy(POLL_OPEN)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, backend);

    // NOTE #Incomplete: implement poll open
    not_supported("destack.io.pollOpen")
}

/// Close a poll handle.
pub unsafe fn destack_io_poll_close(
    context: &RuntimeCallContext,
    handle: resource::PollHandle,
) -> RuntimeResult<()> {
    context.check_policy(POLL_CLOSE)?;

    // NOTE #Incomplete: implement poll close
    let _ = handle;
    not_supported("destack.io.pollClose")
}

/// Register a poll interest.
pub unsafe fn destack_io_poll_register(
    context: &RuntimeCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    context.check_policy(POLL_REGISTER)?;

    // NOTE #Incomplete: implement poll register
    let _ = (handle, target, key, interest);
    not_supported("destack.io.pollRegister")
}

/// Update a poll interest.
pub unsafe fn destack_io_poll_update(
    context: &RuntimeCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    context.check_policy(POLL_UPDATE)?;

    // NOTE #Incomplete: implement poll update
    let _ = (handle, target, key, interest);
    not_supported("destack.io.pollUpdate")
}

/// Deregister a poll interest.
pub unsafe fn destack_io_poll_deregister(
    context: &RuntimeCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    context.check_policy(POLL_DEREGISTER)?;

    // NOTE #Incomplete: implement poll deregister
    let _ = (handle, target);
    not_supported("destack.io.pollDeregister")
}

/// Wait for poll events.
pub unsafe fn destack_io_poll_wait(
    context: &RuntimeCallContext,
    out: *mut NativeArray<PollEvent>,
    handle: resource::PollHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<()> {
    context.check_policy(POLL_WAIT)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns, maxevents);

    // NOTE #Incomplete: implement poll wait
    not_supported("destack.io.pollWait")
}

/// Open an io_uring handle.
pub unsafe fn destack_io_uring_open(
    context: &RuntimeCallContext,
    out: *mut resource::UringHandle,
    parameters: UringParameters,
) -> RuntimeResult<()> {
    context.check_policy(URING_OPEN)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, parameters);

    // NOTE #Incomplete: implement io_uring open
    not_supported("destack.io.uringOpen")
}

/// Close an io_uring handle.
pub unsafe fn destack_io_uring_close(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    context.check_policy(URING_CLOSE)?;

    // NOTE #Incomplete: implement io_uring close
    let _ = handle;
    not_supported("destack.io.uringClose")
}

/// Read io_uring features.
pub unsafe fn destack_io_uring_features(
    context: &RuntimeCallContext,
    out: *mut UringFeatures,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    context.check_policy(URING_FEATURES)?;

    // validate pointers and mark arguments as used
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    // NOTE #Incomplete: implement io_uring feature query
    not_supported("destack.io.uringFeatures")
}

/// Register fixed buffers with io_uring.
pub unsafe fn destack_io_uring_register_buffers(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
    addresses: NativeSlice<u64>,
    lengths: NativeSlice<u32>,
) -> RuntimeResult<()> {
    context.check_policy(URING_REGISTER_BUFFERS)?;

    // NOTE #Incomplete: implement io_uring buffer registration
    let _ = (handle, addresses, lengths);
    not_supported("destack.io.uringRegisterBuffers")
}

/// Unregister fixed buffers with io_uring.
pub unsafe fn destack_io_uring_unregister_buffers(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    context.check_policy(URING_UNREGISTER_BUFFERS)?;

    // NOTE #Incomplete: implement io_uring buffer unregistration
    let _ = handle;
    not_supported("destack.io.uringUnregisterBuffers")
}

/// Register fixed files with io_uring.
pub unsafe fn destack_io_uring_register_files(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
    files: NativeSlice<resource::ResourceId>,
) -> RuntimeResult<()> {
    context.check_policy(URING_REGISTER_FILES)?;

    // NOTE #Incomplete: implement io_uring file registration
    let _ = (handle, files);
    not_supported("destack.io.uringRegisterFiles")
}

/// Unregister fixed files with io_uring.
pub unsafe fn destack_io_uring_unregister_files(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    context.check_policy(URING_UNREGISTER_FILES)?;

    // NOTE #Incomplete: implement io_uring file unregistration
    let _ = handle;
    not_supported("destack.io.uringUnregisterFiles")
}
