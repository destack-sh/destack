#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::io::{
    CompletionEvent, CompletionEventVm, CompletionOperation, CompletionOperationKind,
    CompletionOperationVm, DescriptorControlCommand, DescriptorControlFlags, DescriptorRequest,
    DescriptorRequestVm, DescriptorResult, DescriptorResultVm, EventToken, PollBackend,
    PollEventVm, PollInterest, UringFeaturesVm, UringParametersVm, core as core_io,
    host as host_io,
};
use crate::platform::{NativeSlice, PlatformError, VmArray, VmSlice, VmValueCodec, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Encode one poll event slice into one VM array.
fn encode_poll_events_vm_array(
    context: &mut vm::ExternalCallContext<'_>,
    events: &[PollEventVm],
) -> VmArray<PollEventVm> {
    // encode each poll event aggregate payload
    let mut values = Vec::with_capacity(events.len());
    for event in events {
        let field_0 = vm::Value::uint(event.key, 64);
        let field_1 = vm::Value::uint(event.ready.0 as u64, 32);
        let field_2 = vm::Value::int(event.data as i64, 32);
        values.push(context.allocate_aggregate(vec![field_0, field_1, field_2]));
    }

    // allocate one vm raw value array for the encoded events
    let data = context.allocate_raw_values(values);
    VmArray {
        data,
        len: events.len() as u32,
        capacity: events.len() as u32,
        _marker: std::marker::PhantomData,
    }
}

/// Encode one completion event slice into one VM array.
fn encode_completion_events_vm_array(
    context: &mut vm::ExternalCallContext<'_>,
    events: &[CompletionEventVm],
) -> VmArray<CompletionEventVm> {
    // encode each completion event aggregate payload
    let mut values = Vec::with_capacity(events.len());
    for event in events {
        let field_0 = vm::Value::uint(event.key, 64);
        let field_1 = vm::Value::int(event.result, 64);
        let field_2 = vm::Value::uint(event.flags as u64, 32);
        values.push(context.allocate_aggregate(vec![field_0, field_1, field_2]));
    }

    // allocate one vm raw value array for the encoded events
    let data = context.allocate_raw_values(values);
    VmArray {
        data,
        len: events.len() as u32,
        capacity: events.len() as u32,
        _marker: std::marker::PhantomData,
    }
}

/// Decode one VM slice into one runtime-owned native slice.
fn slice_from_vm<T: VmValueCodec + 'static>(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<T>,
) -> RuntimeResult<NativeSlice<T>> {
    let values = values.read_values(context)?;
    Ok(runtime.store_slice(values))
}

/// Encode one native slice into one VM slice.
fn slice_to_vm<T: VmValueCodec>(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<T>,
) -> RuntimeResult<VmSlice<T>> {
    let values = unsafe { values.as_slice()? };
    VmSlice::from_values(context, values)
}

/// Decode one vm descriptor request into one native descriptor request.
fn descriptor_request_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: DescriptorRequestVm,
) -> RuntimeResult<DescriptorRequest> {
    let input = request.input.read_bytes(context)?;
    let input = runtime.store_slice(input);

    Ok(DescriptorRequest {
        code: request.code,
        input,
        output_size: request.output_size,
        flags: request.flags,
    })
}

