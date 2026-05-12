use std::collections::{HashMap, HashSet, VecDeque};
#[cfg(target_os = "linux")]
use std::ffi::CStr;
#[cfg(not(target_os = "linux"))]
use std::ffi::CString;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::os::unix::io::RawFd;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(unix)]
use std::sync::{Arc, Condvar, Mutex};
#[cfg(target_os = "linux")]
use std::thread;
use std::time::Duration;
#[cfg(target_os = "linux")]
use std::time::Instant;

use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;
#[cfg(target_os = "macos")]
use super::macos as input_macos;
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::core as core_platform;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputDeviceKind, InputEvent, InputEventAction, InputMonitorChangeEvent,
    InputMonitorConnectEvent, InputMonitorDisconnectEvent, InputMonitorEvent,
    InputMonitorEventMetadata, InputReadMode, validation as input_validation,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, PlatformError, resource};
use crate::runtime::BindingCallContext;
#[cfg(unix)]
use crate::runtime::WorkerId;
use crate::runtime::service::ProcessSubscriberRegistry;
#[cfg(unix)]
use crate::runtime::service::Service;
#[cfg(unix)]
use crate::runtime::service::executor::periodic::{PeriodicTaskHandle, open_periodic_task};
#[cfg(target_os = "linux")]
use crate::runtime::start_with_policy;
#[cfg(unix)]
use crate::runtime::{ExecutionMode, ExecutionPolicy};

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
/// Maximum retained monitor topology events per worker runtime.
#[cfg(unix)]
const INPUT_MONITOR_QUEUE_LIMIT: usize = 1024;
/// Poll timeout for one native Linux monitor wait.
#[cfg(target_os = "linux")]
const INPUT_MONITOR_NATIVE_WAIT: Duration = Duration::from_millis(250);
/// Poll interval for one synthetic Unix monitor refresh.
#[cfg(unix)]
const INPUT_MONITOR_SYNTHETIC_INTERVAL: Duration = Duration::from_millis(250);

/// One sequenced monitor topology event delivered by the shared unix monitor service.
#[derive(Debug, Clone)]
struct SequencedMonitorDeltaEvent {
    /// Shared monotonic queue sequence for this topology event.
    sequence: u64,
    /// Monitor payload delivered to per-worker runtimes.
    event: MonitorDeltaEvent,
}

/// Shared queue state for one worker-owned unix monitor runtime.
#[derive(Debug, Default)]
struct UnixInputMonitorQueueState {
    /// Retained monitor topology events for this worker.
    events: VecDeque<SequencedMonitorDeltaEvent>,
    /// Next shared sequence number to assign.
    next_sequence: u64,
}

/// One worker-owned unix monitor runtime state.
#[derive(Debug)]
pub(crate) struct UnixInputMonitorRuntimeState {
    /// Shared event queue for this worker.
    queue: Mutex<UnixInputMonitorQueueState>,
    /// Wake handle for blocking monitor reads.
    wake: Condvar,
    /// Whether teardown finalization was already registered.
    finalizer_registered: AtomicBool,
    /// Whether this runtime was already attached to the shared unix monitor service.
    service_registered: AtomicBool,
}

impl UnixInputMonitorRuntimeState {
    /// Build one worker-owned unix monitor runtime state.
    pub(crate) fn new(_worker_id: WorkerId) -> Self {
        Self {
            queue: Mutex::new(UnixInputMonitorQueueState::default()),
            wake: Condvar::new(),
            finalizer_registered: AtomicBool::new(false),
            service_registered: AtomicBool::new(false),
        }
    }
}

/// One owned worker thread for the shared unix monitor service.
#[derive(Debug)]
enum UnixInputMonitorWorker {
    /// One native inotify-backed worker thread.
    #[cfg(target_os = "linux")]
    Native {
        /// Stop signal for the shared watcher thread.
        stop: Arc<AtomicBool>,
        /// Join handle for deterministic worker teardown.
        handle: Option<thread::JoinHandle<()>>,
    },
    /// One synthetic snapshot polling worker.
    Synthetic {
        /// Registered periodic task.
        task: Option<PeriodicTaskHandle>,
    },
}

