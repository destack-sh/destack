use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
#[cfg(unix)]
use crate::platform::core as core_platform;
use crate::platform::diagnostic::io_error_code_from_errno;
use crate::platform::io::{
    CompletionEvent, CompletionOperation, CompletionOperationKind, EventToken, PollBackend,
    PollEvent, PollInterest, host as io_host,
};
use crate::platform::proactor::{
    Proactor, ProactorAddress, ProactorBuffer, ProactorCompletion, ProactorCompletionData,
    ProactorOp, ProactorRequest,
};
use crate::platform::resource::{self, ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, PlatformErrorCode, ResourceId};
use crate::runtime::BindingCallContext;
use crate::runtime::poller::{
    HostPoller, HostPollerFlags, PlatformInterest, PollerEvent, PollerEventPayload, PollerToken,
    PollerWakeHandle, create_host_poller_for_io,
};

/// Resource label for completion queue payloads.
const COMPLETION_RESOURCE_LABEL: &str = "io.completion";
/// Resource label for event token payloads.
pub(super) const EVENT_RESOURCE_LABEL: &str = "io.event";
/// Event mask bit for completion error records.
const COMPLETION_FLAG_ERROR: u32 = 1 << 31;
/// Event mask bit for timeout completions.
const COMPLETION_FLAG_TIMEOUT: u32 = 1 << 30;
/// Event mask bit for accepted-handle completions.
const COMPLETION_FLAG_ACCEPT: u32 = 1 << 29;
/// Maximum input bytes accepted by generic ioctl.
pub(super) const IOCTL_MAX_INPUT_BYTES: u32 = 16 * 1024 * 1024;
/// Maximum output bytes accepted by generic ioctl.
pub(super) const IOCTL_MAX_OUTPUT_BYTES: u32 = 16 * 1024 * 1024;
/// Reserved token used by proactor wake events.
const COMPLETION_RESERVED_WAKE_TOKEN: u64 = u64::MAX;

/// Runtime payload for one completion queue instance.
struct CompletionResource {
    /// Shared completion state guarded for concurrent runtime access.
    state: Mutex<CompletionState>,
}

impl CompletionResource {
    /// Construct one completion payload from one concrete proactor backend.
    fn new(proactor: Box<dyn Proactor>) -> Self {
        Self {
            state: Mutex::new(CompletionState {
                proactor,
                pending_tokens: HashMap::new(),
                queued_completions: VecDeque::new(),
            }),
        }
    }
}

/// Mutable state tracked for one completion queue.
struct CompletionState {
    /// Proactor backend that drives asynchronous operations.
    proactor: Box<dyn Proactor>,
    /// Pending operation tokens keyed to their target resource id.
    pending_tokens: HashMap<u64, ResourceId>,
    /// Buffered completions waiting to be consumed by completionWait.
    queued_completions: VecDeque<ProactorCompletion>,
}

/// Readable interest bit for io.poll.
const POLL_INTEREST_READABLE: u32 = 1 << 0;
/// Writable interest bit for io.poll.
const POLL_INTEREST_WRITABLE: u32 = 1 << 1;
/// Priority interest bit for io.poll.
const POLL_INTEREST_PRIORITY: u32 = 1 << 4;
/// Supported io.poll interest mask.
const POLL_INTEREST_SUPPORTED: u32 =
    POLL_INTEREST_READABLE | POLL_INTEREST_WRITABLE | POLL_INTEREST_PRIORITY;

/// Runtime payload for one poll resource.
struct PollResource {
    /// Poll backend state protected for concurrent runtime access.
    poller: Mutex<Box<dyn HostPoller>>,
    /// Shared poller wake path used by event routing.
    wake_handle: Option<Arc<dyn PollerWakeHandle>>,
    /// Queued synthetic events emitted by attached event tokens.
    queued_events: Mutex<VecDeque<PollEvent>>,
}

impl PollResource {
    /// Create one poll resource from one concrete backend.
    fn new(poller: Box<dyn HostPoller>) -> Self {
        let wake_handle = poller.wake_handle();

        Self {
            poller: Mutex::new(poller),
            wake_handle,
            queued_events: Mutex::new(VecDeque::new()),
        }
    }
}