/// Encode one native descriptor result into one vm descriptor result.
fn descriptor_result_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    result: DescriptorResult,
) -> RuntimeResult<DescriptorResultVm> {
    let output = slice_to_vm(context, result.output)?;
    Ok(DescriptorResultVm {
        return_value: result.return_value,
        output,
    })
}

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
pub(crate) fn destack_io_completion_cancel(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CompletionHandle,
    target: resource::ResourceId,
) -> RuntimeResult<u32> {
    core_io::completion_cancel(runtime, handle, target)
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
pub(crate) fn destack_io_completion_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CompletionHandle,
) -> RuntimeResult<()> {
    core_io::completion_close(runtime, handle)
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
pub(crate) fn destack_io_completion_enter(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CompletionHandle,
    mincomplete: u32,
    timeoutns: u64,
    flags: u32,
) -> RuntimeResult<u32> {
    core_io::completion_enter(runtime, handle, mincomplete, timeoutns, flags)
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
pub(crate) fn destack_io_completion_open(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    entries: u32,
) -> RuntimeResult<resource::CompletionHandle> {
    core_io::completion_open(runtime, entries)
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
pub(crate) fn destack_io_completion_submit(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CompletionHandle,
    operation: CompletionOperationVm,
) -> RuntimeResult<()> {
    core_io::completion_submit(runtime, handle, operation)
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
pub(crate) fn destack_io_completion_submit_batch(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CompletionHandle,
    operationwords: VmSlice<u64>,
    operationcount: u32,
    operationwordstride: u32,
) -> RuntimeResult<u32> {
    let operationwords = slice_from_vm(runtime, context, operationwords)?;
    core_io::completion_submit_batch(
        runtime,
        handle,
        operationwords,
        operationcount,
        operationwordstride,
    )
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
pub(crate) fn destack_io_completion_wait(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CompletionHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<VmArray<CompletionEventVm>> {
    let events = core_io::completion_wait(runtime, handle, timeoutns, maxevents)?;
    Ok(encode_completion_events_vm_array(context, &events))
}

/// Execute one fcntl-style descriptor command.
///
/// Forward one descriptor control command to the host kernel for the target resource.
/// Command semantics and valid arguments follow the active host ABI.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` when host mapping is unavailable.
/// Uses fcntl(2) style controls on Unix and host descriptor control adapters on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `io.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_control_fcntl(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::ResourceId,
    command: DescriptorControlCommand,
    argument: u64,
    flags: DescriptorControlFlags,
) -> RuntimeResult<i64> {
    host_io::host_control_fcntl(runtime, handle, command, argument, flags)
}

/// Execute one ioctl-style descriptor request.
///
/// Forward one ioctl request with opaque payload bytes to the host kernel for the target resource.
/// Request code semantics and payload layout follow the active host ABI.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` when host mapping is unavailable.
/// Uses ioctl(2) style controls on Unix and DeviceIoControl or ioctlsocket adapters on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_control_ioctl(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::ResourceId,
    request: DescriptorRequestVm,
) -> RuntimeResult<DescriptorResultVm> {
    let request = descriptor_request_from_vm(runtime, context, request)?;
    let result = host_io::host_control_ioctl(runtime, handle, request)?;
    descriptor_result_to_vm(context, result)
}

/// Attach an event token to a poll target key.
///
/// Associate one event token with one runtime resource for explicit wakeup wiring.
/// Association behavior is backend-specific and intended for runtime internals.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime event routing over host poll infrastructure.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.event`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_event_attach(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    token: EventToken,
    target: resource::ResourceId,
    key: u64,
) -> RuntimeResult<()> {
    core_io::event_attach(runtime, token, target, key)
}

/// Close a user-event token.
///
/// Close one user-event token and release host resources.
/// Closing behavior for waiters follows host wakeup semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses close semantics for eventfd, pipe-backed events, or event objects.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.event`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_event_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    token: EventToken,
) -> RuntimeResult<()> {
    core_io::event_close(runtime, token)
}

/// Create a user-event token.
///
/// Create one runtime user-event token for explicit wakeups and cross-task signaling.
/// Token semantics are stable across runtime backends.
///
/// # Platform
/// Unix and Windows.
/// Uses eventfd on Linux, pipe-backed events on other Unix hosts, and event objects on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.event`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_event_open(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    initial: u64,
) -> RuntimeResult<EventToken> {
    core_io::event_open(runtime, initial)
}

/// Signal a user-event token.
///
/// Increment one user-event token and wake waiters.
/// Value must be greater than zero.
/// Counter saturation and coalescing are host-backend defined.
///
/// # Platform
/// Unix and Windows.
/// Uses eventfd writes on Linux, pipe writes on other Unix hosts, and SetEvent on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.event`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_event_signal(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    token: EventToken,
    argument_value: u64,
) -> RuntimeResult<()> {
    core_io::event_signal(runtime, token, argument_value)
}

/// Close a poll instance.
///
/// Close one readiness poller and release host resources.
/// Registered targets are detached as part of host poller teardown.
///
/// # Platform
/// Unix and Windows.
/// Uses host poller close semantics for the selected backend.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_poll_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PollHandle,
) -> RuntimeResult<()> {
    core_io::poll_close(runtime, handle)
}

/// Remove one target from a poll instance.
///
/// Deregister one runtime resource from readiness polling.
/// Pending readiness events may still be observed depending on host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll_ctl del, kevent delete, poll table delete, or Windows readiness teardown.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_poll_deregister(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
) -> RuntimeResult<()> {
    core_io::poll_deregister(runtime, handle, target)
}

