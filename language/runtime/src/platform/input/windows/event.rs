use super::{core as input_core, raw as raw_input};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{InputEvent, InputReadMode};
use crate::platform::resource::{ResourceFinalizer, ResourceId, ResourceKind};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

const INPUT_MONITOR_RESOURCE_LABEL: &str = "input.monitor";
static WINDOWS_MONITOR_STREAMS: AtomicUsize = AtomicUsize::new(0);

/// Resource payload for one windows monitor handle.
#[derive(Debug)]
struct WindowsInputMonitorBinding {
    /// Next per-monitor event sequence number.
    next_sequence: u64,
}

/// Finalizer payload for monitor stream ownership.
#[derive(Debug)]
struct WindowsMonitorFinalizer;

impl ResourceFinalizer for WindowsMonitorFinalizer {
    /// Release one monitor stream lane on resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        WINDOWS_MONITOR_STREAMS.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Return whether one runtime error carries io-would-block.
fn is_io_would_block(error: &RuntimeError) -> bool {
    error.platform_error().map(|platform| platform.code) == Some(PlatformErrorCode::IoWouldBlock)
}

/// Build io-not-found for one missing monitor handle.
fn monitor_not_found(
    operation: &'static str,
    handle: resource::InputMonitorHandle,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("input monitor handle {} not found", handle.0.0),
    ))
    .boxed()
}

/// Validate that one monitor handle points to an input-monitor resource.
fn validate_monitor_handle(
    context: &RuntimeCallContext,
    handle: resource::InputMonitorHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate monitor resource kind
    let valid = context.runtime().resources.with_entry(handle.0, |entry| {
        entry.kind == ResourceKind::Input
            && entry.label.as_deref() == Some(INPUT_MONITOR_RESOURCE_LABEL)
            && entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<WindowsInputMonitorBinding>())
                .is_some()
    });
    if !matches!(valid, Some(true)) {
        return Err(monitor_not_found(operation, handle));
    }

    Ok(())
}

/// Acquire one singleton monitor stream lane.
fn acquire_monitor_stream(operation: &'static str) -> RuntimeResult<()> {
    let mut current = WINDOWS_MONITOR_STREAMS.load(Ordering::Acquire);
    loop {
        if current > 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoWouldBlock),
                None,
                None,
                Some(operation.to_string()),
                None,
                "input monitor stream is already open".to_string(),
            ))
            .boxed());
        }

        match WINDOWS_MONITOR_STREAMS.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return Ok(()),
            Err(next) => current = next,
        }
    }
}

/// Allocate one sequence number for one monitor handle.
fn next_monitor_sequence(
    context: &RuntimeCallContext,
    handle: resource::InputMonitorHandle,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let sequence = context
        .runtime()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::Input {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_MONITOR_RESOURCE_LABEL) {
                return None;
            }

            let binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<WindowsInputMonitorBinding>())?;
            let next = binding.next_sequence;
            binding.next_sequence = binding.next_sequence.saturating_add(1);
            Some(next)
        });

    match sequence.flatten() {
        Some(sequence) => Ok(sequence),
        None => Err(monitor_not_found(operation, handle)),
    }
}

/// Read one input event.
///
/// Read one pending input event from one opened device stream.
/// Per-device streams report control and motion events for that device and exclude global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses evdev event reads on Linux, global-session state polling on macOS, terminal-byte event reads on other Unix hosts, and ReadConsoleInputW queue reads or raw-state polling on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_read(
    context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one event from the selected input backend
    let event = input_core::read_windows_event(context, handle, false, "destack.input.event.read")?;

    // write event payload to output
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Close one global input event monitor.
///
/// Close one opened monitor stream and release host subscription resources.
/// Pending unread monitor events are discarded.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_monitor_close(
    context: &RuntimeCallContext,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate monitor handle shape
    validate_monitor_handle(context, handle, "destack.input.event.monitorClose")?;

    // remove and finalize monitor resource
    let removed = context.runtime().resources.remove_and_finalize(handle.0);
    if !removed {
        return Err(monitor_not_found(
            "destack.input.event.monitorClose",
            handle,
        ));
    }

    Ok(())
}