/// Token-scoped key for event attachment routing.
type EventAttachmentKey = ResourceId;

/// Build one not-found error for poll handles.
fn poll_not_found(op: &'static str, handle: resource::PollHandle) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(op.to_string()),
        None,
        format!("poll handle {} not found", handle.0.0),
    ))
    .boxed()
}

/// Resolve one poll resource payload from one poll handle.
fn resolve_poll_resource(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
) -> RuntimeResult<Arc<PollResource>> {
    // resolve the poll entry payload
    let resolved = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Poll {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<PollResource>>())
                .cloned()
        })
        .flatten();

    match resolved {
        Some(resource) => Ok(resource),
        None => Err(poll_not_found("destack.io.poll.handle", handle)),
    }
}

/// Decode binding-level poll interest into backend interest and flags.
fn decode_interest(interest: PollInterest) -> RuntimeResult<(PlatformInterest, HostPollerFlags)> {
    // validate unknown interest bits
    let raw = interest.0;
    let unknown = raw & !POLL_INTEREST_SUPPORTED;
    if unknown != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "interest",
            format!("unsupported interest bits: {unknown:#x}"),
        ))
        .boxed());
    }

    // decode readiness interests
    let mut decoded = PlatformInterest::NONE;
    if (raw & POLL_INTEREST_READABLE) != 0 {
        decoded |= PlatformInterest::READABLE;
    }
    if (raw & POLL_INTEREST_WRITABLE) != 0 {
        decoded |= PlatformInterest::WRITABLE;
    }

    // decode poller flags from interest compatibility bits
    let mut flags = HostPollerFlags::NONE;
    if (raw & POLL_INTEREST_PRIORITY) != 0 {
        flags |= HostPollerFlags::PRIORITY;
    }

    // reject empty interest requests
    if decoded.is_empty() && flags.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "interest",
            "interest must include at least one requested event bit",
        ))
        .boxed());
    }

    Ok((decoded, flags))
}

/// Convert one platform poll event into one io.poll event.
fn map_poll_event(event: PollerEvent) -> Option<PollEvent> {
    // drop non-io events from mixed poll backends
    let PollerEventPayload::Io { data } = event.payload else {
        return None;
    };

    Some(PollEvent {
        key: event.token.0,
        ready: PollInterest(event.mask.0),
        data: i32::try_from(data).unwrap_or(i32::MAX),
    })
}

/// Queue one synthetic poll event for one attached event token signal.
fn queue_attached_poll_event(
    binding: &BindingCallContext,
    target: ResourceId,
    key: u64,
    value: u64,
) -> RuntimeResult<()> {
    let handle = resource::PollHandle(target);
    let poll = resolve_poll_resource(binding, handle).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "target",
            "target resource is not one poll handle",
        ))
        .boxed()
    })?;

    let event = PollEvent {
        key,
        ready: PollInterest(POLL_INTEREST_READABLE),
        data: i32::try_from(value).unwrap_or(i32::MAX),
    };

    poll.queued_events.lock().push_back(event);

    if let Some(wake_handle) = poll.wake_handle.as_ref() {
        let _ = wake_handle.wake();
    } else if let Some(mut poller) = poll.poller.try_lock() {
        let _ = poller.wake();
    }

    Ok(())
}

/// Drain queued synthetic poll events into the output buffer.
fn drain_queued_poll_events(
    queued_events: &mut VecDeque<PollEvent>,
    output: &mut Vec<PollEvent>,
    max_events: u32,
) {
    while output.len() < max_events as usize {
        let Some(event) = queued_events.pop_front() else {
            break;
        };

        output.push(event);
    }
}

/// Open one io.poll instance.
pub(super) fn poll_open(
    binding: &BindingCallContext,
    backend: PollBackend,
) -> RuntimeResult<resource::PollHandle> {
    // create the selected backend using shared platform policy
    let backend = io_host::host_map_poll_backend(backend);
    let poller = create_host_poller_for_io(backend)?;
    let resource = Arc::new(PollResource::new(poller));

    // store the poll instance as one runtime resource
    let entry = ResourceEntry::new(ResourceKind::Poll).with_payload(resource);
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    Ok(resource::PollHandle(handle))
}

