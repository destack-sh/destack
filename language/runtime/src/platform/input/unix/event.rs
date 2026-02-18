use std::collections::{HashSet, VecDeque};
#[cfg(target_os = "linux")]
use std::ffi::CStr;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::os::unix::io::RawFd;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::thread;
use std::time::Duration;

use super::core as input_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "linux")]
use crate::platform::core as core_platform;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{InputEvent, InputEventAction, InputEventKind, InputReadMode};
use crate::platform::resource::{ResourceEntry, ResourceKind};
#[cfg(target_os = "linux")]
use crate::platform::resource::{ResourceFinalizer, ResourceId};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Resource-table label for opened input-monitor entries.
const INPUT_MONITOR_RESOURCE_LABEL: &str = "input.monitor";
/// Empty text payload for monitor events.
const INPUT_MONITOR_EMPTY_TEXT: &str = "";
/// Linux monitor root path for event-node discovery.
#[cfg(target_os = "linux")]
const INPUT_MONITOR_LINUX_PATH: &str = "/dev/input";
/// Linux event-node prefix used for monitor filtering.
#[cfg(target_os = "linux")]
const INPUT_MONITOR_EVENT_PREFIX: &str = "event";
/// Linux inotify read-buffer size for monitor polling.
#[cfg(target_os = "linux")]
const INPUT_MONITOR_INOTIFY_BUFFER_SIZE: usize = 4096;

/// Monitor event payload queued by unix monitor polling.
#[derive(Debug, Clone)]
struct InputMonitorEvent {
    /// Stable runtime device identifier.
    device_id: String,
    /// Connection state transition.
    action: InputEventAction,
}

/// Resource-table payload for one unix monitor handle.
#[derive(Debug)]
struct UnixInputMonitorBinding {
    /// Known device set from the previous poll snapshot.
    known_devices: Vec<String>,
    /// Pending connect or disconnect events.
    pending_events: VecDeque<InputMonitorEvent>,
    /// Next per-monitor event sequence number.
    next_sequence: u64,
    /// Linux inotify descriptor used for monitor events.
    #[cfg(target_os = "linux")]
    watch_descriptor: Option<RawFd>,
}

/// Finalizer payload for one monitor watcher descriptor.
#[cfg(target_os = "linux")]
#[derive(Debug)]
struct MonitorWatchFinalizer {
    /// Linux inotify descriptor to close.
    fd: RawFd,
}

#[cfg(target_os = "linux")]
impl ResourceFinalizer for MonitorWatchFinalizer {
    /// Close the monitor watch descriptor during resource finalization.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
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
    // validate monitor resource kind and label
    let valid = context.runtime().resources.with_entry(handle.0, |entry| {
        entry.kind == ResourceKind::Input
            && entry.label.as_deref() == Some(INPUT_MONITOR_RESOURCE_LABEL)
            && entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<UnixInputMonitorBinding>())
                .is_some()
    });
    if !matches!(valid, Some(true)) {
        return Err(monitor_not_found(operation, handle));
    }

    Ok(())
}

/// List monitor-visible device identifiers as stable sorted strings.
fn list_monitor_device_ids(context: &RuntimeCallContext) -> RuntimeResult<Vec<String>> {
    #[cfg(target_os = "linux")]
    {
        let _ = context;

        // list linux event node paths without probing full device metadata
        let entries = match fs::read_dir(INPUT_MONITOR_LINUX_PATH) {
            Ok(entries) => entries,
            Err(error) => {
                if error.kind() == std::io::ErrorKind::NotFound {
                    return Ok(Vec::new());
                }

                return Err(RuntimeError::from(PlatformError::io_with(
                    None,
                    None,
                    error.raw_os_error(),
                    Some("read_dir".to_string()),
                    Some(INPUT_MONITOR_LINUX_PATH.to_string()),
                    format!("failed to read {INPUT_MONITOR_LINUX_PATH}: {error}"),
                ))
                .boxed());
            }
        };

        let mut ids = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                RuntimeError::from(PlatformError::io_with(
                    None,
                    None,
                    error.raw_os_error(),
                    Some("read_dir".to_string()),
                    Some(INPUT_MONITOR_LINUX_PATH.to_string()),
                    format!("failed to read directory entry: {error}"),
                ))
                .boxed()
            })?;

            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with(INPUT_MONITOR_EVENT_PREFIX) {
                continue;
            }

            let id = format!("{INPUT_MONITOR_LINUX_PATH}/{name}");
            if Path::new(&id).exists() {
                ids.push(id);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        return Ok(ids);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let devices = input_core::list_unix_devices(context)?;
        let mut ids = Vec::with_capacity(devices.len());
        for device in devices {
            let id = unsafe { device.id.as_str()? };
            ids.push(id.to_string());
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }
}