/// One process-global unix input monitor service.
#[derive(Debug)]
pub(crate) struct UnixInputMonitorService {
    /// Registered unix monitor runtimes keyed by owning worker.
    runtimes: Mutex<ProcessSubscriberRegistry<WorkerId, UnixInputMonitorRuntimeState>>,
    /// Shared monitor worker when the host supports one.
    worker: Mutex<Option<UnixInputMonitorWorker>>,
    /// Whether the shared monitor worker is currently running.
    worker_running: AtomicBool,
}

impl UnixInputMonitorService {
    /// Build one empty unix input monitor service.
    fn new() -> Self {
        Self {
            runtimes: Mutex::new(ProcessSubscriberRegistry::default()),
            worker: Mutex::new(None),
            worker_running: AtomicBool::new(false),
        }
    }

    /// Register one worker-local unix monitor runtime.
    fn register_runtime(
        &self,
        worker_id: WorkerId,
        runtime_state: &Arc<UnixInputMonitorRuntimeState>,
    ) {
        let mut runtimes = self
            .runtimes
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        runtimes.register(worker_id, runtime_state);
    }

    /// Unregister one worker-local unix monitor runtime.
    fn unregister_runtime(&self, worker_id: WorkerId) {
        let should_shutdown = {
            let mut runtimes = self
                .runtimes
                .lock()
                .unwrap_or_else(|error| error.into_inner());

            runtimes.unregister(worker_id);
            runtimes.is_empty()
        };

        if should_shutdown {
            self.shutdown_worker();
        }
    }

    /// Return one snapshot of the live unix monitor runtimes.
    fn runtime_states_snapshot(&self) -> Vec<Arc<UnixInputMonitorRuntimeState>> {
        let mut runtimes = self
            .runtimes
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        runtimes.snapshot()
    }

    /// Return whether the shared monitor worker is currently running.
    fn is_worker_running(&self) -> bool {
        self.worker_running.load(Ordering::Acquire)
    }

    /// Ensure one live unix monitor worker is available when supported.
    fn ensure_worker(
        self: &Arc<Self>,
        binding: &BindingCallContext,
        operation: &'static str,
    ) -> RuntimeResult<()> {
        let mut worker = self
            .worker
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // restart the worker after unexpected exits
        let requires_spawn = match worker.as_ref() {
            Some(_) => !self.is_worker_running(),
            None => true,
        };

        if !requires_spawn {
            return Ok(());
        }

        #[cfg(target_os = "linux")]
        let next_worker = match spawn_unix_monitor_worker(binding, self).map_err(|error| {
            RuntimeError::from(PlatformError::io_with(
                None,
                None,
                None,
                Some(operation.to_string()),
                None,
                format!("unix input monitor unavailable: {error}"),
            ))
            .boxed()
        })? {
            Some(worker) => worker,
            None => spawn_unix_monitor_synthetic_worker(self, operation)?,
        };

        #[cfg(not(target_os = "linux"))]
        let next_worker = {
            let _ = binding;

            spawn_unix_monitor_synthetic_worker(self, operation)?
        };

        self.worker_running.store(true, Ordering::Release);
        *worker = Some(next_worker);

        Ok(())
    }

    /// Shut down one active unix monitor worker when present.
    fn shutdown_worker(&self) {
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let Some(mut worker) = worker else {
            return;
        };

        match &mut worker {
            // stop one native watch worker
            #[cfg(target_os = "linux")]
            UnixInputMonitorWorker::Native { stop, handle } => {
                stop.store(true, Ordering::Release);

                if let Some(handle) = handle.take() {
                    let _ = handle.join();
                }
            }

            // stop one synthetic polling worker
            UnixInputMonitorWorker::Synthetic { task } => {
                let _task = task.take();
            }
        }

        self.worker_running.store(false, Ordering::Release);
        self.wake_runtimes();
    }

    /// Wake all registered unix monitor runtimes.
    fn wake_runtimes(&self) {
        let runtimes = self.runtime_states_snapshot();

        for runtime_state in &runtimes {
            runtime_state.wake.notify_all();
        }
    }
}