/// Close one io.poll instance.
pub(super) fn poll_close(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
) -> RuntimeResult<()> {
    // verify this handle points to one poll resource
    resolve_poll_resource(binding, handle)?;

    // drop stale event token attachments for this poll handle
    {
        let mut attachments_by_token = binding
            .worker()
            .platform_state
            .io
            .event_attachments()
            .lock();
        attachments_by_token.retain(|_, attachments| {
            attachments.remove(&handle.0);
            !attachments.is_empty()
        });
    }

    // remove one poll instance from the resource table
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(poll_not_found("destack.io.poll.close", handle));
    }

    Ok(())
}

/// Register one target with one io.poll instance.
pub(super) fn poll_register(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
    target: ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    // resolve the poll instance payload
    let poll = resolve_poll_resource(binding, handle)?;

    // decode interests and resolve the target handle
    let (interests, flags) = decode_interest(interest)?;
    let target_handle = io_host::host_poll_resolve_target_handle(binding, target)?;

    // forward registration into the selected backend
    let mut poller = poll.poller.lock();
    poller.register(target, target_handle, PollerToken(key), interests, flags)
}

/// Update one registered target in one io.poll instance.
pub(super) fn poll_update(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
    target: ResourceId,
    key: u64,
    interest: PollInterest,
) -> RuntimeResult<()> {
    // resolve the poll instance payload
    let poll = resolve_poll_resource(binding, handle)?;

    // decode interests and update the existing registration
    let (interests, flags) = decode_interest(interest)?;
    let mut poller = poll.poller.lock();
    poller.update(target, PollerToken(key), interests, flags)
}

/// Remove one registered target from one io.poll instance.
pub(super) fn poll_deregister(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
    target: ResourceId,
) -> RuntimeResult<()> {
    // resolve the poll instance payload
    let poll = resolve_poll_resource(binding, handle)?;

    // forward deregistration into the selected backend
    let mut poller = poll.poller.lock();
    poller.deregister(target)
}

/// Wait for one batch of poll events from one io.poll instance.
pub(super) fn poll_wait(
    binding: &BindingCallContext,
    handle: resource::PollHandle,
    timeout_nanos: u64,
    max_events: u32,
) -> RuntimeResult<Vec<PollEvent>> {
    // reject invalid max event bounds early
    if max_events == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxevents",
            "maxevents must be greater than zero",
        ))
        .boxed());
    }

    // resolve the poll instance payload
    let poll = resolve_poll_resource(binding, handle)?;

    // consume queued synthetic events before polling the backend
    let mut output = Vec::with_capacity(max_events as usize);
    {
        let mut queued_events = poll.queued_events.lock();
        drain_queued_poll_events(&mut queued_events, &mut output, max_events);
    }
    if output.len() >= max_events as usize {
        return Ok(output);
    }

    // poll the backend and stage overflow events for future waits
    let backend_timeout = if output.is_empty() { timeout_nanos } else { 0 };
    let events = {
        let mut poller = poll.poller.lock();
        poller.poll(Some(backend_timeout))?
    };
    let mut queued_events = poll.queued_events.lock();
    for event in events {
        let Some(event) = map_poll_event(event) else {
            continue;
        };

        if output.len() < max_events as usize {
            output.push(event);
        } else {
            queued_events.push_back(event);
        }
    }
    drain_queued_poll_events(&mut queued_events, &mut output, max_events);

    Ok(output)
}

/// Build one not-found error for completion handles.
fn completion_not_found(
    operation: &'static str,
    handle: resource::CompletionHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("completion handle {} not found", handle.0.0),
    ))
    .boxed()
}

/// Build one not-found error for event tokens.
pub(super) fn event_not_found(operation: &'static str, token: EventToken) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("event token {} not found", token.0),
    ))
    .boxed()
}

/// Build one not-found error for io targets.
pub(super) fn io_target_not_found(
    operation: &'static str,
    target: ResourceId,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("target resource {} not found", target.0),
    ))
    .boxed()
}