/// Queue connect and disconnect deltas from one device snapshot.
fn enqueue_monitor_delta(binding: &mut UnixInputMonitorBinding, current_devices: &[String]) {
    // build previous and current membership sets
    let previous: HashSet<&str> = binding.known_devices.iter().map(String::as_str).collect();
    let current: HashSet<&str> = current_devices.iter().map(String::as_str).collect();

    // collect newly discovered devices
    let mut connect_events = Vec::new();
    for device_id in current_devices {
        if !previous.contains(device_id.as_str()) {
            connect_events.push(device_id.clone());
        }
    }

    // collect devices that disappeared from the snapshot
    let mut disconnect_events = Vec::new();
    for device_id in &binding.known_devices {
        if !current.contains(device_id.as_str()) {
            disconnect_events.push(device_id.clone());
        }
    }

    // enqueue connect transitions
    for device_id in connect_events {
        enqueue_monitor_action(binding, device_id, InputEventAction::Connect);
    }

    // enqueue disconnect transitions
    for device_id in disconnect_events {
        enqueue_monitor_action(binding, device_id, InputEventAction::Disconnect);
    }
}

/// Queue one monitor action and keep known-device state synchronized.
fn enqueue_monitor_action(
    binding: &mut UnixInputMonitorBinding,
    device_id: String,
    action: InputEventAction,
) {
    // skip duplicate connect transitions
    if action == InputEventAction::Connect && binding.known_devices.contains(&device_id) {
        return;
    }

    // skip duplicate disconnect transitions
    if action == InputEventAction::Disconnect && !binding.known_devices.contains(&device_id) {
        return;
    }

    // apply connect and disconnect membership changes
    if action == InputEventAction::Connect {
        binding.known_devices.push(device_id.clone());
        binding.known_devices.sort_unstable();
        binding.known_devices.dedup();
    } else if action == InputEventAction::Disconnect {
        binding.known_devices.retain(|known| known != &device_id);
    }

    // enqueue one monitor event packet
    binding
        .pending_events
        .push_back(InputMonitorEvent { device_id, action });
}

/// Open one inotify watcher for input monitor events.
#[cfg(target_os = "linux")]
fn open_linux_monitor_watch() -> RuntimeResult<Option<RawFd>> {
    // create one nonblocking inotify descriptor
    let descriptor = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
    if descriptor < 0 {
        return Err(core_platform::io_error("inotify_init1", None));
    }

    // register watch events for node creation and removal
    let mask = libc::IN_CREATE | libc::IN_DELETE | libc::IN_MOVED_TO | libc::IN_MOVED_FROM;
    let path_cstring = std::ffi::CString::new(INPUT_MONITOR_LINUX_PATH).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "input monitor path contains nul byte",
        ))
        .boxed()
    })?;
    let watch = unsafe { libc::inotify_add_watch(descriptor, path_cstring.as_ptr(), mask) };
    if watch < 0 {
        let errno = core_platform::get_errno();
        unsafe {
            libc::close(descriptor);
        }

        if errno == libc::ENOENT
            || errno == libc::ENOTDIR
            || errno == libc::ENOSYS
            || errno == libc::EACCES
            || errno == libc::EPERM
        {
            return Ok(None);
        }

        return Err(core_platform::io_error(
            "inotify_add_watch",
            Some(INPUT_MONITOR_LINUX_PATH),
        ));
    }

    Ok(Some(descriptor))
}

