#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::io::bindings_generated as bindings;
use crate::platform::{NativeArray, NativeSlice, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::io::{
    CompletionEvent, CompletionOperation, DescriptorControlCommand, DescriptorControlFlags,
    DescriptorRequest, DescriptorResult, EventFdFlags, EventToken, PollBackend, PollEvent,
    PollInterest, UringFeatures, UringParameters,
};
use crate::platform::resource;

/// Stub for destack.io.completion.cancel.
pub unsafe fn destack_io_completion_cancel(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    context.check_policy(IO_COMPLETION_CANCEL)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, target);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.completion.cancel")).boxed())
}

/// Stub for destack.io.completion.close.
pub unsafe fn destack_io_completion_close(
    context: &RuntimeCallContext,
    handle: resource::CompletionHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_COMPLETION_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.completion.close")).boxed())
}

/// Stub for destack.io.completion.enter.
pub unsafe fn destack_io_completion_enter(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    mincomplete: u32,
    timeoutns: u64,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(IO_COMPLETION_ENTER)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, mincomplete, timeoutns, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.completion.enter")).boxed())
}

/// Stub for destack.io.completion.open.
pub unsafe fn destack_io_completion_open(
    context: &RuntimeCallContext,
    out: *mut resource::CompletionHandle,
    entries: u32,
) -> RuntimeResult<()> {
    context.check_policy(IO_COMPLETION_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, entries);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.completion.open")).boxed())
}

/// Stub for destack.io.completion.submit.
pub unsafe fn destack_io_completion_submit(
    context: &RuntimeCallContext,
    handle: resource::CompletionHandle,
    operation: CompletionOperation,
) -> RuntimeResult<()> {
    context.check_policy(IO_COMPLETION_SUBMIT)?;
    let _ = (handle, operation);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.completion.submit")).boxed())
}

/// Stub for destack.io.completion.submitBatch.
pub unsafe fn destack_io_completion_submit_batch(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: resource::CompletionHandle,
    operationwords: NativeSlice<u64>,
    operationcount: u32,
    operationwordstride: u32,
) -> RuntimeResult<()> {
    context.check_policy(IO_COMPLETION_SUBMIT_BATCH)?;
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

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.completion.submitBatch",
    ))
    .boxed())
}

/// Stub for destack.io.completion.wait.
pub unsafe fn destack_io_completion_wait(
    context: &RuntimeCallContext,
    out: *mut NativeArray<CompletionEvent>,
    handle: resource::CompletionHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<()> {
    context.check_policy(IO_COMPLETION_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.completion.wait")).boxed())
}

/// Stub for destack.io.control.fcntl.
pub unsafe fn destack_io_control_fcntl(
    context: &RuntimeCallContext,
    out: *mut i64,
    handle: resource::ResourceId,
    command: DescriptorControlCommand,
    argument: u64,
    flags: DescriptorControlFlags,
) -> RuntimeResult<()> {
    context.check_policy(IO_CONTROL_FCNTL)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, command, argument, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.control.fcntl")).boxed())
}

/// Stub for destack.io.control.ioctl.
pub unsafe fn destack_io_control_ioctl(
    context: &RuntimeCallContext,
    out: *mut DescriptorResult,
    handle: resource::ResourceId,
    request: DescriptorRequest,
) -> RuntimeResult<()> {
    context.check_policy(IO_CONTROL_IOCTL)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.control.ioctl")).boxed())
}

/// Stub for destack.io.event.attach.
pub unsafe fn destack_io_event_attach(
    context: &RuntimeCallContext,
    token: EventToken,
    target: resource::ResourceId,
    key: u64,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_ATTACH)?;
    let _ = (token, target, key);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.attach")).boxed())
}

/// Stub for destack.io.event.close.
pub unsafe fn destack_io_event_close(
    context: &RuntimeCallContext,
    token: EventToken,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_CLOSE)?;
    let _ = token;

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.close")).boxed())
}

/// Stub for destack.io.event.fdClose.
pub unsafe fn destack_io_event_fd_close(
    context: &RuntimeCallContext,
    handle: resource::EventFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_FD_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.fdClose")).boxed())
}

/// Stub for destack.io.event.fdOpen.
pub unsafe fn destack_io_event_fd_open(
    context: &RuntimeCallContext,
    out: *mut resource::EventFdHandle,
    initial: u64,
    flags: EventFdFlags,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_FD_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, initial, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.fdOpen")).boxed())
}

/// Stub for destack.io.event.fdRead.
pub unsafe fn destack_io_event_fd_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::EventFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_FD_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.fdRead")).boxed())
}

/// Stub for destack.io.event.fdTryRead.
pub unsafe fn destack_io_event_fd_try_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::EventFdHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_FD_TRY_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.fdTryRead")).boxed())
}