/// Build one io error from one explicit errno and message.
#[cfg_attr(not(unix), allow(dead_code))]
fn io_error_with_errno(operation: &'static str, errno: i32, detail: String) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        io_error_code_from_errno(errno),
        None,
        Some(errno),
        Some(operation.to_string()),
        None,
        detail,
    ))
    .boxed()
}

/// Build one errno error from the current unix thread errno.
#[cfg(unix)]
pub(super) fn io_error_from_errno(operation: &'static str) -> Box<RuntimeError> {
    let errno = core_platform::get_errno();
    io_error_with_errno(
        operation,
        errno,
        format!("{operation} failed: errno {errno}"),
    )
}

/// Resolve one completion resource payload from one completion handle.
fn resolve_completion_resource(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
) -> RuntimeResult<Arc<CompletionResource>> {
    // resolve one completion resource payload
    let resolved = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Completion {
                return None;
            }

            if entry.label.as_deref() != Some(COMPLETION_RESOURCE_LABEL) {
                return None;
            }

            entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<CompletionResource>>())
                .cloned()
        })
        .flatten();

    match resolved {
        Some(resource) => Ok(resource),
        None => Err(completion_not_found("destack.io.completion.handle", handle)),
    }
}

/// Return whether one event token exists and carries the event label.
fn event_exists(binding: &BindingCallContext, token: EventToken) -> bool {
    binding
        .worker()
        .resources
        .with_entry(ResourceId(token.0), |entry| {
            if entry.kind != ResourceKind::Event {
                return false;
            }

            entry.label.as_deref() == Some(EVENT_RESOURCE_LABEL)
        })
        .unwrap_or(false)
}

/// Return the attachment key for one event token in one worker.
fn event_attachment_key(_binding: &BindingCallContext, token: EventToken) -> EventAttachmentKey {
    ResourceId(token.0)
}

/// Build one optional timeout from one nanosecond value.
fn timeout_option(timeoutns: u64) -> Option<u64> {
    if timeoutns == u64::MAX {
        None
    } else {
        Some(timeoutns)
    }
}

/// Compute one remaining timeout in nanoseconds.
fn remaining_timeout(timeoutns: u64, started_at: Instant) -> Option<u64> {
    if timeoutns == u64::MAX {
        return None;
    }

    let elapsed = started_at.elapsed().as_nanos();
    if elapsed >= timeoutns as u128 {
        return Some(0);
    }

    Some((timeoutns as u128 - elapsed) as u64)
}