#[cfg(target_os = "linux")]
impl Service for UnixInputMonitorService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Loop);
}

#[cfg(not(target_os = "linux"))]
impl Service for UnixInputMonitorService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Polling);
}

/// Return one shared unix input monitor service.
pub(crate) fn unix_input_monitor_service(
    operation: &'static str,
) -> RuntimeResult<Arc<UnixInputMonitorService>> {
    UnixInputMonitorService::global(|| Ok(UnixInputMonitorService::new())).map_err(|error| {
        RuntimeError::from(PlatformError::io_with(
            None,
            None,
            None,
            Some(operation.to_string()),
            None,
            format!("failed to initialize unix input monitor service: {error}"),
        ))
        .boxed()
    })
}

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
    /// Last shared service sequence delivered to this handle.
    last_service_sequence: u64,
    /// Pending connect or disconnect events.
    pending_events: VecDeque<MonitorDeltaEvent>,
    /// Next per-monitor event sequence number.
    next_sequence: u64,
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
        format!("input monitor handle {} not found", handle.0.local_id),
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
    let valid = binding.worker().resources.with_entry(handle.0, |entry| {
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

        list_monitor_devices_snapshot()
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

/// Drain one inotify descriptor into normalized monitor delta events.
#[cfg(target_os = "linux")]
fn drain_linux_monitor_watch(
    descriptor: RawFd,
    known_linux_paths: &mut HashMap<String, String>,
    mut publish: impl FnMut(MonitorDeltaEvent),
) -> RuntimeResult<()> {
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
                        known_linux_paths.insert(device_path.clone(), device_id.clone());

                        publish(MonitorDeltaEvent {
                            device_id,
                            device_kind,
                            action: InputEventAction::Connect,
                        });
                    }

                    if (mask & (libc::IN_DELETE | libc::IN_MOVED_FROM)) != 0 {
                        let fallback_id =
                            input_linux::linux_runtime_device_id_for_path(&device_path);
                        let device_id = known_linux_paths
                            .remove(&device_path)
                            .unwrap_or(fallback_id);

                        publish(MonitorDeltaEvent {
                            device_id,
                            device_kind: InputDeviceKind::Raw,
                            action: InputEventAction::Disconnect,
                        });
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
fn wait_for_monitor_watch_event(descriptor: RawFd, timeout: Duration) -> RuntimeResult<bool> {
    let mut pollfd = libc::pollfd {
        fd: descriptor,
        events: libc::POLLIN,
        revents: 0,
    };
    let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
        core_platform::invalid_argument("timeout", "timeout overflowed host deadline")
    })?;

    loop {
        let timeout_ms = monitor_watch_timeout_ms(deadline);
        let status = unsafe { libc::poll(&mut pollfd as *mut libc::pollfd, 1, timeout_ms) };
        if status > 0 {
            return Ok(true);
        }
        if status == 0 {
            return Ok(false);
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        return Err(core_platform::io_error("poll", None));
    }
}

/// Convert one absolute deadline into one poll timeout in milliseconds.
#[cfg(target_os = "linux")]
fn monitor_watch_timeout_ms(deadline: Instant) -> i32 {
    let now = Instant::now();
    if now >= deadline {
        return 0;
    }

    let remaining = deadline.saturating_duration_since(now);
    let remaining_ms = remaining.as_nanos().div_ceil(1_000_000);

    remaining_ms.min(i32::MAX as u128) as i32
}

/// Register one unix monitor runtime teardown finalizer.
fn register_unix_monitor_runtime_finalizer(
    binding: &BindingCallContext,
    service: &Arc<UnixInputMonitorService>,
    runtime_state: &Arc<UnixInputMonitorRuntimeState>,
) {
    if runtime_state
        .finalizer_registered
        .swap(true, Ordering::AcqRel)
    {
        return;
    }

    let worker_id = binding.worker().id;
    let service = Arc::clone(service);
    binding.worker().finalizers.register(move || {
        service.unregister_runtime(worker_id);
    });
}

/// Return one worker-owned unix monitor runtime state.
fn unix_input_monitor_runtime_state(
    binding: &BindingCallContext,
) -> Arc<UnixInputMonitorRuntimeState> {
    binding
        .worker()
        .platform_state
        .input
        .unix_input_monitor_runtime_state(binding)
}

/// Register one runtime with the shared unix monitor service once.
fn ensure_unix_monitor_runtime_registration(
    binding: &BindingCallContext,
    service: &Arc<UnixInputMonitorService>,
    runtime_state: &Arc<UnixInputMonitorRuntimeState>,
) {
    if runtime_state
        .service_registered
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    register_unix_monitor_runtime_finalizer(binding, service, runtime_state);
    service.register_runtime(binding.worker().id, runtime_state);
}

/// Publish one topology event into every live unix monitor runtime.
fn publish_unix_monitor_event(service: &Arc<UnixInputMonitorService>, event: MonitorDeltaEvent) {
    let runtimes = service.runtime_states_snapshot();

    for runtime_state in &runtimes {
        let mut queue = runtime_state
            .queue
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let sequence = queue.next_sequence.saturating_add(1);
        queue.next_sequence = sequence;

        if queue.events.len() >= INPUT_MONITOR_QUEUE_LIMIT {
            queue.events.pop_front();
        }

        queue.events.push_back(SequencedMonitorDeltaEvent {
            sequence,
            event: event.clone(),
        });
        runtime_state.wake.notify_all();
    }
}

/// Publish topology deltas from one full monitor snapshot refresh.
fn publish_unix_monitor_snapshot_delta(
    service: &Arc<UnixInputMonitorService>,
    previous_devices: &mut HashMap<String, InputDeviceKind>,
    next_devices: &[MonitorDeviceSnapshot],
) {
    let mut next_device_map = HashMap::new();

    for device in next_devices {
        next_device_map.insert(device.device_id.clone(), device.device_kind);
    }

    // publish disconnects first so reconnects are ordered deterministically
    for (device_id, device_kind) in previous_devices.iter() {
        if next_device_map.contains_key(device_id) {
            continue;
        }

        publish_unix_monitor_event(
            service,
            MonitorDeltaEvent {
                device_id: device_id.clone(),
                device_kind: *device_kind,
                action: InputEventAction::Disconnect,
            },
        );
    }

    // publish connects and kind changes from the next snapshot
    for device in next_devices {
        let previous_kind = previous_devices.get(&device.device_id).copied();

        // new devices surface as connects
        if previous_kind.is_none() {
            publish_unix_monitor_event(
                service,
                MonitorDeltaEvent {
                    device_id: device.device_id.clone(),
                    device_kind: device.device_kind,
                    action: InputEventAction::Connect,
                },
            );
            continue;
        }

        // kind changes surface as explicit topology changes
        if previous_kind != Some(device.device_kind) {
            publish_unix_monitor_event(
                service,
                MonitorDeltaEvent {
                    device_id: device.device_id.clone(),
                    device_kind: device.device_kind,
                    action: InputEventAction::Update,
                },
            );
        }
    }

    *previous_devices = next_device_map;
}

/// Spawn one shared unix monitor worker when the host supports one.
#[cfg(target_os = "linux")]
fn spawn_unix_monitor_worker(
    _binding: &BindingCallContext,
    service: &Arc<UnixInputMonitorService>,
) -> Result<Option<UnixInputMonitorWorker>, String> {
    let Some(descriptor) = open_linux_monitor_watch().map_err(|error| error.to_string())? else {
        return Ok(None);
    };

    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop);
    let service = Arc::clone(service);
    let handle = start_with_policy(
        "destack-input-unix-monitor",
        "destack.input.event.monitorOpen",
        ExecutionPolicy::process(ExecutionMode::Loop),
        move || {
            let mut known_linux_paths = HashMap::new();

            // seed the disconnect map from the current device snapshot
            if let Ok(devices) = list_monitor_devices_snapshot() {
                for device in devices {
                    known_linux_paths.insert(device.device_path, device.device_id);
                }
            }

            loop {
                // stop when the shared service tears the worker down
                if stop_signal.load(Ordering::Acquire) {
                    break;
                }

                // wait for one monitor event and then publish all queued deltas
                let ready =
                    match wait_for_monitor_watch_event(descriptor, INPUT_MONITOR_NATIVE_WAIT) {
                        Ok(ready) => ready,
                        Err(_) => break,
                    };
                if !ready {
                    continue;
                }

                let drain =
                    drain_linux_monitor_watch(descriptor, &mut known_linux_paths, |event| {
                        publish_unix_monitor_event(&service, event);
                    });
                if drain.is_err() {
                    break;
                }
            }

            unsafe {
                libc::close(descriptor);
            }

            service.worker_running.store(false, Ordering::Release);
            service.wake_runtimes();
        },
    )
    .map_err(|error| error.to_string())?;

    Ok(Some(UnixInputMonitorWorker::Native {
        stop,
        handle: Some(handle),
    }))
}

/// Spawn one shared synthetic unix monitor worker when no native watch exists.
fn spawn_unix_monitor_synthetic_worker(
    service: &Arc<UnixInputMonitorService>,
    operation: &'static str,
) -> RuntimeResult<UnixInputMonitorWorker> {
    let service = Arc::clone(service);
    let previous_devices = Arc::new(Mutex::new(HashMap::<String, InputDeviceKind>::new()));
    let previous_devices_for_task = Arc::clone(&previous_devices);

    // seed one initial synthetic topology snapshot
    if let Ok(devices) = list_monitor_devices_snapshot() {
        let mut previous_devices = previous_devices
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        for device in devices {
            previous_devices.insert(device.device_id, device.device_kind);
        }
    }

    // poll topology snapshots through the shared periodic executor
    let task = open_periodic_task(
        "destack-input-unix-monitor",
        UnixInputMonitorService::POLICY,
        INPUT_MONITOR_SYNTHETIC_INTERVAL,
        move || {
            let Ok(next_devices) = list_monitor_devices_snapshot() else {
                return Ok(());
            };

            let mut previous_devices = previous_devices_for_task
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            publish_unix_monitor_snapshot_delta(&service, &mut previous_devices, &next_devices);

            Ok(())
        },
    )
    .map_err(|error| {
        RuntimeError::from(PlatformError::io_with(
            None,
            None,
            None,
            Some(operation.to_string()),
            None,
            format!("failed to start unix input synthetic monitor: {error}"),
        ))
        .boxed()
    })?;

    Ok(UnixInputMonitorWorker::Synthetic { task: Some(task) })
}

/// Return one linux snapshot of monitor-visible devices without a binding context.
#[cfg(target_os = "linux")]
fn list_monitor_devices_snapshot() -> RuntimeResult<Vec<MonitorDeviceSnapshot>> {
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

    Ok(devices)
}

/// Return one terminal monitor snapshot when `/dev/tty` is available.
#[cfg(not(target_os = "linux"))]
fn terminal_monitor_device_snapshot() -> RuntimeResult<Option<MonitorDeviceSnapshot>> {
    let path = CString::new(input_core::UNIX_INPUT_TTY_PATH).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "terminal path contains nul byte",
        ))
        .boxed()
    })?;

    let descriptor = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if descriptor < 0 {
        let errno = core_platform::get_errno();
        if errno == libc::ENOENT
            || errno == libc::ENOTTY
            || errno == libc::ENXIO
            || errno == libc::EACCES
            || errno == libc::EPERM
        {
            return Ok(None);
        }

        return Err(core_platform::io_error(
            "open",
            Some(input_core::UNIX_INPUT_TTY_PATH),
        ));
    }

    unsafe {
        libc::close(descriptor);
    }

    Ok(Some(MonitorDeviceSnapshot {
        device_id: input_core::UNIX_INPUT_TTY_ID.to_string(),
        device_kind: InputDeviceKind::Keyboard,
    }))
}