/// Stub for destack.io.event.fdWrite.
pub unsafe fn destack_io_event_fd_write(
    context: &RuntimeCallContext,
    handle: resource::EventFdHandle,
    value: u64,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_FD_WRITE)?;
    let _ = (handle, value);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.fdWrite")).boxed())
}

/// Stub for destack.io.event.open.
pub unsafe fn destack_io_event_open(
    context: &RuntimeCallContext,
    out: *mut EventToken,
    initial: u64,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, initial);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.open")).boxed())
}

/// Stub for destack.io.event.read.
pub unsafe fn destack_io_event_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    token: EventToken,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, token);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.read")).boxed())
}

/// Stub for destack.io.event.signal.
pub unsafe fn destack_io_event_signal(
    context: &RuntimeCallContext,
    token: EventToken,
    value: u64,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_SIGNAL)?;
    let _ = (token, value);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.signal")).boxed())
}

/// Stub for destack.io.event.tryRead.
pub unsafe fn destack_io_event_try_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    token: EventToken,
) -> RuntimeResult<()> {
    context.check_policy(IO_EVENT_TRY_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, token);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.event.tryRead")).boxed())
}

/// Stub for destack.io.poll.close.
pub unsafe fn destack_io_poll_close(
    context: &RuntimeCallContext,
    handle: resource::PollHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_POLL_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.poll.close")).boxed())
}

/// Stub for destack.io.poll.deregister.
pub unsafe fn destack_io_poll_deregister(
    context: &RuntimeCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    context.check_policy(IO_POLL_DEREGISTER)?;
    let _ = (handle, target);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.poll.deregister")).boxed())
}

/// Stub for destack.io.poll.open.
pub unsafe fn destack_io_poll_open(
    context: &RuntimeCallContext,
    out: *mut resource::PollHandle,
    backend: PollBackend,
) -> RuntimeResult<()> {
    context.check_policy(IO_POLL_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, backend);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.poll.open")).boxed())
}

/// Stub for destack.io.poll.register.
pub unsafe fn destack_io_poll_register(
    context: &RuntimeCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    context.check_policy(IO_POLL_REGISTER)?;
    let _ = (handle, target, key, interest);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.poll.register")).boxed())
}

/// Stub for destack.io.poll.update.
pub unsafe fn destack_io_poll_update(
    context: &RuntimeCallContext,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    context.check_policy(IO_POLL_UPDATE)?;
    let _ = (handle, target, key, interest);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.poll.update")).boxed())
}

/// Stub for destack.io.poll.wait.
pub unsafe fn destack_io_poll_wait(
    context: &RuntimeCallContext,
    out: *mut NativeArray<PollEvent>,
    handle: resource::PollHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<()> {
    context.check_policy(IO_POLL_WAIT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, timeoutns, maxevents);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.poll.wait")).boxed())
}

/// Stub for destack.io.uring.close.
pub unsafe fn destack_io_uring_close(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_URING_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.close")).boxed())
}

/// Stub for destack.io.uring.features.
pub unsafe fn destack_io_uring_features(
    context: &RuntimeCallContext,
    out: *mut UringFeatures,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_URING_FEATURES)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.features")).boxed())
}

/// Stub for destack.io.uring.open.
pub unsafe fn destack_io_uring_open(
    context: &RuntimeCallContext,
    out: *mut resource::UringHandle,
    parameters: UringParameters,
) -> RuntimeResult<()> {
    context.check_policy(IO_URING_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.io.uring.open")).boxed())
}

/// Stub for destack.io.uring.registerBuffers.
pub unsafe fn destack_io_uring_register_buffers(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
    addresses: NativeSlice<u64>,
    lengths: NativeSlice<u32>,
) -> RuntimeResult<()> {
    context.check_policy(IO_URING_REGISTER_BUFFERS)?;
    let _ = (handle, addresses, lengths);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.registerBuffers",
    ))
    .boxed())
}

/// Stub for destack.io.uring.registerFiles.
pub unsafe fn destack_io_uring_register_files(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
    files: NativeSlice<resource::ResourceId>,
) -> RuntimeResult<()> {
    context.check_policy(IO_URING_REGISTER_FILES)?;
    let _ = (handle, files);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.registerFiles",
    ))
    .boxed())
}

/// Stub for destack.io.uring.unregisterBuffers.
pub unsafe fn destack_io_uring_unregister_buffers(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_URING_UNREGISTER_BUFFERS)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.unregisterBuffers",
    ))
    .boxed())
}

/// Stub for destack.io.uring.unregisterFiles.
pub unsafe fn destack_io_uring_unregister_files(
    context: &RuntimeCallContext,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    context.check_policy(IO_URING_UNREGISTER_FILES)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.unregisterFiles",
    ))
    .boxed())
}