/// Map one completion operation into one proactor request.
fn completion_request(
    binding: &BindingCallContext,
    operation: CompletionOperation,
) -> RuntimeResult<ProactorRequest> {
    // reject reserved runtime tokens
    if operation.key == COMPLETION_RESERVED_WAKE_TOKEN {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "operation.key",
            "key uses one reserved token value",
        ))
        .boxed());
    }

    // build one proactor request payload
    let request = match operation.kind {
        CompletionOperationKind::Read => {
            let target = io_host::host_completion_resolve_target_handle(
                binding,
                operation.target,
                "destack.io.completion.submit",
            )?;
            let data = operation.argument0 as *mut u8;
            if operation.length != 0 && data.is_null() {
                return Err(
                    RuntimeError::from(PlatformError::null_pointer("operation.argument0")).boxed(),
                );
            }
            let offset = if operation.offset == u64::MAX {
                None
            } else {
                Some(operation.offset)
            };

            ProactorRequest {
                resource_id: operation.target,
                token: operation.key,
                op: ProactorOp::Read {
                    handle: target,
                    buffer: ProactorBuffer {
                        data,
                        len: operation.length,
                    },
                    offset,
                },
            }
        }
        CompletionOperationKind::Write => {
            let target = io_host::host_completion_resolve_target_handle(
                binding,
                operation.target,
                "destack.io.completion.submit",
            )?;
            let data = operation.argument0 as *mut u8;
            if operation.length != 0 && data.is_null() {
                return Err(
                    RuntimeError::from(PlatformError::null_pointer("operation.argument0")).boxed(),
                );
            }
            let offset = if operation.offset == u64::MAX {
                None
            } else {
                Some(operation.offset)
            };

            ProactorRequest {
                resource_id: operation.target,
                token: operation.key,
                op: ProactorOp::Write {
                    handle: target,
                    buffer: ProactorBuffer {
                        data,
                        len: operation.length,
                    },
                    offset,
                },
            }
        }
        CompletionOperationKind::Accept => {
            let target = io_host::host_completion_resolve_target_handle(
                binding,
                operation.target,
                "destack.io.completion.submit",
            )?;
            ProactorRequest {
                resource_id: operation.target,
                token: operation.key,
                op: ProactorOp::Accept {
                    handle: target,
                    address: None,
                },
            }
        }
        CompletionOperationKind::Connect => {
            let target = io_host::host_completion_resolve_target_handle(
                binding,
                operation.target,
                "destack.io.completion.submit",
            )?;
            let data = operation.argument0 as *const u8;
            if operation.length != 0 && data.is_null() {
                return Err(
                    RuntimeError::from(PlatformError::null_pointer("operation.argument0")).boxed(),
                );
            }

            ProactorRequest {
                resource_id: operation.target,
                token: operation.key,
                op: ProactorOp::Connect {
                    handle: target,
                    address: ProactorAddress {
                        data,
                        len: operation.length,
                    },
                },
            }
        }
        CompletionOperationKind::Timeout => {
            let timeout_ns = if operation.offset != 0 {
                operation.offset
            } else {
                operation.argument0
            };

            ProactorRequest {
                resource_id: operation.target,
                token: operation.key,
                op: ProactorOp::Timeout { timeout_ns },
            }
        }
        CompletionOperationKind::Fsync => {
            let target = io_host::host_completion_resolve_target_handle(
                binding,
                operation.target,
                "destack.io.completion.submit",
            )?;
            ProactorRequest {
                resource_id: operation.target,
                token: operation.key,
                op: ProactorOp::Fsync { handle: target },
            }
        }
        CompletionOperationKind::Send => {
            let target = io_host::host_completion_resolve_target_handle(
                binding,
                operation.target,
                "destack.io.completion.submit",
            )?;
            let data = operation.argument0 as *mut u8;
            if operation.length != 0 && data.is_null() {
                return Err(
                    RuntimeError::from(PlatformError::null_pointer("operation.argument0")).boxed(),
                );
            }

            ProactorRequest {
                resource_id: operation.target,
                token: operation.key,
                op: ProactorOp::Send {
                    handle: target,
                    buffer: ProactorBuffer {
                        data,
                        len: operation.length,
                    },
                    flags: operation.flags,
                },
            }
        }
        CompletionOperationKind::Receive => {
            let target = io_host::host_completion_resolve_target_handle(
                binding,
                operation.target,
                "destack.io.completion.submit",
            )?;
            let data = operation.argument0 as *mut u8;
            if operation.length != 0 && data.is_null() {
                return Err(
                    RuntimeError::from(PlatformError::null_pointer("operation.argument0")).boxed(),
                );
            }

            ProactorRequest {
                resource_id: operation.target,
                token: operation.key,
                op: ProactorOp::Recv {
                    handle: target,
                    buffer: ProactorBuffer {
                        data,
                        len: operation.length,
                    },
                    flags: operation.flags,
                },
            }
        }
    };

    Ok(request)
}

/// Decode one compact completion operation record from one batch word lane.
fn decode_completion_operation_words(
    words: &[u64],
    index: u32,
) -> RuntimeResult<CompletionOperation> {
    // decode one operation kind from the compact lane
    let kind = match words[0] as u8 {
        1 => CompletionOperationKind::Read,
        2 => CompletionOperationKind::Write,
        3 => CompletionOperationKind::Accept,
        4 => CompletionOperationKind::Connect,
        5 => CompletionOperationKind::Timeout,
        6 => CompletionOperationKind::Fsync,
        7 => CompletionOperationKind::Send,
        8 => CompletionOperationKind::Receive,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "operationwords",
                format!("unknown CompletionOperationKind at index {index}"),
            ))
            .boxed());
        }
    };

    Ok(CompletionOperation {
        kind,
        target: ResourceId(words[1]),
        key: words[2],
        offset: words[3],
        length: words[4] as u32,
        flags: words[5] as u32,
        argument0: words[6],
        argument1: words[7],
    })
}