/// Drain queued inotify events into monitor events.
#[cfg(target_os = "linux")]
fn drain_linux_monitor_watch(binding: &mut UnixInputMonitorBinding) -> RuntimeResult<()> {
    let Some(descriptor) = binding.watch_descriptor else {
        return Ok(());
    };

    let mut buffer = [0u8; INPUT_MONITOR_INOTIFY_BUFFER_SIZE];
    loop {
        // read one chunk of inotify events
        let read_status = unsafe {
            libc::read(
                descriptor,
                buffer.as_mut_ptr().cast::<libc::c_void>(),
                buffer.len(),
            )
        };

        // retry interrupted reads
        if read_status < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }
            if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
                break;
            }

            return Err(core_platform::io_error("read", None));
        }

        if read_status == 0 {
            break;
        }

        // parse one read chunk into individual inotify events
        let mut offset = 0usize;
        let total = read_status as usize;
        while offset + std::mem::size_of::<libc::inotify_event>() <= total {
            let event_ptr = unsafe { buffer.as_ptr().add(offset).cast::<libc::inotify_event>() };
            let event = unsafe { std::ptr::read_unaligned(event_ptr) };
            let event_len = std::mem::size_of::<libc::inotify_event>() + event.len as usize;
            if event_len == 0 || offset + event_len > total {
                break;
            }

            if event.len > 0 {
                let name_ptr = unsafe {
                    buffer
                        .as_ptr()
                        .add(offset + std::mem::size_of::<libc::inotify_event>())
                        .cast::<libc::c_char>()
                };
                let name = unsafe { CStr::from_ptr(name_ptr) }.to_string_lossy();
                if name.starts_with(INPUT_MONITOR_EVENT_PREFIX) {
                    let device_id = format!("{INPUT_MONITOR_LINUX_PATH}/{name}");
                    let mask = event.mask;

                    if (mask & (libc::IN_CREATE | libc::IN_MOVED_TO)) != 0 {
                        enqueue_monitor_action(
                            binding,
                            device_id.clone(),
                            InputEventAction::Connect,
                        );
                    }

                    if (mask & (libc::IN_DELETE | libc::IN_MOVED_FROM)) != 0 {
                        enqueue_monitor_action(binding, device_id, InputEventAction::Disconnect);
                    }
                }
            }

            offset += event_len;
        }
    }

    Ok(())
}

/// Block until one inotify descriptor reports monitor activity.
#[cfg(target_os = "linux")]
fn wait_for_monitor_watch_event(descriptor: RawFd) -> RuntimeResult<()> {
    let mut pollfd = libc::pollfd {
        fd: descriptor,
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        let status = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, -1) };
        if status > 0 {
            return Ok(());
        }
        if status == 0 {
            continue;
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        return Err(core_platform::io_error("poll", None));
    }
}

/// Convert one monitor packet into one runtime input event.
fn monitor_event_to_input_event(
    context: &RuntimeCallContext,
    event: InputMonitorEvent,
    code: u32,
    sequence: u64,
) -> InputEvent {
    // stamp event payload with one monotonic timestamp
    let timestamp = input_core::monotonic_timestamp_ns();

    InputEvent {
        kind: InputEventKind::Device,
        timestamp_ns: timestamp,
        sequence,
        device_id: context.store_string(&event.device_id),
        action: event.action,
        code,
        scan_code: code,
        value: if event.action == InputEventAction::Connect {
            1
        } else {
            0
        },
        x: 0.0,
        y: 0.0,
        wheel_x: 0.0,
        wheel_y: 0.0,
        modifiers: 0,
        repeat: false,
        text: context.store_string(INPUT_MONITOR_EMPTY_TEXT),
    }
}