/// Open a poll instance.
///
/// Create one readiness poller with the selected backend.
/// Backend selection is validated against host capability support.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll, kqueue, poll, or the Windows readiness backend depending on backend.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_poll_open(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    backend: PollBackend,
) -> RuntimeResult<resource::PollHandle> {
    core_io::poll_open(runtime, backend)
}

/// Register one target with a poll instance.
///
/// Register one runtime resource and interest mask with the poll backend.
/// Resource ownership remains unchanged and key values are returned in wait events.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll_ctl add, kevent add, poll table add, or Windows readiness association.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_poll_register(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    core_io::poll_register(runtime, handle, target, key, interest)
}

/// Update one target in a poll instance.
///
/// Replace the registered interest mask and key for one poll target.
/// Target identity remains stable while readiness subscription changes.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll_ctl mod, kevent update, poll table update, or Windows readiness metadata update.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_poll_update(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PollHandle,
    target: resource::ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    core_io::poll_update(runtime, handle, target, key, interest)
}

/// Wait for poll events.
///
/// Wait for readiness events and return one batch of normalized poll events.
/// Timeout units are nanoseconds and follow host backend wait semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses epoll_wait, kevent wait, poll wait, or Windows readiness wait operations.
///
/// # Errors
/// Returns invalidArgument, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.poll`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_poll_wait(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PollHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<VmArray<PollEventVm>> {
    let events = core_io::poll_wait(runtime, handle, timeoutns, maxevents)?;
    Ok(encode_poll_events_vm_array(context, &events))
}

/// Close one io_uring ring.
///
/// Tear down one io_uring instance and unmap ring memory.
/// Pending entries are canceled or drained by kernel behavior.
///
/// # Platform
/// Linux.
/// Uses io_uring ring teardown and unmap operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.uring`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_uring_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    core_io::uring_close(runtime, handle)
}

/// Query io_uring feature support.
///
/// Probe one ring instance for normalized feature support metadata.
/// Feature flags are derived from host kernel capability bits.
///
/// # Platform
/// Linux.
/// Uses io_uring register and probe primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.uring`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_uring_features(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<UringFeaturesVm> {
    core_io::uring_features(runtime, handle)
}

/// Open one io_uring ring.
///
/// Create one io_uring instance with explicit setup parameters.
/// Kernel feature availability is validated during setup.
///
/// # Platform
/// Linux.
/// Uses io_uring_setup and associated ring mappings.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.uring`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_uring_open(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    parameters: UringParametersVm,
) -> RuntimeResult<resource::UringHandle> {
    core_io::uring_open(runtime, parameters)
}

/// Register fixed buffers with a ring.
///
/// Register one fixed-buffer table from address and length lanes.
/// Buffer registration semantics follow io_uring fixed-buffer contracts.
///
/// # Platform
/// Linux.
/// Uses io_uring register buffers operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.register`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_uring_register_buffers(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UringHandle,
    addresses: VmSlice<u64>,
    lengths: VmSlice<u32>,
) -> RuntimeResult<()> {
    let addresses = slice_from_vm(runtime, context, addresses)?;
    let lengths = slice_from_vm(runtime, context, lengths)?;
    core_io::uring_register_buffers(runtime, handle, addresses, lengths)
}

/// Register fixed files with a ring.
///
/// Register one fixed-file table from runtime resource identifiers.
/// File registration semantics follow io_uring fixed-file contracts.
///
/// # Platform
/// Linux.
/// Uses io_uring register files operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.register`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_uring_register_files(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UringHandle,
    files: VmSlice<resource::ResourceId>,
) -> RuntimeResult<()> {
    let files = slice_from_vm(runtime, context, files)?;
    core_io::uring_register_files(runtime, handle, files)
}

/// Unregister fixed buffers for a ring.
///
/// Remove the fixed-buffer table for one ring.
/// Pending operations that reference fixed buffers follow kernel cancellation semantics.
///
/// # Platform
/// Linux.
/// Uses io_uring unregister buffers operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.register`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_uring_unregister_buffers(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    core_io::uring_unregister_buffers(runtime, handle)
}

/// Unregister fixed files for a ring.
///
/// Remove the fixed-file table for one ring.
/// Pending operations that reference fixed files follow kernel cancellation semantics.
///
/// # Platform
/// Linux.
/// Uses io_uring unregister files operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.register`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_io_uring_unregister_files(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::UringHandle,
) -> RuntimeResult<()> {
    core_io::uring_unregister_files(runtime, handle)
}