/// Merge one proactor completion batch into one completion state queue.
fn enqueue_proactor_completions(
    state: &mut CompletionState,
    completions: Vec<ProactorCompletion>,
) -> usize {
    let count = completions.len();
    for completion in completions {
        state.pending_tokens.remove(&completion.token);
        state.queued_completions.push_back(completion);
    }

    count
}

/// Map one backend completion into one io completion event.
fn completion_event_from_backend(
    binding: &BindingCallContext,
    completion: ProactorCompletion,
) -> RuntimeResult<CompletionEvent> {
    // encode one base flag mask from the completion kind
    let mut flags = completion.op as u32;
    let mut result = completion.result as i64;

    // map backend error metadata
    if completion.error_code.is_some() {
        flags |= COMPLETION_FLAG_ERROR;
        if let Some(errno) = completion.error_errno {
            result = -(errno as i64);
        } else if result >= 0 {
            result = -1;
        }
    }

    // map specialized completion payloads
    match completion.data {
        ProactorCompletionData::Accept { handle, .. } => {
            flags |= COMPLETION_FLAG_ACCEPT;
            result = io_host::host_completion_register_accepted_handle(binding, handle)?;
        }
        ProactorCompletionData::Timeout => {
            flags |= COMPLETION_FLAG_TIMEOUT;
        }
        ProactorCompletionData::None
        | ProactorCompletionData::Recv { .. }
        | ProactorCompletionData::Poll { .. } => {}
    }

    Ok(CompletionEvent {
        key: completion.token,
        result,
        flags,
    })
}

/// Open one completion queue instance.
pub(super) fn completion_open(
    binding: &BindingCallContext,
    entries: u32,
) -> RuntimeResult<resource::CompletionHandle> {
    // reject empty queue capacities
    if entries == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "entries",
            "entries must be greater than zero",
        ))
        .boxed());
    }

    // create one host completion backend
    let proactor = io_host::host_completion_create_proactor(entries)?;
    let resource = Arc::new(CompletionResource::new(proactor));

    // store one runtime completion resource
    let entry = ResourceEntry::new(ResourceKind::Completion)
        .with_label(COMPLETION_RESOURCE_LABEL)
        .with_payload(resource);
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    Ok(resource::CompletionHandle(handle))
}

/// Close one completion queue instance.
pub(super) fn completion_close(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
) -> RuntimeResult<()> {
    // verify this handle points to one completion resource
    resolve_completion_resource(binding, handle)?;

    // remove one completion queue from the resource table
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );
    if !removed {
        return Err(completion_not_found("destack.io.completion.close", handle));
    }

    Ok(())
}

/// Submit one completion operation.
pub(super) fn completion_submit(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
    operation: CompletionOperation,
) -> RuntimeResult<()> {
    // resolve the completion backend and map the request
    let resource = resolve_completion_resource(binding, handle)?;
    let request = completion_request(binding, operation)?;

    // submit one request and track the pending token
    let mut state = resource.state.lock();
    if state.pending_tokens.contains_key(&request.token) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "operation.key",
            "key is already pending in this completion queue",
        ))
        .boxed());
    }

    state.proactor.submit(request)?;
    state
        .pending_tokens
        .insert(request.token, request.resource_id);

    Ok(())
}