/// Poll monitor state until one event is available or would-block.
fn poll_monitor_event(
    context: &RuntimeCallContext,
    handle: resource::InputMonitorHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputEvent> {
    loop {
        // drain watcher queues and attempt one queue pop
        let next = context
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
                    .and_then(|payload| payload.downcast_mut::<UnixInputMonitorBinding>())?;

                #[cfg(target_os = "linux")]
                if let Err(error) = drain_linux_monitor_watch(binding) {
                    return Some(Err(error));
                }

                // request one fallback snapshot only when no watch backend is available
                #[cfg(target_os = "linux")]
                let requires_snapshot =
                    binding.pending_events.is_empty() && binding.watch_descriptor.is_none();
                #[cfg(not(target_os = "linux"))]
                let requires_snapshot = binding.pending_events.is_empty();

                #[cfg(target_os = "linux")]
                let watch_descriptor = binding.watch_descriptor;
                #[cfg(not(target_os = "linux"))]
                let watch_descriptor: Option<i32> = None;

                let event = binding.pending_events.pop_front();
                let event = event.map(|event| {
                    let sequence = binding.next_sequence;
                    binding.next_sequence = binding.next_sequence.saturating_add(1);
                    (event, sequence)
                });
                Some(Ok((event, watch_descriptor, requires_snapshot)))
            });

        match next {
            // return one queued monitor event
            Some(Some(Ok((Some((event, sequence)), _, _)))) => {
                return Ok(monitor_event_to_input_event(context, event, 0, sequence));
            }

            // when no watch backend exists, rescan device ids and enqueue topology deltas
            Some(Some(Ok((None, _, true)))) => {
                let current_devices = list_monitor_device_ids(context)?;
                let next = context
                    .runtime()
                    .resources
                    .with_entry_mut(handle.0, |entry| {
                        if entry.kind != ResourceKind::Input {
                            return None;
                        }

                        if entry.label.as_deref() != Some(INPUT_MONITOR_RESOURCE_LABEL) {
                            return None;
                        }

                        let binding = entry.payload.as_mut().and_then(|payload| {
                            payload.downcast_mut::<UnixInputMonitorBinding>()
                        })?;
                        enqueue_monitor_delta(binding, &current_devices);
                        let event = binding.pending_events.pop_front();
                        let event = event.map(|event| {
                            let sequence = binding.next_sequence;
                            binding.next_sequence = binding.next_sequence.saturating_add(1);
                            (event, sequence)
                        });
                        Some(event)
                    });

                match next {
                    Some(Some(Some((event, sequence)))) => {
                        return Ok(monitor_event_to_input_event(context, event, 0, sequence));
                    }
                    Some(Some(None)) => {
                        if nonblocking {
                            return Err(RuntimeError::from(PlatformError::io_with(
                                Some(PlatformErrorCode::IoWouldBlock),
                                None,
                                None,
                                Some(operation.to_string()),
                                None,
                                "input monitor queue is empty",
                            ))
                            .boxed());
                        }

                        // avoid hot-spin when no topology delta is available
                        thread::sleep(Duration::from_millis(8));
                    }
                    Some(None) | None => return Err(monitor_not_found(operation, handle)),
                }
            }

            // nonblocking calls stop when no pending event is available
            Some(Some(Ok((None, _, _)))) if nonblocking => {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoWouldBlock),
                    None,
                    None,
                    Some(operation.to_string()),
                    None,
                    "input monitor queue is empty",
                ))
                .boxed());
            }
            // blocking calls wait briefly and poll again
            Some(Some(Ok((None, watch_descriptor, _)))) => {
                #[cfg(target_os = "linux")]
                if let Some(watch_descriptor) = watch_descriptor {
                    wait_for_monitor_watch_event(watch_descriptor)?;
                } else {
                    thread::sleep(Duration::from_millis(8));
                }

                #[cfg(not(target_os = "linux"))]
                {
                    let _ = watch_descriptor;
                    thread::sleep(Duration::from_millis(8));
                }
            }
            // bubble watcher failures
            Some(Some(Err(error))) => return Err(error),
            // return not-found when monitor payload cannot be resolved
            Some(None) => return Err(monitor_not_found(operation, handle)),
            // return not-found when monitor handle is invalid
            None => return Err(monitor_not_found(operation, handle)),
        }
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
/// Uses evdev event reads on Linux, event-tap queue reads on macOS, terminal-byte event reads on other Unix hosts, and ReadConsoleInputW queue reads or raw-state polling on Windows.
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

    // resolve binding and read one event
    let binding =
        input_core::resolve_unix_input_binding(context, handle, "destack.input.event.read")?;
    let event =
        input_core::read_unix_event(context, &binding, handle, false, "destack.input.event.read")?;

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
/// Uses inotify-backed `/dev/input` monitor events on Linux with snapshot fallback when watcher setup is unavailable, session and terminal-device scans on macOS and other Unix hosts, and raw-input device-change subscriptions on Windows.
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

    // create one monitor payload from current device snapshot
    let known_devices = list_monitor_device_ids(context)?;
    #[cfg(target_os = "linux")]
    let watch_descriptor = open_linux_monitor_watch()?;

    let entry = ResourceEntry::new(ResourceKind::Input)
        .with_label(INPUT_MONITOR_RESOURCE_LABEL)
        .with_payload(UnixInputMonitorBinding {
            known_devices,
            pending_events: VecDeque::new(),
            next_sequence: 1,
            #[cfg(target_os = "linux")]
            watch_descriptor,
        });
    #[cfg(target_os = "linux")]
    let entry = if let Some(descriptor) = watch_descriptor {
        entry.with_finalizer(MonitorWatchFinalizer { fd: descriptor })
    } else {
        entry
    };
    let handle = resource::InputMonitorHandle(context.runtime().resources.insert(entry));

    // write monitor handle to output
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

    // validate monitor handle and block for one event
    validate_monitor_handle(context, handle, "destack.input.event.monitorRead")?;

    let event = poll_monitor_event(context, handle, false, "destack.input.event.monitorRead")?;

    // write event payload to output
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

    // validate monitor handle and poll once
    validate_monitor_handle(context, handle, "destack.input.event.monitorTryRead")?;

    let event = poll_monitor_event(context, handle, true, "destack.input.event.monitorTryRead")?;

    // write event payload to output
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
    // resolve binding and apply backend-specific grab semantics
    let binding =
        input_core::resolve_unix_input_binding(context, handle, "destack.input.event.setGrab")?;
    input_core::set_unix_grab(binding.descriptor, binding.backend, enable)
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

    // resolve binding and read the first blocking event
    let binding =
        input_core::resolve_unix_input_binding(context, handle, "destack.input.event.readBatch")?;
    let mut events = Vec::with_capacity(maxevents as usize);
    let first = input_core::read_unix_event(
        context,
        &binding,
        handle,
        false,
        "destack.input.event.readBatch",
    )?;
    events.push(first);

    // continue with nonblocking reads until drained or full
    while events.len() < maxevents as usize {
        match input_core::read_unix_event(
            context,
            &binding,
            handle,
            true,
            "destack.input.event.readBatch",
        ) {
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
    // delegate mode updates to unix core helpers
    input_core::set_unix_read_mode(context, handle, mode, "destack.input.event.setReadMode")
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
/// Uses nonblocking evdev reads on Linux, nonblocking event-tap queue reads on macOS, nonblocking terminal-byte reads on other Unix hosts, and nonblocking console queue reads or raw-state polling on Windows.
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

    // resolve binding and poll one nonblocking event
    let binding =
        input_core::resolve_unix_input_binding(context, handle, "destack.input.event.tryRead")?;
    let event = input_core::read_unix_event(
        context,
        &binding,
        handle,
        true,
        "destack.input.event.tryRead",
    )?;

    // write event payload to output
    unsafe {
        *out = event;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one empty monitor binding for logic tests.
    fn empty_monitor_binding() -> UnixInputMonitorBinding {
        UnixInputMonitorBinding {
            known_devices: Vec::new(),
            pending_events: VecDeque::new(),
            next_sequence: 1,
            #[cfg(target_os = "linux")]
            watch_descriptor: None,
        }
    }

    /// Enqueue connect and disconnect deltas in stable sequence order.
    #[test]
    fn test_enqueue_monitor_delta_collects_connect_and_disconnect() {
        let mut binding = empty_monitor_binding();
        binding.known_devices = vec!["/dev/input/event0".to_string()];

        let current = vec!["/dev/input/event1".to_string()];
        enqueue_monitor_delta(&mut binding, &current);

        let first = binding
            .pending_events
            .pop_front()
            .expect("connect event expected");
        assert_eq!(first.action, InputEventAction::Connect);
        assert_eq!(first.device_id, "/dev/input/event1");

        let second = binding
            .pending_events
            .pop_front()
            .expect("disconnect event expected");
        assert_eq!(second.action, InputEventAction::Disconnect);
        assert_eq!(second.device_id, "/dev/input/event0");
    }

    /// Ignore duplicate connect and disconnect transitions.
    #[test]
    fn test_enqueue_monitor_action_ignores_duplicate_transitions() {
        let mut binding = empty_monitor_binding();
        enqueue_monitor_action(
            &mut binding,
            "/dev/input/event2".to_string(),
            InputEventAction::Connect,
        );
        enqueue_monitor_action(
            &mut binding,
            "/dev/input/event2".to_string(),
            InputEventAction::Connect,
        );
        assert_eq!(binding.pending_events.len(), 1);

        enqueue_monitor_action(
            &mut binding,
            "/dev/input/event2".to_string(),
            InputEventAction::Disconnect,
        );
        enqueue_monitor_action(
            &mut binding,
            "/dev/input/event2".to_string(),
            InputEventAction::Disconnect,
        );
        assert_eq!(binding.pending_events.len(), 2);
    }
}
