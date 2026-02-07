use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::io::{
    CompletionEventVm, CompletionOperationVm, EventToken, PollBackend, PollEventVm, PollInterest,
    UringFeaturesVm, UringParametersVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Return a not supported vm binding error.
fn vm_not_supported(binding_name: &'static str) -> RuntimeResult<()> {
    let message = format!("{binding_name} is not available in the VM yet");
    Err(RuntimeError::from(PlatformError::not_supported(message)).boxed())
}

/// Open a completion queue.
pub(super) fn destack_io_completion_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    entries: u32,
) -> RuntimeResult<resource::CompletionHandle> {
    // NOTE #Incomplete: implement vm completion queue open
    let _ = entries;
    vm_not_supported("destack.io.completionOpen")?;

    unreachable!()
}

/// Cancel a completion operation.
pub(super) fn destack_io_completion_cancel(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    target: resource::ResourceId,
) -> RuntimeResult<u32> {
    // NOTE #Incomplete: implement vm completion queue cancel
    let _ = (handle, target);
    vm_not_supported("destack.io.completionCancel")?;

    unreachable!()
}

/// Close a completion queue.
pub(super) fn destack_io_completion_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm completion queue close
    let _ = handle;
    vm_not_supported("destack.io.completionClose")
}

/// Enter a completion wait loop.
pub(super) fn destack_io_completion_enter(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    mincomplete: u32,
    timeoutns: u64,
    flags: u32,
) -> RuntimeResult<u32> {
    // NOTE #Incomplete: implement vm completion queue enter
    let _ = (handle, mincomplete, timeoutns, flags);
    vm_not_supported("destack.io.completionEnter")?;

    unreachable!()
}

/// Submit a single completion operation.
pub(super) fn destack_io_completion_submit(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    operation: CompletionOperationVm,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm completion queue submit
    let _ = (handle, operation);
    vm_not_supported("destack.io.completionSubmit")
}

/// Submit a batch of completion operations.
pub(super) fn destack_io_completion_submit_batch(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    operationwords: VmSlice<u64>,
    operationcount: u32,
    operationwordstride: u32,
) -> RuntimeResult<u32> {
    // NOTE #Incomplete: implement vm completion queue batch submit
    let _ = (handle, operationwords, operationcount, operationwordstride);
    vm_not_supported("destack.io.completionSubmitBatch")?;

    unreachable!()
}

/// Wait for completion events.
pub(super) fn destack_io_completion_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::CompletionHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<VmArray<CompletionEventVm>> {
    // NOTE #Incomplete: implement vm completion queue wait
    let _ = (handle, timeoutns, maxevents);
    vm_not_supported("destack.io.completionWait")?;

    unreachable!()
}

/// Attach a resource to an event token.
pub(super) fn destack_io_event_attach(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    token: EventToken,
    target: resource::ResourceId,
    key: u64,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm event attach
    let _ = (token, target, key);
    vm_not_supported("destack.io.eventAttach")
}

/// Close an event token.
pub(super) fn destack_io_event_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    token: EventToken,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm event close
    let _ = token;
    vm_not_supported("destack.io.eventClose")
}

/// Open an event token.
pub(super) fn destack_io_event_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    initial: u64,
) -> RuntimeResult<EventToken> {
    // NOTE #Incomplete: implement vm event open
    let _ = initial;
    vm_not_supported("destack.io.eventOpen")?;

    unreachable!()
}

/// Signal an event token.
pub(super) fn destack_io_event_signal(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    token: EventToken,
    value: u64,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm event signal
    let _ = (token, value);
    vm_not_supported("destack.io.eventSignal")
}

/// Open a poll handle.
pub(super) fn destack_io_poll_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    backend: PollBackend,
) -> RuntimeResult<resource::PollHandle> {
    // NOTE #Incomplete: implement vm poll open
    let _ = backend;
    vm_not_supported("destack.io.pollOpen")?;

    unreachable!()
}

/// Close a poll handle.
pub(super) fn destack_io_poll_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm poll close
    let _ = handle;
    vm_not_supported("destack.io.pollClose")
}

/// Register a poll interest.
pub(super) fn destack_io_poll_register(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm poll register
    let _ = (handle, target, key, interest);
    vm_not_supported("destack.io.pollRegister")
}

/// Update a poll interest.
pub(super) fn destack_io_poll_update(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm poll update
    let _ = (handle, target, key, interest);
    vm_not_supported("destack.io.pollUpdate")
}

/// Deregister a poll interest.
pub(super) fn destack_io_poll_deregister(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm poll deregister
    let _ = (handle, target);
    vm_not_supported("destack.io.pollDeregister")
}

/// Wait for poll events.
pub(super) fn destack_io_poll_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::PollHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<VmArray<PollEventVm>> {
    // NOTE #Incomplete: implement vm poll wait
    let _ = (handle, timeoutns, maxevents);
    vm_not_supported("destack.io.pollWait")?;

    unreachable!()
}

/// Open an io_uring handle.
pub(super) fn destack_io_uring_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    parameters: UringParametersVm,
) -> RuntimeResult<resource::UringHandle> {
    // NOTE #Incomplete: implement vm io_uring open
    let _ = parameters;
    vm_not_supported("destack.io.uringOpen")?;

    unreachable!()
}

/// Close an io_uring handle.
pub(super) fn destack_io_uring_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm io_uring close
    let _ = handle;
    vm_not_supported("destack.io.uringClose")
}

/// Read io_uring features.
pub(super) fn destack_io_uring_features(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<UringFeaturesVm> {
    // NOTE #Incomplete: implement vm io_uring feature query
    let _ = handle;
    vm_not_supported("destack.io.uringFeatures")?;

    unreachable!()
}

/// Register fixed buffers with io_uring.
pub(super) fn destack_io_uring_register_buffers(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
    addresses: VmSlice<u64>,
    lengths: VmSlice<u32>,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm io_uring buffer registration
    let _ = (handle, addresses, lengths);
    vm_not_supported("destack.io.uringRegisterBuffers")
}

/// Unregister fixed buffers with io_uring.
pub(super) fn destack_io_uring_unregister_buffers(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm io_uring buffer unregistration
    let _ = handle;
    vm_not_supported("destack.io.uringUnregisterBuffers")
}

/// Register fixed files with io_uring.
pub(super) fn destack_io_uring_register_files(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
    files: VmSlice<resource::ResourceId>,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm io_uring file registration
    let _ = (handle, files);
    vm_not_supported("destack.io.uringRegisterFiles")
}

/// Unregister fixed files with io_uring.
pub(super) fn destack_io_uring_unregister_files(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm io_uring file unregistration
    let _ = handle;
    vm_not_supported("destack.io.uringUnregisterFiles")
}