/// Open one global input event monitor.
///
/// Open one monitor stream that reports host input topology events, including connect and disconnect.
/// Monitor streams are independent from per-device data streams.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses inotify-backed `/dev/input` monitor events on Linux with snapshot fallback when watcher setup is unavailable, global-session subscriptions on macOS, terminal-device scans on other Unix hosts, and raw-input device-change subscriptions on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_monitor_open(
    context: &RuntimeCallContext,
    out: *mut resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure raw monitor backend is active
    raw_input::ensure_service("destack.input.event.monitorOpen")?;
    acquire_monitor_stream("destack.input.event.monitorOpen")?;

    // allocate monitor handle in the resource table
    let entry = resource::ResourceEntry::new(ResourceKind::Input)
        .with_label(INPUT_MONITOR_RESOURCE_LABEL)
        .with_payload(WindowsInputMonitorBinding { next_sequence: 1 })
        .with_finalizer(WindowsMonitorFinalizer);
    let handle = resource::InputMonitorHandle(context.runtime().resources.insert(entry));

    // write handle to output
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Read one global input monitor event.
///
/// Read one pending monitor event from the global input monitor stream.
/// This stream is the canonical source for device connect and disconnect events.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses blocking reads from inotify-backed Linux monitor queues with snapshot fallback on watcherless hosts, terminal or session monitor streams on Unix hosts, and raw-input monitor queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_monitor_read(
    context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    validate_monitor_handle(context, handle, "destack.input.event.monitorRead")?;

    // read one monitor event from the raw-input service
    let mut event =
        raw_input::read_monitor_event(context, false, "destack.input.event.monitorRead")?;
    event.sequence = next_monitor_sequence(context, handle, "destack.input.event.monitorRead")?;

    // write event to output
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Poll one global input monitor event without blocking.
///
/// Poll one pending monitor event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses nonblocking reads from inotify-backed Linux monitor queues with snapshot fallback on watcherless hosts, terminal or session monitor streams on Unix hosts, and raw-input monitor queues on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_monitor_try_read(
    context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    validate_monitor_handle(context, handle, "destack.input.event.monitorTryRead")?;

    // poll one monitor event from the raw-input service
    let mut event =
        raw_input::read_monitor_event(context, true, "destack.input.event.monitorTryRead")?;
    event.sequence = next_monitor_sequence(context, handle, "destack.input.event.monitorTryRead")?;

    // write event to output
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Enable or disable exclusive device grab.
///
/// Toggle exclusive-grab mode for one input device when the host backend supports it.
/// Grabs can prevent event delivery to other clients.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where exclusive grab is not defined by host policy.
/// Uses EVIOCGRAB on Linux, returns notSupported for global-session and terminal-backed Unix input, and uses SetConsoleMode capture toggles on Windows console input.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_set_grab(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    // delegate grab control to shared windows core helpers
    input_core::set_windows_grab(context, handle, enable, "destack.input.event.setGrab")
}

/// Read one batch of input events.
///
/// Read up to `maxEvents` events from one opened device stream in one call.
/// Batch ordering matches backend delivery order and excludes global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses batched reads when supported and runtime looped reads otherwise.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_read_batch(
    context: &RuntimeCallContext,
    out: *mut NativeArray<InputEvent>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate max-events contract
    if maxevents == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxevents",
            "maxevents must be greater than zero",
        ))
        .boxed());
    }

    // read at least one event to preserve blocking readBatch semantics
    let mut events = Vec::with_capacity(maxevents as usize);
    let first =
        input_core::read_windows_event(context, handle, false, "destack.input.event.readBatch")?;
    events.push(first);

    // keep polling until queue is drained or the batch is full
    while events.len() < maxevents as usize {
        match input_core::read_windows_event(context, handle, true, "destack.input.event.readBatch")
        {
            Ok(event) => events.push(event),
            Err(error) => {
                if is_io_would_block(error.as_ref()) {
                    break;
                }

                return Err(error);
            }
        }
    }

    // write collected events to output array
    unsafe {
        *out = context.store_array(events);
    }

    Ok(())
}

/// Select event decoding mode for one input stream.
///
/// Select translated or raw decoding mode for one opened input endpoint.
/// Hosts can return notSupported when raw mode is unavailable for the selected endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses per-stream runtime mode selection on Linux evdev and macOS session backends, termios raw and cooked mode updates on Unix TTY paths, and SetConsoleMode updates on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_set_read_mode(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    // delegate read-mode selection to shared windows core helpers
    input_core::set_windows_read_mode(context, handle, mode, "destack.input.event.setReadMode")
}

/// Poll one input event without blocking.
///
/// Poll one pending input event from one opened device stream and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet that carries overflow details in code and value.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses nonblocking evdev reads on Linux, nonblocking global-session polling on macOS, nonblocking terminal-byte reads on other Unix hosts, and nonblocking console queue reads or raw-state polling on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_try_read(
    context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one event in nonblocking mode
    let event =
        input_core::read_windows_event(context, handle, true, "destack.input.event.tryRead")?;

    // write event payload to output
    unsafe {
        *out = event;
    }

    Ok(())
}
