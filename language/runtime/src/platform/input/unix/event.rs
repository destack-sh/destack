use std::collections::{HashMap, HashSet, VecDeque};
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
#[cfg(target_os = "linux")]
use super::linux as input_linux;
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(target_os = "linux")]
use crate::platform::core as core_platform;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputDeviceKind, InputEvent, InputEventAction, InputMonitorChangeEvent,
    InputMonitorConnectEvent, InputMonitorDisconnectEvent, InputMonitorEvent,
    InputMonitorEventKind, InputMonitorEventMetadata, InputReadMode,
    validation as input_validation,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
#[cfg(target_os = "linux")]
use crate::platform::resource::{ResourceFinalizer, ResourceId};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resource-table label for opened input-monitor entries.
const INPUT_MONITOR_RESOURCE_LABEL: &str = "input.monitor";
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
struct MonitorDeltaEvent {
    /// Stable runtime device identifier.
    device_id: String,
    /// Classified device kind for this topology transition.
    device_kind: InputDeviceKind,
    /// Connection state transition.
    action: InputEventAction,
}

/// Resource-table payload for one unix monitor handle.
#[derive(Debug)]
struct UnixInputMonitorBinding {
    /// Known device set from the previous poll snapshot.
    known_devices: Vec<String>,
    /// Known device kinds keyed by stable device id.
    known_device_kinds: HashMap<String, InputDeviceKind>,
    /// Stable ids keyed by Linux monitor node path.
    #[cfg(target_os = "linux")]
    known_linux_paths: HashMap<String, String>,
    /// Pending connect or disconnect events.
    pending_events: VecDeque<MonitorDeltaEvent>,
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
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate monitor resource kind and label
    let valid = binding.agent().resources.with_entry(handle.0, |entry| {
        entry.kind == ResourceKind::InputMonitor
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

/// Snapshot record for one monitor-visible device.
#[derive(Debug, Clone)]
struct MonitorDeviceSnapshot {
    /// Stable runtime device identifier.
    device_id: String,
    /// Linux node path for this monitor-visible endpoint.
    #[cfg(target_os = "linux")]
    device_path: String,
    /// Classified runtime device kind.
    device_kind: InputDeviceKind,
}

/// List monitor-visible devices as stable sorted identifiers with kinds.
fn list_monitor_devices(binding: &BindingCallContext) -> RuntimeResult<Vec<MonitorDeviceSnapshot>> {
    #[cfg(target_os = "linux")]
    {
        let _ = binding;

        // list linux event node paths and classify each visible endpoint
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

        let mut devices = Vec::new();
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

            let path = format!("{INPUT_MONITOR_LINUX_PATH}/{name}");
            if Path::new(&path).exists() {
                let device_id = input_linux::linux_runtime_device_id_for_path(&path);
                devices.push(MonitorDeviceSnapshot {
                    device_id,
                    device_path: path.clone(),
                    device_kind: input_linux::linux_device_kind_for_path(&path),
                });
            }
        }
        devices.sort_unstable_by(|left, right| left.device_id.cmp(&right.device_id));
        devices.dedup_by(|left, right| left.device_id == right.device_id);
        return Ok(devices);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let devices = input_core::list_unix_devices(binding)?;
        let mut snapshots = Vec::with_capacity(devices.len());
        for device in devices {
            let id = unsafe { device.id.as_str()? };
            snapshots.push(MonitorDeviceSnapshot {
                device_id: id.to_string(),
                device_kind: device.kind,
            });
        }
        snapshots.sort_unstable_by(|left, right| left.device_id.cmp(&right.device_id));
        snapshots.dedup_by(|left, right| left.device_id == right.device_id);
        Ok(snapshots)
    }
}

/// Queue connect and disconnect deltas from one device snapshot.
fn enqueue_monitor_delta(
    binding: &mut UnixInputMonitorBinding,
    current_devices: &[MonitorDeviceSnapshot],
) {
    // build previous and current membership sets
    let previous: HashSet<&str> = binding.known_devices.iter().map(String::as_str).collect();
    let current: HashSet<&str> = current_devices
        .iter()
        .map(|device| device.device_id.as_str())
        .collect();

    // collect newly discovered devices
    let mut connect_events = Vec::new();
    for device in current_devices {
        #[cfg(target_os = "linux")]
        {
            binding
                .known_linux_paths
                .insert(device.device_path.clone(), device.device_id.clone());
        }

        if !previous.contains(device.device_id.as_str()) {
            connect_events.push(device.clone());
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
    for device in connect_events {
        enqueue_monitor_action(
            binding,
            device.device_id,
            InputEventAction::Connect,
            Some(device.device_kind),
        );
    }

    // enqueue disconnect transitions
    for device_id in disconnect_events {
        #[cfg(target_os = "linux")]
        {
            binding
                .known_linux_paths
                .retain(|_, known_id| known_id != &device_id);
        }

        enqueue_monitor_action(binding, device_id, InputEventAction::Disconnect, None);
    }
}

/// Queue one monitor action and keep known-device state synchronized.
fn enqueue_monitor_action(
    binding: &mut UnixInputMonitorBinding,
    device_id: String,
    action: InputEventAction,
    device_kind: Option<InputDeviceKind>,
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
        let device_kind = device_kind.unwrap_or(InputDeviceKind::Raw);
        binding.known_devices.push(device_id.clone());
        binding.known_devices.sort_unstable();
        binding.known_devices.dedup();
        binding
            .known_device_kinds
            .insert(device_id.clone(), device_kind);

        // enqueue one monitor event packet
        binding.pending_events.push_back(MonitorDeltaEvent {
            device_id,
            device_kind,
            action,
        });
    } else if action == InputEventAction::Disconnect {
        let disconnected_kind = binding
            .known_device_kinds
            .remove(&device_id)
            .unwrap_or(InputDeviceKind::Raw);
        binding.known_devices.retain(|known| known != &device_id);

        // enqueue one monitor event packet
        binding.pending_events.push_back(MonitorDeltaEvent {
            device_id,
            device_kind: disconnected_kind,
            action,
        });
    }
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
                    let device_path = format!("{INPUT_MONITOR_LINUX_PATH}/{name}");
                    let mask = event.mask;

                    if (mask & (libc::IN_CREATE | libc::IN_MOVED_TO)) != 0 {
                        let device_kind = input_linux::linux_device_kind_for_path(&device_path);
                        let device_id = input_linux::linux_runtime_device_id_for_path(&device_path);
                        binding
                            .known_linux_paths
                            .insert(device_path.clone(), device_id.clone());

                        enqueue_monitor_action(
                            binding,
                            device_id,
                            InputEventAction::Connect,
                            Some(device_kind),
                        );
                    }

                    if (mask & (libc::IN_DELETE | libc::IN_MOVED_FROM)) != 0 {
                        let fallback_id =
                            input_linux::linux_runtime_device_id_for_path(&device_path);
                        let device_id = binding
                            .known_linux_paths
                            .remove(&device_path)
                            .unwrap_or(fallback_id);

                        enqueue_monitor_action(
                            binding,
                            device_id,
                            InputEventAction::Disconnect,
                            None,
                        );
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

/// Convert one monitor packet into one runtime monitor event.
fn monitor_kind_from_action(action: InputEventAction) -> InputMonitorEventKind {
    match action {
        InputEventAction::Connect => InputMonitorEventKind::Connect,
        InputEventAction::Disconnect => InputMonitorEventKind::Disconnect,
        _ => InputMonitorEventKind::Change,
    }
}

/// Build one monitor event payload from one topology delta.
fn build_unix_monitor_event(
    binding: &BindingCallContext,
    timestamp_ns: u64,
    sequence: u64,
    device_id: &str,
    device_kind: InputDeviceKind,
    action: InputEventAction,
) -> InputMonitorEvent {
    let kind = monitor_kind_from_action(action);
    let connected = !matches!(kind, InputMonitorEventKind::Disconnect);
    let metadata = InputMonitorEventMetadata {
        timestamp_ns,
        sequence,
        device_id: binding.store_string(device_id),
        device_kind,
        connected,
    };

    match kind {
        InputMonitorEventKind::Connect => {
            InputMonitorEvent::InputMonitorConnectEvent(InputMonitorConnectEvent {
                kind: binding.store_string("connect"),
                metadata,
            })
        }
        InputMonitorEventKind::Disconnect => {
            InputMonitorEvent::InputMonitorDisconnectEvent(InputMonitorDisconnectEvent {
                kind: binding.store_string("disconnect"),
                metadata,
            })
        }
        InputMonitorEventKind::Change => {
            InputMonitorEvent::InputMonitorChangeEvent(InputMonitorChangeEvent {
                kind: binding.store_string("change"),
                metadata,
            })
        }
    }
}

/// Convert one monitor packet into one runtime monitor event.
fn monitor_event_to_output(
    binding: &BindingCallContext,
    event: MonitorDeltaEvent,
    sequence: u64,
) -> InputMonitorEvent {
    // stamp event payload with one monotonic timestamp
    let timestamp = input_core::monotonic_timestamp_ns();

    build_unix_monitor_event(
        binding,
        timestamp,
        sequence,
        &event.device_id,
        event.device_kind,
        event.action,
    )
}

/// Poll monitor state until one event is available or would-block.
fn poll_monitor_event(
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputMonitorEvent> {
    loop {
        // drain watcher queues and attempt one queue pop
        let next = binding.agent().resources.with_entry_mut(handle.0, |entry| {
            if entry.kind != ResourceKind::InputMonitor {
                return None;
            }

            if entry.label.as_deref() != Some(INPUT_MONITOR_RESOURCE_LABEL) {
                return None;
            }

            let resolved_binding = entry
                .payload
                .as_mut()
                .and_then(|payload| payload.downcast_mut::<UnixInputMonitorBinding>())?;

            #[cfg(target_os = "linux")]
            if let Err(error) = drain_linux_monitor_watch(resolved_binding) {
                return Some(Err(error));
            }

            // request one fallback snapshot only when no watch backend is available
            #[cfg(target_os = "linux")]
            let requires_snapshot = resolved_binding.pending_events.is_empty()
                && resolved_binding.watch_descriptor.is_none();
            #[cfg(not(target_os = "linux"))]
            let requires_snapshot = resolved_binding.pending_events.is_empty();

            #[cfg(target_os = "linux")]
            let watch_descriptor = resolved_binding.watch_descriptor;
            #[cfg(not(target_os = "linux"))]
            let watch_descriptor: Option<i32> = None;

            let event = resolved_binding.pending_events.pop_front();
            let event = event.map(|event| {
                let sequence = resolved_binding.next_sequence;
                resolved_binding.next_sequence = resolved_binding.next_sequence.saturating_add(1);
                (event, sequence)
            });
            Some(Ok((event, watch_descriptor, requires_snapshot)))
        });

        match next {
            // return one queued monitor event
            Some(Some(Ok((Some((event, sequence)), _, _)))) => {
                return Ok(monitor_event_to_output(binding, event, sequence));
            }

            // when no watch backend exists, rescan device ids and enqueue topology deltas
            Some(Some(Ok((None, _, true)))) => {
                let current_devices = list_monitor_devices(binding)?;
                let next = binding.agent().resources.with_entry_mut(handle.0, |entry| {
                    if entry.kind != ResourceKind::InputMonitor {
                        return None;
                    }

                    if entry.label.as_deref() != Some(INPUT_MONITOR_RESOURCE_LABEL) {
                        return None;
                    }

                    let resolved_binding = entry
                        .payload
                        .as_mut()
                        .and_then(|payload| payload.downcast_mut::<UnixInputMonitorBinding>())?;
                    enqueue_monitor_delta(resolved_binding, &current_devices);
                    let event = resolved_binding.pending_events.pop_front();
                    let event = event.map(|event| {
                        let sequence = resolved_binding.next_sequence;
                        resolved_binding.next_sequence =
                            resolved_binding.next_sequence.saturating_add(1);
                        (event, sequence)
                    });
                    Some(event)
                });

                match next {
                    Some(Some(Some((event, sequence)))) => {
                        return Ok(monitor_event_to_output(binding, event, sequence));
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
/// Queue pressure can report one device cancel packet through the typed payload.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses evdev event reads on Linux.
/// Uses event-tap queue reads on macOS.
/// Uses terminal-byte event reads on other Unix hosts.
/// Uses `ReadConsoleInputW` queue reads or raw-state polling on Windows.
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
    binding: &BindingCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve resolved_binding and read one event
    let resolved_binding =
        input_core::resolve_unix_input_binding(binding, handle, "destack.input.event.read")?;
    let event = input_core::read_unix_event(
        binding,
        &resolved_binding,
        handle,
        false,
        "destack.input.event.read",
    )?;

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
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate monitor handle shape
    validate_monitor_handle(binding, handle, "destack.input.event.monitorClose")?;

    // remove and finalize monitor resource
    let removed = binding.agent().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    );
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
/// Uses inotify-backed `/dev/input` monitor events on Linux.
/// Falls back to snapshot scans on Linux when watcher setup is unavailable.
/// Uses session and terminal-device scans on macOS and other Unix hosts.
/// Uses raw-input device-change subscriptions on Windows.
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
    binding: &BindingCallContext,
    out: *mut resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // initialize one watcher backend before taking the first topology snapshot
    #[cfg(target_os = "linux")]
    let watch_descriptor = open_linux_monitor_watch()?;

    // create one monitor payload from current device snapshot
    let known_devices = list_monitor_devices(binding)?;
    let mut known_device_ids = Vec::with_capacity(known_devices.len());
    let mut known_device_kinds = HashMap::with_capacity(known_devices.len());
    #[cfg(target_os = "linux")]
    let mut known_linux_paths = HashMap::with_capacity(known_devices.len());
    for device in known_devices {
        #[cfg(target_os = "linux")]
        {
            known_linux_paths.insert(device.device_path.clone(), device.device_id.clone());
        }

        known_device_ids.push(device.device_id.clone());
        known_device_kinds.insert(device.device_id, device.device_kind);
    }
    #[cfg(target_os = "linux")]
    let mut resolved_binding = UnixInputMonitorBinding {
        known_devices: known_device_ids,
        known_device_kinds,
        known_linux_paths,
        pending_events: VecDeque::new(),
        next_sequence: 1,
        watch_descriptor,
    };

    #[cfg(not(target_os = "linux"))]
    let resolved_binding = UnixInputMonitorBinding {
        known_devices: known_device_ids,
        known_device_kinds,
        pending_events: VecDeque::new(),
        next_sequence: 1,
    };

    // drain watcher-delivered events queued during snapshot creation
    #[cfg(target_os = "linux")]
    if watch_descriptor.is_some() {
        drain_linux_monitor_watch(&mut resolved_binding)?;
    }

    let entry = ResourceEntry::new(ResourceKind::InputMonitor)
        .with_label(INPUT_MONITOR_RESOURCE_LABEL)
        .with_payload(resolved_binding);
    #[cfg(target_os = "linux")]
    let entry = if let Some(descriptor) = watch_descriptor {
        entry.with_finalizer(MonitorWatchFinalizer { fd: descriptor })
    } else {
        entry
    };
    let handle = resource::InputMonitorHandle(binding.agent().resources.insert(
        binding.world(),
        entry,
        Some(binding.engine()),
    ));

    // write monitor handle to output
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Read one global input monitor event.
///
/// Read one pending monitor event from the global input monitor stream.
/// This stream is the canonical source for device connect, disconnect, and metadata-change events.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one global monitor stream.
/// Uses blocking reads from inotify-backed Linux monitor queues.
/// Falls back to snapshot scans on Linux when watcher setup is unavailable.
/// Uses terminal or session monitor streams on Unix hosts.
/// Uses raw-input monitor queues on Windows.
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
    binding: &BindingCallContext,
    out: *mut InputMonitorEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate monitor handle and block for one event
    validate_monitor_handle(binding, handle, "destack.input.event.monitorRead")?;

    let event = poll_monitor_event(binding, handle, false, "destack.input.event.monitorRead")?;

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
/// Uses nonblocking reads from inotify-backed Linux monitor queues.
/// Falls back to snapshot scans on Linux when watcher setup is unavailable.
/// Uses terminal or session monitor streams on Unix hosts.
/// Uses raw-input monitor queues on Windows.
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
    binding: &BindingCallContext,
    out: *mut InputMonitorEvent,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate monitor handle and poll once
    validate_monitor_handle(binding, handle, "destack.input.event.monitorTryRead")?;

    let event = poll_monitor_event(binding, handle, true, "destack.input.event.monitorTryRead")?;

    // write event payload to output
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Enable or disable exclusive device grab.
///
/// Toggle exclusive-grab mode for one input device when the host backend supports it.
/// This is one device-wide exclusivity control and is distinct from pointer confinement or locking modes.
/// Grabs can prevent event delivery to other clients.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where exclusive grab is not defined by host policy.
/// Uses `EVIOCGRAB` on Linux.
/// Returns `notSupported` for global-session and terminal-backed Unix input.
/// Uses `SetConsoleMode` capture toggles on Windows console input.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.grab`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_set_exclusive_grab(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    // resolve resolved_binding and apply backend-specific grab semantics
    let resolved_binding = input_core::resolve_unix_input_binding(
        binding,
        handle,
        "destack.input.event.setExclusiveGrab",
    )?;
    input_core::set_unix_grab(
        resolved_binding.descriptor,
        resolved_binding.backend,
        enable,
    )
}

/// Read one batch of input events.
///
/// Read up to `maxEvents` events from one opened device stream in one call.
/// Batch ordering matches backend delivery order and excludes global device topology events.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet through the typed payload.
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
    binding: &BindingCallContext,
    out: *mut NativeArray<InputEvent>,
    handle: resource::InputDeviceHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate max-events contract
    let maxevents = input_validation::validate_read_batch_maxevents(maxevents)?;

    // resolve resolved_binding and read the first blocking event
    let resolved_binding =
        input_core::resolve_unix_input_binding(binding, handle, "destack.input.event.readBatch")?;
    let mut events = Vec::with_capacity(maxevents);
    let first = input_core::read_unix_event(
        binding,
        &resolved_binding,
        handle,
        false,
        "destack.input.event.readBatch",
    )?;
    events.push(first);

    // continue with nonblocking reads until drained or full
    while events.len() < maxevents {
        match input_core::read_unix_event(
            binding,
            &resolved_binding,
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
        *out = binding.store_array(events);
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
/// Uses per-stream runtime mode selection on Linux evdev and macOS session backends.
/// Uses termios raw and cooked mode updates on Unix TTY paths.
/// Uses `SetConsoleMode` updates on Windows.
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
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    // delegate mode updates to unix core helpers
    input_core::set_unix_read_mode(binding, handle, mode, "destack.input.event.setReadMode")
}

/// Poll one input event without blocking.
///
/// Poll one pending input event from one opened device stream and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
/// Backend framing packets are filtered from this semantic stream.
/// Queue pressure can report one device cancel packet through the typed payload.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` on hosts that do not expose one readable input backend.
/// Uses nonblocking evdev reads on Linux.
/// Uses nonblocking event-tap queue reads on macOS.
/// Uses nonblocking terminal-byte reads on other Unix hosts.
/// Uses nonblocking console queue reads or raw-state polling on Windows.
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
    binding: &BindingCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve resolved_binding and poll one nonblocking event
    let resolved_binding =
        input_core::resolve_unix_input_binding(binding, handle, "destack.input.event.tryRead")?;
    let event = input_core::read_unix_event(
        binding,
        &resolved_binding,
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
    use std::collections::{HashMap, VecDeque};

    use crate::platform::input::{InputDeviceKind, InputEventAction};

    use super::{
        MonitorDeviceSnapshot, UnixInputMonitorBinding, enqueue_monitor_action,
        enqueue_monitor_delta,
    };

    /// Build one empty monitor binding for logic tests.
    fn empty_monitor_binding() -> UnixInputMonitorBinding {
        UnixInputMonitorBinding {
            known_devices: Vec::new(),
            known_device_kinds: HashMap::new(),
            #[cfg(target_os = "linux")]
            known_linux_paths: HashMap::new(),
            pending_events: VecDeque::new(),
            next_sequence: 1,
            #[cfg(target_os = "linux")]
            watch_descriptor: None,
        }
    }

    /// Enqueue connect and disconnect deltas in stable sequence order.
    #[test]
    fn test_enqueue_monitor_delta_collects_connect_and_disconnect() {
        let mut monitor_binding = empty_monitor_binding();
        monitor_binding.known_devices = vec!["device:old".to_string()];
        monitor_binding
            .known_device_kinds
            .insert("device:old".to_string(), InputDeviceKind::Keyboard);

        let current = vec![MonitorDeviceSnapshot {
            device_id: "device:new".to_string(),
            #[cfg(target_os = "linux")]
            device_path: "/dev/input/event1".to_string(),
            device_kind: InputDeviceKind::Mouse,
        }];
        enqueue_monitor_delta(&mut monitor_binding, &current);

        let first = monitor_binding
            .pending_events
            .pop_front()
            .expect("connect event expected");
        assert_eq!(first.action, InputEventAction::Connect);
        assert_eq!(first.device_id, "device:new");
        assert_eq!(first.device_kind, InputDeviceKind::Mouse);

        let second = monitor_binding
            .pending_events
            .pop_front()
            .expect("disconnect event expected");
        assert_eq!(second.action, InputEventAction::Disconnect);
        assert_eq!(second.device_id, "device:old");
        assert_eq!(second.device_kind, InputDeviceKind::Keyboard);
    }

    /// Ignore duplicate connect and disconnect transitions.
    #[test]
    fn test_enqueue_monitor_action_ignores_duplicate_transitions() {
        let mut monitor_binding = empty_monitor_binding();
        enqueue_monitor_action(
            &mut monitor_binding,
            "device:dup".to_string(),
            InputEventAction::Connect,
            Some(InputDeviceKind::Mouse),
        );
        enqueue_monitor_action(
            &mut monitor_binding,
            "device:dup".to_string(),
            InputEventAction::Connect,
            Some(InputDeviceKind::Mouse),
        );
        assert_eq!(monitor_binding.pending_events.len(), 1);

        enqueue_monitor_action(
            &mut monitor_binding,
            "device:dup".to_string(),
            InputEventAction::Disconnect,
            None,
        );
        enqueue_monitor_action(
            &mut monitor_binding,
            "device:dup".to_string(),
            InputEventAction::Disconnect,
            None,
        );
        assert_eq!(monitor_binding.pending_events.len(), 2);
    }
}
