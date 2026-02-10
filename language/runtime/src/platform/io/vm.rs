use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::io::{
    CompletionEventVm, CompletionOperationVm, DescriptorControlCommand, DescriptorControlFlags,
    DescriptorRequestVm, DescriptorResultVm, EventFdFlags, EventToken, PollBackend, PollEventVm,
    PollInterest, UringFeaturesVm, UringParametersVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.io.completion.cancel.
pub(super) fn destack_io_completion_cancel(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    target: resource::ResourceId,
) -> RuntimeResult<u32> {
    let _ = (handle, target);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.completion.cancel is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.completion.close.
pub(super) fn destack_io_completion_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.completion.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.completion.enter.
pub(super) fn destack_io_completion_enter(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    mincomplete: u32,
    timeoutns: u64,
    flags: u32,
) -> RuntimeResult<u32> {
    let _ = (handle, mincomplete, timeoutns, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.completion.enter is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.completion.open.
pub(super) fn destack_io_completion_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    entries: u32,
) -> RuntimeResult<resource::CompletionHandle> {
    let _ = entries;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.completion.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.completion.submit.
pub(super) fn destack_io_completion_submit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    operation: CompletionOperationVm,
) -> RuntimeResult<()> {
    let _ = (handle, operation);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.completion.submit is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.completion.submitBatch.
pub(super) fn destack_io_completion_submit_batch(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    operationwords: VmSlice<u64>,
    operationcount: u32,
    operationwordstride: u32,
) -> RuntimeResult<u32> {
    let _ = (handle, operationwords, operationcount, operationwordstride);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.completion.submitBatch is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.completion.wait.
pub(super) fn destack_io_completion_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<VmArray<CompletionEventVm>> {
    let _ = (handle, timeoutns, maxevents);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.completion.wait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.control.fcntl.
pub(super) fn destack_io_control_fcntl(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ResourceId,
    command: DescriptorControlCommand,
    argument: u64,
    flags: DescriptorControlFlags,
) -> RuntimeResult<i64> {
    let _ = (handle, command, argument, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.control.fcntl is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.control.ioctl.
pub(super) fn destack_io_control_ioctl(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ResourceId,
    request: DescriptorRequestVm,
) -> RuntimeResult<DescriptorResultVm> {
    let _ = (handle, request);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.control.ioctl is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.attach.
pub(super) fn destack_io_event_attach(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    token: EventToken,
    target: resource::ResourceId,
    key: u64,
) -> RuntimeResult<()> {
    let _ = (token, target, key);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.attach is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.close.
pub(super) fn destack_io_event_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    token: EventToken,
) -> RuntimeResult<()> {
    let _ = token;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.fdClose.
pub(super) fn destack_io_event_fd_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::EventFdHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.fdClose is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.fdOpen.
pub(super) fn destack_io_event_fd_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    initial: u64,
    flags: EventFdFlags,
) -> RuntimeResult<resource::EventFdHandle> {
    let _ = (initial, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.fdOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.fdRead.
pub(super) fn destack_io_event_fd_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::EventFdHandle,
) -> RuntimeResult<u64> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.fdRead is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.fdTryRead.
pub(super) fn destack_io_event_fd_try_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::EventFdHandle,
) -> RuntimeResult<u64> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.fdTryRead is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.fdWrite.
pub(super) fn destack_io_event_fd_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::EventFdHandle,
    value: u64,
) -> RuntimeResult<()> {
    let _ = (handle, value);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.fdWrite is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.open.
pub(super) fn destack_io_event_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    initial: u64,
) -> RuntimeResult<EventToken> {
    let _ = initial;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.read.
pub(super) fn destack_io_event_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    token: EventToken,
) -> RuntimeResult<u64> {
    let _ = token;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.signal.
pub(super) fn destack_io_event_signal(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    token: EventToken,
    value: u64,
) -> RuntimeResult<()> {
    let _ = (token, value);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.signal is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.event.tryRead.
pub(super) fn destack_io_event_try_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    token: EventToken,
) -> RuntimeResult<u64> {
    let _ = token;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.event.tryRead is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.poll.close.
pub(super) fn destack_io_poll_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.poll.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.poll.deregister.
pub(super) fn destack_io_poll_deregister(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    let _ = (handle, target);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.poll.deregister is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.poll.open.
pub(super) fn destack_io_poll_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    backend: PollBackend,
) -> RuntimeResult<resource::PollHandle> {
    let _ = backend;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.poll.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.poll.register.
pub(super) fn destack_io_poll_register(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    let _ = (handle, target, key, interest);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.poll.register is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.poll.update.
pub(super) fn destack_io_poll_update(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    let _ = (handle, target, key, interest);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.poll.update is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.poll.wait.
pub(super) fn destack_io_poll_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<VmArray<PollEventVm>> {
    let _ = (handle, timeoutns, maxevents);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.poll.wait is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.uring.close.
pub(super) fn destack_io_uring_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.uring.features.
pub(super) fn destack_io_uring_features(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<UringFeaturesVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.features is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.uring.open.
pub(super) fn destack_io_uring_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    parameters: UringParametersVm,
) -> RuntimeResult<resource::UringHandle> {
    let _ = parameters;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.uring.registerBuffers.
pub(super) fn destack_io_uring_register_buffers(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
    addresses: VmSlice<u64>,
    lengths: VmSlice<u32>,
) -> RuntimeResult<()> {
    let _ = (handle, addresses, lengths);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.registerBuffers is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.uring.registerFiles.
pub(super) fn destack_io_uring_register_files(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
    files: VmSlice<resource::ResourceId>,
) -> RuntimeResult<()> {
    let _ = (handle, files);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.registerFiles is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.uring.unregisterBuffers.
pub(super) fn destack_io_uring_unregister_buffers(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.unregisterBuffers is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.io.uring.unregisterFiles.
pub(super) fn destack_io_uring_unregister_files(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.io.uring.unregisterFiles is not available in the VM yet",
    ))
    .boxed())
}