/// Submit a batch of completion operations.
pub(super) fn completion_submit_batch(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
    operationwords: NativeSlice<u64>,
    operationcount: u32,
    operationwordstride: u32,
) -> RuntimeResult<u32> {
    // reject invalid compact operation stride
    if operationwordstride < 8 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "operationwordstride",
            "operationwordstride must be at least 8 words",
        ))
        .boxed());
    }

    // decode one operation word slice payload
    let words = unsafe { operationwords.as_slice()? };
    let required = operationcount as usize * operationwordstride as usize;
    if words.len() < required {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "operationwords",
            "operationwords is shorter than operationcount * operationwordstride",
        ))
        .boxed());
    }

    // resolve the completion backend and predecode requests
    let resource = resolve_completion_resource(binding, handle)?;
    let mut requests = Vec::with_capacity(operationcount as usize);
    let mut seen = HashSet::with_capacity(operationcount as usize);
    for index in 0..operationcount {
        let base = index as usize * operationwordstride as usize;
        let operation = decode_completion_operation_words(&words[base..base + 8], index)?;
        let request = completion_request(binding, operation)?;
        if !seen.insert(request.token) {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "operationwords",
                format!("duplicate completion token in batch: {}", request.token),
            ))
            .boxed());
        }
        requests.push(request);
    }

    // submit the predecoded request batch
    let mut state = resource.state.lock();
    let mut submitted_tokens = Vec::with_capacity(requests.len());
    let mut submitted = 0u32;
    for request in requests {
        if state.pending_tokens.contains_key(&request.token) {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "operationwords",
                format!("completion token already pending: {}", request.token),
            ))
            .boxed());
        }

        if let Err(error) = state.proactor.submit(request) {
            for token in &submitted_tokens {
                let _ = state.proactor.cancel(*token);
                state.pending_tokens.remove(token);
            }

            return Err(error);
        }
        state
            .pending_tokens
            .insert(request.token, request.resource_id);
        submitted_tokens.push(request.token);
        submitted = submitted.saturating_add(1);
    }

    Ok(submitted)
}

/// Enter the completion backend and stage completions for later waits.
pub(super) fn completion_enter(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
    mincomplete: u32,
    timeoutns: u64,
    flags: u32,
) -> RuntimeResult<u32> {
    // reject unsupported backend enter flags
    if flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            "flags must be zero",
        ))
        .boxed());
    }

    // resolve the completion backend state
    let resource = resolve_completion_resource(binding, handle)?;
    let mut state = resource.state.lock();

    // fast path: queue already satisfies the requested minimum
    if state.queued_completions.len() >= mincomplete as usize {
        return Ok(0);
    }

    // poll until enough completions are queued or timeout expires
    let started_at = Instant::now();
    let mut queued = 0usize;
    loop {
        let timeout = remaining_timeout(timeoutns, started_at);
        if timeout == Some(0) {
            break;
        }

        let completions = state.proactor.poll(timeout)?;
        let added = enqueue_proactor_completions(&mut state, completions);
        queued = queued.saturating_add(added);

        if state.queued_completions.len() >= mincomplete as usize {
            break;
        }

        if timeout.is_some() && added == 0 {
            break;
        }
    }

    Ok(queued as u32)
}

/// Cancel queued operations for one completion target.
pub(super) fn completion_cancel(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
    target: ResourceId,
) -> RuntimeResult<u32> {
    // reject unknown targets early
    if !binding.worker().resources.contains(target) {
        return Err(io_target_not_found("destack.io.completion.cancel", target));
    }

    // resolve one completion backend and collect matching tokens
    let resource = resolve_completion_resource(binding, handle)?;
    let mut state = resource.state.lock();
    let tokens = state
        .pending_tokens
        .iter()
        .filter_map(|(token, resource_id)| (*resource_id == target).then_some(*token))
        .collect::<Vec<_>>();

    // forward cancellation requests into the backend
    for token in &tokens {
        state.proactor.cancel(*token)?;
    }

    Ok(tokens.len() as u32)
}

/// Wait for completion events from one completion queue.
pub(super) fn completion_wait(
    binding: &BindingCallContext,
    handle: resource::CompletionHandle,
    timeoutns: u64,
    maxevents: u32,
) -> RuntimeResult<Vec<CompletionEvent>> {
    // reject invalid max event bounds early
    if maxevents == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxevents",
            "maxevents must be greater than zero",
        ))
        .boxed());
    }

    // resolve one completion backend state
    let resource = resolve_completion_resource(binding, handle)?;
    let mut state = resource.state.lock();

    // poll the backend when the staged queue is empty
    if state.queued_completions.is_empty() {
        let completions = state.proactor.poll(timeout_option(timeoutns))?;
        enqueue_proactor_completions(&mut state, completions);
    }

    // map staged backend completions into io completion events
    let mut events = Vec::with_capacity(maxevents as usize);
    while events.len() < maxevents as usize {
        let Some(completion) = state.queued_completions.pop_front() else {
            break;
        };
        events.push(completion_event_from_backend(binding, completion)?);
    }

    Ok(events)
}