/// Return one macOS snapshot of monitor-visible devices without a binding context.
#[cfg(target_os = "macos")]
fn list_monitor_devices_snapshot() -> RuntimeResult<Vec<MonitorDeviceSnapshot>> {
    let mut devices = vec![MonitorDeviceSnapshot {
        device_id: input_macos::MACOS_INPUT_SESSION_ID.to_string(),
        device_kind: InputDeviceKind::Raw,
    }];

    if let Some(tty_device) = terminal_monitor_device_snapshot()? {
        devices.push(tty_device);
    }

    Ok(devices)
}

/// Return one generic Unix snapshot of monitor-visible devices without a binding context.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn list_monitor_devices_snapshot() -> RuntimeResult<Vec<MonitorDeviceSnapshot>> {
    let tty_device = terminal_monitor_device_snapshot()?;

    Ok(tty_device.into_iter().collect())
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
    let connected = !matches!(action, InputEventAction::Disconnect);
    let metadata = InputMonitorEventMetadata {
        timestamp_ns,
        sequence,
        device_id: binding.store_string(device_id),
        instance_id: None,
        device_kind,
        connected,
    };

    match action {
        InputEventAction::Connect => {
            InputMonitorEvent::InputMonitorConnectEvent(InputMonitorConnectEvent {
                kind: binding.store_string("connect"),
                metadata,
            })
        }
        InputEventAction::Disconnect => {
            InputMonitorEvent::InputMonitorDisconnectEvent(InputMonitorDisconnectEvent {
                kind: binding.store_string("disconnect"),
                metadata,
            })
        }
        _ => InputMonitorEvent::InputMonitorChangeEvent(InputMonitorChangeEvent {
            kind: binding.store_string("change"),
            metadata,
        }),
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

/// Drain one worker-local unix monitor queue into one monitor binding.
fn drain_runtime_monitor_events(
    binding: &mut UnixInputMonitorBinding,
    runtime_state: &UnixInputMonitorRuntimeState,
) -> bool {
    let queue = runtime_state
        .queue
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let front_sequence = queue.events.front().map(|event| event.sequence);
    let back_sequence = queue.events.back().map(|event| event.sequence);

    // when this handle fell behind queue retention, rebuild from the current snapshot
    if let Some(front_sequence) = front_sequence
        && binding.last_service_sequence != 0
        && front_sequence > binding.last_service_sequence.saturating_add(1)
    {
        let current_devices = list_monitor_devices_snapshot().unwrap_or_default();
        enqueue_monitor_delta(binding, &current_devices);
        binding.last_service_sequence = back_sequence.unwrap_or(binding.last_service_sequence);

        return !binding.pending_events.is_empty();
    }

    for event in &queue.events {
        if event.sequence <= binding.last_service_sequence {
            continue;
        }

        enqueue_monitor_action(
            binding,
            event.event.device_id.clone(),
            event.event.action,
            Some(event.event.device_kind),
        );
        binding.last_service_sequence = event.sequence;
    }

    !binding.pending_events.is_empty()
}

/// Poll monitor state until one event is available or would-block.
fn poll_monitor_event(
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputMonitorEvent> {
    #[cfg(unix)]
    let runtime_state = unix_input_monitor_runtime_state(binding);
    #[cfg(unix)]
    let service = binding
        .worker()
        .platform_state
        .input
        .unix_input_monitor_service("destack.input.service.monitor")?;
    #[cfg(unix)]
    ensure_unix_monitor_runtime_registration(binding, &service, &runtime_state);
    #[cfg(unix)]
    service.ensure_worker(binding, operation)?;

    loop {
        // drain watcher queues and attempt one queue pop
        let next = binding
            .worker()
            .resources
            .with_entry_mut(handle.0, |entry| {
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

                #[cfg(unix)]
                let requires_snapshot =
                    resolved_binding.pending_events.is_empty() && !service.is_worker_running();

                #[cfg(unix)]
                let has_runtime_events =
                    drain_runtime_monitor_events(resolved_binding, &runtime_state);

                let event = resolved_binding.pending_events.pop_front();
                let event = event.map(|event| {
                    let sequence = resolved_binding.next_sequence;
                    resolved_binding.next_sequence =
                        resolved_binding.next_sequence.saturating_add(1);
                    (event, sequence)
                });
                Some(Ok::<_, Box<RuntimeError>>((
                    event,
                    has_runtime_events,
                    requires_snapshot,
                )))
            });

        match next {
            // return one queued monitor event
            Some(Some(Ok((Some((event, sequence)), _, _)))) => {
                return Ok(monitor_event_to_output(binding, event, sequence));
            }

            // when no watch backend exists, rescan device ids and enqueue topology deltas
            Some(Some(Ok((None, _, true)))) => {
                let current_devices = list_monitor_devices(binding)?;
                let next = binding
                    .worker()
                    .resources
                    .with_entry_mut(handle.0, |entry| {
                        if entry.kind != ResourceKind::InputMonitor {
                            return None;
                        }

                        if entry.label.as_deref() != Some(INPUT_MONITOR_RESOURCE_LABEL) {
                            return None;
                        }

                        let resolved_binding = entry.payload.as_mut().and_then(|payload| {
                            payload.downcast_mut::<UnixInputMonitorBinding>()
                        })?;
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

                        // restart the shared service before blocking again
                        service.ensure_worker(binding, operation)?;
                        let queue = runtime_state
                            .queue
                            .lock()
                            .unwrap_or_else(|error| error.into_inner());
                        let _queue = runtime_state
                            .wake
                            .wait(queue)
                            .unwrap_or_else(|error| error.into_inner());
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
            Some(Some(Ok((None, has_runtime_events, _)))) => {
                if has_runtime_events {
                    continue;
                } else if !service.is_worker_running() {
                    service.ensure_worker(binding, operation)?;
                } else {
                    let queue = runtime_state
                        .queue
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    let _queue = runtime_state
                        .wake
                        .wait(queue)
                        .unwrap_or_else(|error| error.into_inner());
                }
            }
            // bubble queue and snapshot failures
            Some(Some(Err(error))) => return Err(error),
            // return not-found when monitor payload cannot be resolved
            Some(None) => return Err(monitor_not_found(operation, handle)),
            // return not-found when monitor handle is invalid
            None => return Err(monitor_not_found(operation, handle)),
        }
    }
}

/// Read one input event.
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
pub(crate) unsafe fn destack_input_monitor_close(
    binding: &BindingCallContext,
    handle: resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate monitor handle shape
    validate_monitor_handle(binding, handle, "destack.input.event.monitorClose")?;

    // remove and finalize monitor resource
    let removed = binding.worker().resources.remove_and_finalize(
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
pub(crate) unsafe fn destack_input_monitor_open(
    binding: &BindingCallContext,
    out: *mut resource::InputMonitorHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(unix)]
    let runtime_state = unix_input_monitor_runtime_state(binding);
    #[cfg(unix)]
    let service = binding
        .worker()
        .platform_state
        .input
        .unix_input_monitor_service("destack.input.event.monitorOpen")?;
    #[cfg(unix)]
    ensure_unix_monitor_runtime_registration(binding, &service, &runtime_state);
    #[cfg(unix)]
    service.ensure_worker(binding, "destack.input.event.monitorOpen")?;

    // create one monitor payload from current device snapshot
    let known_devices = list_monitor_devices(binding)?;
    let mut known_device_ids = Vec::with_capacity(known_devices.len());
    let mut known_device_kinds = HashMap::with_capacity(known_devices.len());
    for device in known_devices {
        known_device_ids.push(device.device_id.clone());
        known_device_kinds.insert(device.device_id, device.device_kind);
    }
    let resolved_binding = UnixInputMonitorBinding {
        known_devices: known_device_ids,
        known_device_kinds,
        last_service_sequence: 0,
        pending_events: VecDeque::new(),
        next_sequence: 1,
    };

    let entry = ResourceEntry::new(ResourceKind::InputMonitor)
        .with_label(INPUT_MONITOR_RESOURCE_LABEL)
        .with_payload(resolved_binding);
    let handle = resource::InputMonitorHandle(binding.worker().resources.insert(
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
pub(crate) unsafe fn destack_input_set_read_mode(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    mode: InputReadMode,
) -> RuntimeResult<()> {
    // delegate mode updates to unix core helpers
    input_core::set_unix_read_mode(binding, handle, mode, "destack.input.event.setReadMode")
}

/// Poll one input event without blocking.
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
            last_service_sequence: 0,
            pending_events: VecDeque::new(),
            next_sequence: 1,
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