/// Open one user-event token.
pub(super) fn event_open(binding: &BindingCallContext, initial: u64) -> RuntimeResult<EventToken> {
    // reject one reserved eventfd increment value
    if initial == u64::MAX {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "initial",
            "initial may not be uint64::MAX",
        ))
        .boxed());
    }

    io_host::host_event_open(binding, initial)
}

/// Close one user-event token.
pub(super) fn event_close(binding: &BindingCallContext, token: EventToken) -> RuntimeResult<()> {
    // verify this token points to one event resource
    if !event_exists(binding, token) {
        return Err(event_not_found("destack.io.event.close", token));
    }

    // remove stored poll attachments for this token
    let attachment_key = event_attachment_key(binding, token);
    binding
        .worker()
        .platform_state
        .io
        .event_attachments()
        .lock()
        .remove(&attachment_key);

    io_host::host_event_close(binding, token)
}

/// Signal one user-event token.
pub(super) fn event_signal(
    binding: &BindingCallContext,
    token: EventToken,
    value: u64,
) -> RuntimeResult<()> {
    // verify this token points to one event resource
    if !event_exists(binding, token) {
        return Err(event_not_found("destack.io.event.signal", token));
    }

    // reject one reserved eventfd increment value
    if value == u64::MAX {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "value",
            "value may not be uint64::MAX",
        ))
        .boxed());
    }

    // reject zero increments for cross-platform consistency
    if value == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "value",
            "value must be greater than zero",
        ))
        .boxed());
    }

    // dispatch the host-level signal first
    io_host::host_event_signal(binding, token, value)?;

    // read current attachment mappings before dispatch
    let attachment_key = event_attachment_key(binding, token);
    let attachments = binding
        .worker()
        .platform_state
        .io
        .event_attachments()
        .lock()
        .get(&attachment_key)
        .cloned()
        .unwrap_or_default();

    // queue one synthetic poll event for each attachment
    let mut stale_targets = Vec::new();
    for (target, key) in attachments {
        let result = queue_attached_poll_event(binding, target, key, value);
        if result.is_err() {
            stale_targets.push(target);
        }
    }

    // prune stale attachments that no longer point to live poll handles
    if !stale_targets.is_empty() {
        let mut attachments_by_token = binding
            .worker()
            .platform_state
            .io
            .event_attachments()
            .lock();
        if let Some(attachments) = attachments_by_token.get_mut(&attachment_key) {
            for target in stale_targets {
                attachments.remove(&target);
            }
        }

        if matches!(
            attachments_by_token.get(&attachment_key),
            Some(attachments) if attachments.is_empty()
        ) {
            attachments_by_token.remove(&attachment_key);
        }
    }

    Ok(())
}

/// Attach one event token to one runtime target.
pub(super) fn event_attach(
    binding: &BindingCallContext,
    token: EventToken,
    target: ResourceId,
    key: u64,
) -> RuntimeResult<()> {
    // reject unknown target ids early
    if !binding.worker().resources.contains(target) {
        return Err(io_target_not_found("destack.io.event.attach", target));
    }

    // reject unknown event tokens early
    if !event_exists(binding, token) {
        return Err(event_not_found("destack.io.event.attach", token));
    }

    // reject non-poll targets for attachment routing
    if resolve_poll_resource(binding, resource::PollHandle(target)).is_err() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "target",
            "target resource is not one poll handle",
        ))
        .boxed());
    }

    // store the attachment routing metadata for this token
    let attachment_key = event_attachment_key(binding, token);
    let mut attachments = binding
        .worker()
        .platform_state
        .io
        .event_attachments()
        .lock();
    attachments
        .entry(attachment_key)
        .or_default()
        .insert(target, key);

    Ok(())
}
