use super::*;
use crate::platform::audio::{
    AudioBackendDisconnectedEvent, AudioBackendDisconnectedPayload, AudioBackendResetEvent,
    AudioBackendResetPayload, AudioDefaultCaptureChangedEvent, AudioDefaultCaptureChangedPayload,
    AudioDefaultLoopbackChangedEvent, AudioDefaultLoopbackChangedPayload,
    AudioDefaultPlaybackChangedEvent, AudioDefaultPlaybackChangedPayload, AudioDeviceAddedEvent,
    AudioDeviceAddedPayload, AudioDeviceFormatChangedEvent, AudioDeviceFormatChangedPayload,
    AudioDeviceRemovedEvent, AudioDeviceRemovedPayload, AudioDeviceReroutedEvent,
    AudioDeviceReroutedPayload, AudioEventMetadata, AudioInterruptionBeganEvent,
    AudioInterruptionBeganPayload, AudioInterruptionEndedEvent, AudioInterruptionEndedPayload,
    AudioStreamDeviceChangedEvent, AudioStreamDeviceChangedPayload, AudioStreamStateChangedEvent,
    AudioStreamStateChangedPayload, AudioStreamXRunEvent, AudioStreamXRunPayload, host,
};
use std::sync::atomic::{AtomicBool, Ordering};

/// Mask for all recognized subscription flags.
const KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK: u32 = EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0
    | EVENT_SUBSCRIBE_DEFAULT_ROUTE.0
    | EVENT_SUBSCRIBE_FORMAT_CHANGE.0
    | EVENT_SUBSCRIBE_REROUTE.0
    | EVENT_SUBSCRIBE_INTERRUPTION.0
    | EVENT_SUBSCRIBE_BACKEND.0
    | EVENT_SUBSCRIBE_STREAM.0;

/// Mask for flags that require stream tracking.
const STREAM_EVENT_SUBSCRIPTION_FLAGS_MASK: u32 =
    EVENT_SUBSCRIBE_INTERRUPTION.0 | EVENT_SUBSCRIBE_BACKEND.0 | EVENT_SUBSCRIBE_STREAM.0;
/// Mask for flags that request device-level event tracking.
const DEVICE_EVENT_SUBSCRIPTION_FLAGS_MASK: u32 = EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0
    | EVENT_SUBSCRIBE_DEFAULT_ROUTE.0
    | EVENT_SUBSCRIBE_FORMAT_CHANGE.0
    | EVENT_SUBSCRIBE_REROUTE.0;

/// Shared registry for active event subscriptions used by native event delivery.
static EVENT_BINDING_REGISTRY: OnceLock<Mutex<Vec<std::sync::Weak<Mutex<AudioEventBinding>>>>> =
    OnceLock::new();

/// Shared registry mapping stream bindings to stream handles for native event delivery.
static STREAM_BINDING_REGISTRY: OnceLock<Mutex<HashMap<usize, resource::AudioStreamHandle>>> =
    OnceLock::new();
/// Shared registry for backend device-monitor worker threads.
static DEVICE_MONITOR_REGISTRY: OnceLock<Mutex<HashMap<AudioBackend, DeviceMonitorWorker>>> =
    OnceLock::new();

/// One backend device-monitor worker payload.
struct DeviceMonitorWorker {
    /// Stop signal shared with the worker thread.
    stop: Arc<AtomicBool>,
    /// Running monitor thread handle.
    handle: JoinHandle<()>,
}

/// Return one shared event-binding registry.
fn event_binding_registry() -> &'static Mutex<Vec<std::sync::Weak<Mutex<AudioEventBinding>>>> {
    EVENT_BINDING_REGISTRY.get_or_init(|| Mutex::new(Vec::new()))
}

/// Return one shared stream-handle registry.
fn stream_binding_registry() -> &'static Mutex<HashMap<usize, resource::AudioStreamHandle>> {
    STREAM_BINDING_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Return one shared backend device-monitor registry.
fn device_monitor_registry() -> &'static Mutex<HashMap<AudioBackend, DeviceMonitorWorker>> {
    DEVICE_MONITOR_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Return one stable identity for one stream binding pointer.
fn stream_binding_identity(binding: &AudioStreamBinding) -> usize {
    binding as *const AudioStreamBinding as usize
}

/// Return one subscription flag selector for one stream event kind.
fn subscription_flag_for_stream_event(kind: AudioEventKind) -> Option<AudioEventSubscriptionFlags> {
    match kind {
        AudioEventKind::StreamStateChanged
        | AudioEventKind::StreamXRun
        | AudioEventKind::StreamDeviceChanged => Some(EVENT_SUBSCRIBE_STREAM),
        AudioEventKind::InterruptionBegan | AudioEventKind::InterruptionEnded => {
            Some(EVENT_SUBSCRIBE_INTERRUPTION)
        }
        AudioEventKind::BackendDisconnected | AudioEventKind::BackendReset => {
            Some(EVENT_SUBSCRIBE_BACKEND)
        }
        _ => None,
    }
}

/// Return the native-only event-subscription support mask for one backend.
fn native_only_supported_subscription_flags(backend: AudioBackend) -> u32 {
    let mut flags = EVENT_SUBSCRIBE_STREAM.0;

    if backend == AudioBackend::CoreAudio {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
        flags |= EVENT_SUBSCRIBE_DEFAULT_ROUTE.0;
        flags |= EVENT_SUBSCRIBE_FORMAT_CHANGE.0;
        flags |= EVENT_SUBSCRIBE_REROUTE.0;
    }

    if backend == AudioBackend::Wasapi {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
        flags |= EVENT_SUBSCRIBE_DEFAULT_ROUTE.0;
        flags |= EVENT_SUBSCRIBE_FORMAT_CHANGE.0;
        flags |= EVENT_SUBSCRIBE_REROUTE.0;
    }

    if backend == AudioBackend::Jack {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
        flags |= EVENT_SUBSCRIBE_FORMAT_CHANGE.0;
        flags |= EVENT_SUBSCRIBE_REROUTE.0;
    }

    if backend == AudioBackend::PulseAudio || backend == AudioBackend::PipeWire {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
        flags |= EVENT_SUBSCRIBE_DEFAULT_ROUTE.0;
        flags |= EVENT_SUBSCRIBE_FORMAT_CHANGE.0;
        flags |= EVENT_SUBSCRIBE_REROUTE.0;
    }

    if backend == AudioBackend::Alsa || backend == AudioBackend::Asio {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
    }

    if backend == AudioBackend::AAudio || backend == AudioBackend::OpenSLES {
        flags |= EVENT_SUBSCRIBE_BACKEND.0;
    }

    flags
}

/// Return whether one subscription tracks device-level events.
fn tracks_device_events(binding: &AudioEventBinding) -> bool {
    if binding.options.flags.0 == 0 {
        return true;
    }

    (binding.options.flags.0 & DEVICE_EVENT_SUBSCRIPTION_FLAGS_MASK) != 0
}

/// Return active bindings that accept native device events for one backend.
fn active_native_device_publish_bindings(
    backend: AudioBackend,
) -> Vec<Arc<Mutex<AudioEventBinding>>> {
    let mut registry = event_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut bindings = Vec::new();
    registry.retain(|value| {
        let Some(binding) = value.upgrade() else {
            return false;
        };

        let is_active = {
            let binding_guard = binding.lock().unwrap_or_else(|error| error.into_inner());
            binding_guard.options.backend == backend
                && binding_guard.options.delivery_mode != AudioEventDeliveryMode::PollOnly
                && tracks_device_events(&binding_guard)
        };
        if is_active {
            bindings.push(binding);
        }

        true
    });

    bindings
}

/// Return whether one active native-only device subscription exists for one backend.
fn has_native_device_subscription(backend: AudioBackend) -> bool {
    let mut registry = event_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.retain(|value| value.strong_count() > 0);

    for value in registry.iter() {
        let Some(binding) = value.upgrade() else {
            continue;
        };

        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        if binding.options.backend != backend {
            continue;
        }

        if binding.options.delivery_mode != AudioEventDeliveryMode::NativeOnly {
            continue;
        }

        if !tracks_device_events(&binding) {
            continue;
        }

        return true;
    }

    false
}

/// Return active native-only device subscription bindings for one backend.
fn active_native_device_bindings(backend: AudioBackend) -> Vec<Arc<Mutex<AudioEventBinding>>> {
    let mut registry = event_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut bindings = Vec::new();
    registry.retain(|value| {
        let Some(binding) = value.upgrade() else {
            return false;
        };

        let is_active = {
            let binding_guard = binding.lock().unwrap_or_else(|error| error.into_inner());
            binding_guard.options.backend == backend
                && binding_guard.options.delivery_mode == AudioEventDeliveryMode::NativeOnly
                && tracks_device_events(&binding_guard)
        };
        if is_active {
            bindings.push(binding);
        }

        true
    });

    bindings
}

/// Return whether one binding refresh interval has elapsed for one timestamp.
fn refresh_due(binding: &AudioEventBinding, now: u64) -> bool {
    let poll_interval = binding.options.poll_interval_ns.max(1);
    now.saturating_sub(binding.last_refresh_ns) >= poll_interval
}

/// Run one native-only device-monitor worker loop.
fn run_native_only_device_monitor(backend: AudioBackend, stop: Arc<AtomicBool>) {
    let sleep_interval =
        Duration::from_nanos(EVENT_POLL_INTERVAL_NS.max(MIN_EVENT_POLL_INTERVAL_NS));

    loop {
        if stop.load(Ordering::Relaxed) {
            return;
        }

        let active_bindings = active_native_device_bindings(backend);
        if active_bindings.is_empty() {
            thread::sleep(sleep_interval);
            continue;
        }

        let now = host_monotonic_nanos();
        let snapshot = monitor_snapshot(backend);

        if let Ok(snapshot) = snapshot {
            for binding in active_bindings {
                let mut binding_guard = binding.lock().unwrap_or_else(|error| error.into_inner());

                if !refresh_due(&binding_guard, now) {
                    continue;
                }

                refresh_device_events_from_snapshot(
                    &mut binding_guard,
                    &snapshot,
                    now,
                    AudioEventSource::SyntheticPoll,
                );
            }
        }

        thread::sleep(sleep_interval);
    }
}

/// Refresh one backend device-monitor worker based on active native-only subscriptions.
pub(crate) fn refresh_backend_device_monitor(backend: AudioBackend) -> RuntimeResult<()> {
    if host::backend_native_device_events_supported(backend) {
        let has_native_bindings = !active_native_device_publish_bindings(backend).is_empty();
        if has_native_bindings {
            host::start_backend_native_device_events(backend)?;
        } else {
            host::stop_backend_native_device_events(backend);
        }
    }

    // null backend always uses runtime synthetic device rows and does not need one worker thread
    if backend == AudioBackend::Null {
        return Ok(());
    }

    let requires_worker = has_native_device_subscription(backend);
    let mut registry = device_monitor_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // start one worker when a native-only device subscription first appears
    if requires_worker && !registry.contains_key(&backend) {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_signal = Arc::clone(&stop);
        let handle = thread::spawn(move || run_native_only_device_monitor(backend, stop_signal));
        registry.insert(backend, DeviceMonitorWorker { stop, handle });
        return Ok(());
    }

    // stop and remove the worker when no matching subscriptions remain
    if !requires_worker && let Some(worker) = registry.remove(&backend) {
        worker.stop.store(true, Ordering::Relaxed);
        let _ = worker.handle.join();
    }

    Ok(())
}

/// Publish one backend-native device snapshot diff to active subscriptions.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub(crate) fn publish_device_snapshot_native(backend: AudioBackend) {
    let now = host_monotonic_nanos();
    let snapshot = match monitor_snapshot(backend) {
        Ok(snapshot) => snapshot,
        Err(_) => return,
    };

    for binding in active_native_device_publish_bindings(backend) {
        let mut binding_guard = binding.lock().unwrap_or_else(|error| error.into_inner());
        refresh_device_events_from_snapshot(
            &mut binding_guard,
            &snapshot,
            now,
            AudioEventSource::Native,
        );
    }
}

/// Register one opened event binding for native event delivery.
pub(crate) fn register_event_binding(binding: &Arc<Mutex<AudioEventBinding>>) {
    let mut registry = event_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.push(Arc::downgrade(binding));
}

/// Unregister one closed event binding from native event delivery.
pub(crate) fn unregister_event_binding(binding: &Arc<Mutex<AudioEventBinding>>) {
    let binding_identity = Arc::as_ptr(binding) as usize;
    let mut registry = event_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.retain(|value| {
        let Some(active_binding) = value.upgrade() else {
            return false;
        };

        let active_identity = Arc::as_ptr(&active_binding) as usize;
        active_identity != binding_identity
    });
}

/// Register one opened stream binding for native event delivery.
pub(crate) fn register_stream_binding_handle(
    binding: &Arc<AudioStreamBinding>,
    handle: resource::AudioStreamHandle,
) {
    let binding_identity = Arc::as_ptr(binding) as usize;
    let mut registry = stream_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.insert(binding_identity, handle);
}

/// Unregister one closed stream binding from native event delivery.
pub(crate) fn unregister_stream_binding_handle(binding: &AudioStreamBinding) {
    let binding_identity = stream_binding_identity(binding);
    let mut registry = stream_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.remove(&binding_identity);
}

/// Return one registered stream handle for one stream binding when present.
pub(crate) fn stream_handle_for_binding(
    binding: &AudioStreamBinding,
) -> Option<resource::AudioStreamHandle> {
    let binding_identity = stream_binding_identity(binding);
    let registry = stream_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.get(&binding_identity).copied()
}

/// Publish one native stream event to matching active subscriptions.
pub(crate) fn publish_stream_event_native(
    stream: resource::AudioStreamHandle,
    stream_binding: &AudioStreamBinding,
    kind: AudioEventKind,
    status_flags: AudioStreamStatusFlags,
    xrun_count_delta: u64,
) {
    let Some(subscription_flag) = subscription_flag_for_stream_event(kind) else {
        return;
    };

    let timestamp_ns = host_monotonic_nanos();
    let mut registry = event_binding_registry()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    registry.retain(|value| {
        let Some(active_binding) = value.upgrade() else {
            return false;
        };

        let mut active_binding = active_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if active_binding.options.delivery_mode == AudioEventDeliveryMode::PollOnly {
            return true;
        }

        if active_binding.options.backend != stream_binding.device.backend {
            return true;
        }

        if active_binding.options.stream != Some(stream) {
            return true;
        }

        if !event_subscription_enabled(&active_binding, subscription_flag) {
            return true;
        }

        push_event_record(
            &mut active_binding,
            AudioEventRecord {
                kind,
                timestamp_ns,
                sequence: 0,
                dropped_count: 0,
                source: AudioEventSource::Native,
                backend: stream_binding.device.backend,
                device_id: stream_binding.device.id.clone(),
                flags: 0,
                status_flags,
                xrun_count_delta,
                stream: Some(stream),
            },
        );

        true
    });
}

/// One monitor snapshot used to compare event state.
struct MonitorSnapshot {
    /// Device signatures keyed by stable device id.
    signatures: HashMap<String, u64>,
    /// Device id set for one snapshot.
    ids: HashSet<String>,
    /// Default playback id from one snapshot.
    default_playback: Option<String>,
    /// Default capture id from one snapshot.
    default_capture: Option<String>,
    /// Default loopback id from one snapshot.
    default_loopback: Option<String>,
}

/// Build one stable hash for one identifier payload.
pub(crate) fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Build one signature value for one device descriptor.
pub(crate) fn device_signature(info: &HostDeviceDescriptor) -> u64 {
    let text = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        info.id,
        info.name,
        info.backend as u8,
        info.direction as u8,
        info.supported_directions,
        info.min_sample_rate,
        info.max_sample_rate,
        info.min_channels,
        info.max_channels,
        info.format_mask,
        info.supported_channel_mask,
    );
    stable_hash(&text)
}

/// Enumerate monitor-visible devices for one backend selector.
fn enumerate_monitor_devices(backend: AudioBackend) -> RuntimeResult<Vec<HostDeviceDescriptor>> {
    let mut devices = Vec::new();
    let mut seen = HashSet::new();

    for direction in [
        AudioDeviceDirection::Playback,
        AudioDeviceDirection::Capture,
        AudioDeviceDirection::Duplex,
        AudioDeviceDirection::Loopback,
    ] {
        let request = AudioDeviceListRequest {
            direction,
            backend,
            backend_policy: AudioBackendSelectionPolicy::Strict,
            flags: AudioDeviceListFlags(0),
        };

        for device in enumerate_devices_for_request(request)? {
            if seen.insert(device.id.clone()) {
                devices.push(device);
            }
        }
    }

    Ok(devices)
}

/// Capture one monitor snapshot for one backend.
fn monitor_snapshot(backend: AudioBackend) -> RuntimeResult<MonitorSnapshot> {
    let devices = enumerate_monitor_devices(backend)?;
    let mut signatures = HashMap::new();
    let mut ids = HashSet::new();
    let mut default_playback = None;
    let mut default_capture = None;
    let mut default_loopback = None;

    for device in &devices {
        ids.insert(device.id.clone());
        signatures.insert(device.id.clone(), device_signature(device));
        if device.is_default_playback {
            default_playback = Some(device.id.clone());
        }
        if device.is_default_capture {
            default_capture = Some(device.id.clone());
        }
        if device.is_default_loopback {
            default_loopback = Some(device.id.clone());
        }
    }

    Ok(MonitorSnapshot {
        signatures,
        ids,
        default_playback,
        default_capture,
        default_loopback,
    })
}

/// Normalize one event subscription options payload.
pub(crate) fn normalize_event_subscription_options(
    context: &BindingCallContext,
    mut options: AudioEventSubscriptionOptions,
    operation: &'static str,
) -> RuntimeResult<AudioEventSubscriptionOptions> {
    let backend =
        host::resolve_requested_backend(options.backend, options.backend_policy, operation)?;
    options.backend = backend;
    let supported_flags = supported_backend_event_subscription_flags(backend);

    // reject unknown subscription flags
    let unknown_flags = options.flags.0 & !KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK;
    if unknown_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.flags",
            format!("options.flags contains unknown bits: 0x{unknown_flags:08x}"),
        ))
        .boxed());
    }

    // reject subscription flags that the selected backend does not advertise
    let unsupported_flags = options.flags.0 & !supported_flags.0;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} unsupported subscription flags for backend {backend:?}: 0x{unsupported_flags:08x}",
        )))
        .boxed());
    }

    // require explicit stream target when stream related flags are requested
    let stream_flags = options.flags.0 & STREAM_EVENT_SUBSCRIPTION_FLAGS_MASK;
    if stream_flags != 0 && options.stream.is_none() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.stream",
            "stream subscription flags require options.stream",
        ))
        .boxed());
    }

    // enforce native-only delivery contracts by backend capability
    if options.delivery_mode == AudioEventDeliveryMode::NativeOnly {
        let requested_flags = if options.flags.0 == 0 {
            KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK
        } else {
            options.flags.0
        };
        let native_supported_flags = native_only_supported_subscription_flags(backend);
        let unsupported_native_flags = requested_flags & !native_supported_flags;
        if unsupported_native_flags != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(format!(
                "{operation} native-only delivery unsupported subscription flags for backend {backend:?}: 0x{unsupported_native_flags:08x}",
            )))
            .boxed());
        }
    }

    // ensure the stream target exists and matches the selected backend
    if let Some(stream_handle) = options.stream {
        let stream = resolve_stream_binding(context, stream_handle, operation)?;
        if stream.device.backend != backend {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.backend",
                format!(
                    "options.backend does not match stream backend: requested {:?}, stream {:?}",
                    backend, stream.device.backend,
                ),
            ))
            .boxed());
        }
    }

    // normalize queue and polling values
    if options.queue_capacity == 0 {
        options.queue_capacity = DEFAULT_EVENT_QUEUE_CAPACITY;
    }
    if options.poll_interval_ns == 0 {
        options.poll_interval_ns = EVENT_POLL_INTERVAL_NS;
    }
    options.poll_interval_ns = options
        .poll_interval_ns
        .clamp(MIN_EVENT_POLL_INTERVAL_NS, MAX_EVENT_POLL_INTERVAL_NS);

    Ok(options)
}

/// Build one event binding with seeded baseline state.
pub(crate) fn build_event_binding(
    context: &BindingCallContext,
    options: AudioEventSubscriptionOptions,
) -> RuntimeResult<AudioEventBinding> {
    let snapshot = monitor_snapshot(options.backend)?;

    let mut previous_stream_state = None;
    let mut previous_stream_device_id = None;
    let mut previous_stream_xrun_count = 0;

    // seed stream baseline so first refresh only reports true deltas
    if let Some(stream_handle) = options.stream {
        let stream = resolve_stream_binding(context, stream_handle, "destack.audio.event.open")?;
        let state = stream_state_snapshot(&stream);
        previous_stream_state = Some(state.state);
        previous_stream_device_id = Some(stream.device.id.clone());
        previous_stream_xrun_count = state.xrun_count;
    }

    Ok(AudioEventBinding {
        options,
        previous_signatures: snapshot.signatures,
        previous_default_playback: snapshot.default_playback,
        previous_default_capture: snapshot.default_capture,
        previous_default_loopback: snapshot.default_loopback,
        previous_stream_state,
        previous_stream_device_id,
        previous_stream_xrun_count,
        last_refresh_ns: host_monotonic_nanos(),
        next_sequence: 1,
        dropped_count: 0,
        overflow_error_pending: false,
        pending: VecDeque::new(),
    })
}

/// Push one device-level monitor event.
fn push_event_record(binding: &mut AudioEventBinding, mut event: AudioEventRecord) {
    let queue_capacity = binding.options.queue_capacity.max(1) as usize;

    // enforce queue capacity according to overflow policy
    while binding.pending.len() >= queue_capacity {
        match binding.options.overflow_policy {
            AudioEventOverflowPolicy::DropOldest => {
                let _ = binding.pending.pop_front();
                binding.dropped_count = binding.dropped_count.saturating_add(1);
            }
            AudioEventOverflowPolicy::DropNewest | AudioEventOverflowPolicy::Error => {
                binding.dropped_count = binding.dropped_count.saturating_add(1);
                if binding.options.overflow_policy == AudioEventOverflowPolicy::Error {
                    binding.overflow_error_pending = true;
                }
                return;
            }
        }
    }

    event.sequence = binding.next_sequence;
    binding.next_sequence = binding.next_sequence.saturating_add(1);
    event.dropped_count = binding.dropped_count;

    binding.pending.push_back(event);
}

/// Return one pending queue-overflow error and clear the latched overflow state.
pub(crate) fn take_event_overflow_error(
    binding: &mut AudioEventBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    if !binding.overflow_error_pending {
        return Ok(());
    }

    binding.overflow_error_pending = false;
    Err(audio_busy(
        operation,
        "event queue overflowed with overflow policy error",
    ))
}

/// Push one device-level monitor event.
fn push_device_event(
    binding: &mut AudioEventBinding,
    kind: AudioEventKind,
    timestamp_ns: u64,
    backend: AudioBackend,
    device_id: String,
    source: AudioEventSource,
) {
    push_event_record(
        binding,
        AudioEventRecord {
            kind,
            timestamp_ns,
            sequence: 0,
            dropped_count: 0,
            source,
            backend,
            device_id,
            flags: 0,
            status_flags: AudioStreamStatusFlags(0),
            xrun_count_delta: 0,
            stream: None,
        },
    );
}

/// Push one stream-level monitor event.
fn push_stream_event(
    binding: &mut AudioEventBinding,
    kind: AudioEventKind,
    timestamp_ns: u64,
    backend: AudioBackend,
    device_id: String,
    status_flags: AudioStreamStatusFlags,
    xrun_count_delta: u64,
    stream: resource::AudioStreamHandle,
) {
    push_event_record(
        binding,
        AudioEventRecord {
            kind,
            timestamp_ns,
            sequence: 0,
            dropped_count: 0,
            source: AudioEventSource::SyntheticPoll,
            backend,
            device_id,
            flags: 0,
            status_flags,
            xrun_count_delta,
            stream: Some(stream),
        },
    );
}

/// Refresh one binding with one captured device snapshot.
fn refresh_device_events_from_snapshot(
    binding: &mut AudioEventBinding,
    snapshot: &MonitorSnapshot,
    now: u64,
    source: AudioEventSource,
) {
    let backend = binding.options.backend;

    for id in &snapshot.ids {
        if !binding.previous_signatures.contains_key(id) {
            if event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEVICE_HOTPLUG) {
                push_device_event(
                    binding,
                    AudioEventKind::DeviceAdded,
                    now,
                    backend,
                    id.clone(),
                    source,
                );
            }
            continue;
        }

        if binding.previous_signatures.get(id) != snapshot.signatures.get(id) {
            if event_subscription_enabled(binding, EVENT_SUBSCRIBE_FORMAT_CHANGE) {
                push_device_event(
                    binding,
                    AudioEventKind::DeviceFormatChanged,
                    now,
                    backend,
                    id.clone(),
                    source,
                );
            }

            if event_subscription_enabled(binding, EVENT_SUBSCRIBE_REROUTE) {
                push_device_event(
                    binding,
                    AudioEventKind::DeviceRerouted,
                    now,
                    backend,
                    id.clone(),
                    source,
                );
            }
        }
    }

    let previous_ids = binding
        .previous_signatures
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    for id in &previous_ids {
        if !snapshot.ids.contains(id)
            && event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEVICE_HOTPLUG)
        {
            push_device_event(
                binding,
                AudioEventKind::DeviceRemoved,
                now,
                backend,
                id.clone(),
                source,
            );
        }
    }

    if binding.previous_default_playback != snapshot.default_playback
        && event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEFAULT_ROUTE)
    {
        push_device_event(
            binding,
            AudioEventKind::DefaultPlaybackChanged,
            now,
            backend,
            snapshot.default_playback.clone().unwrap_or_default(),
            source,
        );
    }

    if binding.previous_default_capture != snapshot.default_capture
        && event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEFAULT_ROUTE)
    {
        push_device_event(
            binding,
            AudioEventKind::DefaultCaptureChanged,
            now,
            backend,
            snapshot.default_capture.clone().unwrap_or_default(),
            source,
        );
    }

    if binding.previous_default_loopback != snapshot.default_loopback
        && event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEFAULT_ROUTE)
    {
        push_device_event(
            binding,
            AudioEventKind::DefaultLoopbackChanged,
            now,
            backend,
            snapshot.default_loopback.clone().unwrap_or_default(),
            source,
        );
    }

    binding.previous_signatures = snapshot.signatures.clone();
    binding.previous_default_playback = snapshot.default_playback.clone();
    binding.previous_default_capture = snapshot.default_capture.clone();
    binding.previous_default_loopback = snapshot.default_loopback.clone();
    binding.last_refresh_ns = now;
}

/// Refresh pending events for one monitor binding.
pub(crate) fn refresh_event_queue(
    context: &BindingCallContext,
    binding: &mut AudioEventBinding,
) -> RuntimeResult<()> {
    let now = host_monotonic_nanos();
    if !refresh_due(binding, now) {
        return Ok(());
    }

    let backend = binding.options.backend;
    let snapshot = monitor_snapshot(backend)?;
    refresh_device_events_from_snapshot(binding, &snapshot, now, AudioEventSource::SyntheticPoll);

    let should_track_stream = binding.options.stream.is_some()
        && (event_subscription_enabled(binding, EVENT_SUBSCRIBE_STREAM)
            || event_subscription_enabled(binding, EVENT_SUBSCRIBE_INTERRUPTION)
            || event_subscription_enabled(binding, EVENT_SUBSCRIBE_BACKEND));
    if should_track_stream {
        let stream_handle = binding
            .options
            .stream
            .expect("stream tracking requires stream");
        let stream_result =
            resolve_stream_binding(context, stream_handle, "destack.audio.event.read");

        match stream_result {
            Ok(stream) => {
                let stream_state = stream_state_snapshot(&stream);
                let stream_device_id = stream.device.id.clone();

                if let Some(previous_state) = binding.previous_stream_state
                    && previous_state != stream_state.state
                {
                    // publish one generic stream-state transition event
                    if event_subscription_enabled(binding, EVENT_SUBSCRIBE_STREAM) {
                        push_stream_event(
                            binding,
                            AudioEventKind::StreamStateChanged,
                            now,
                            stream.device.backend,
                            stream_device_id.clone(),
                            stream_state.status_flags,
                            0,
                            stream_handle,
                        );
                    }

                    // map interruption state transitions to dedicated interruption events
                    if event_subscription_enabled(binding, EVENT_SUBSCRIBE_INTERRUPTION) {
                        if previous_state != AudioStreamStateKind::Interrupted
                            && stream_state.state == AudioStreamStateKind::Interrupted
                        {
                            push_stream_event(
                                binding,
                                AudioEventKind::InterruptionBegan,
                                now,
                                stream.device.backend,
                                stream_device_id.clone(),
                                stream_state.status_flags,
                                0,
                                stream_handle,
                            );
                        } else if previous_state == AudioStreamStateKind::Interrupted
                            && stream_state.state != AudioStreamStateKind::Interrupted
                        {
                            push_stream_event(
                                binding,
                                AudioEventKind::InterruptionEnded,
                                now,
                                stream.device.backend,
                                stream_device_id.clone(),
                                stream_state.status_flags,
                                0,
                                stream_handle,
                            );
                        }
                    }

                    // map backend failure states to backend disconnect and backend reset events
                    if event_subscription_enabled(binding, EVENT_SUBSCRIBE_BACKEND) {
                        if stream_state.state == AudioStreamStateKind::BackendDisconnected {
                            push_stream_event(
                                binding,
                                AudioEventKind::BackendDisconnected,
                                now,
                                stream.device.backend,
                                stream_device_id.clone(),
                                stream_state.status_flags,
                                0,
                                stream_handle,
                            );
                        } else if stream_state.state == AudioStreamStateKind::DeviceLost {
                            push_stream_event(
                                binding,
                                AudioEventKind::BackendReset,
                                now,
                                stream.device.backend,
                                stream_device_id.clone(),
                                stream_state.status_flags,
                                0,
                                stream_handle,
                            );
                        }
                    }
                }

                if let Some(previous_device_id) = &binding.previous_stream_device_id
                    && previous_device_id != &stream_device_id
                    && event_subscription_enabled(binding, EVENT_SUBSCRIBE_STREAM)
                {
                    push_stream_event(
                        binding,
                        AudioEventKind::StreamDeviceChanged,
                        now,
                        stream.device.backend,
                        stream_device_id.clone(),
                        stream_state.status_flags,
                        0,
                        stream_handle,
                    );
                }

                if stream_state.xrun_count > binding.previous_stream_xrun_count
                    && event_subscription_enabled(binding, EVENT_SUBSCRIBE_STREAM)
                {
                    let delta = stream_state
                        .xrun_count
                        .saturating_sub(binding.previous_stream_xrun_count);
                    push_stream_event(
                        binding,
                        AudioEventKind::StreamXRun,
                        now,
                        stream.device.backend,
                        stream_device_id.clone(),
                        stream_state.status_flags,
                        delta,
                        stream_handle,
                    );
                }

                binding.previous_stream_state = Some(stream_state.state);
                binding.previous_stream_device_id = Some(stream_device_id);
                binding.previous_stream_xrun_count = stream_state.xrun_count;
            }
            Err(_) => {
                // clear cached stream tracking state when the monitored stream handle is gone
                binding.previous_stream_state = None;
                binding.previous_stream_device_id = None;
                binding.previous_stream_xrun_count = 0;
            }
        }
    }

    Ok(())
}

/// Refresh one event queue according to selected delivery mode.
pub(crate) fn refresh_event_queue_for_delivery_mode(
    context: &BindingCallContext,
    binding: &mut AudioEventBinding,
) -> RuntimeResult<()> {
    if binding.options.delivery_mode == AudioEventDeliveryMode::NativeOnly {
        return Ok(());
    }

    refresh_event_queue(context, binding)
}

/// Force one immediate refresh pass for device subscriptions on one backend.
pub(crate) fn refresh_device_subscriptions_for_rescan(
    context: &BindingCallContext,
    backend: AudioBackend,
) -> RuntimeResult<()> {
    let active_bindings = {
        let mut registry = event_binding_registry()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut active_bindings = Vec::new();
        registry.retain(|value| {
            let Some(active_binding) = value.upgrade() else {
                return false;
            };

            active_bindings.push(active_binding);
            true
        });

        active_bindings
    };

    for active_binding in active_bindings {
        let mut active_binding = active_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if active_binding.options.backend != backend || active_binding.options.stream.is_some() {
            continue;
        }

        active_binding.last_refresh_ns = 0;
        refresh_event_queue(context, &mut active_binding)?;
    }

    Ok(())
}

/// Return whether one event category is enabled for one subscription.
fn event_subscription_enabled(
    binding: &AudioEventBinding,
    flag: AudioEventSubscriptionFlags,
) -> bool {
    if binding.options.flags.0 == 0 {
        return true;
    }

    (binding.options.flags.0 & flag.0) != 0
}

/// Convert one stored event record into an ABI event payload.
pub(crate) fn abi_event(context: &BindingCallContext, event: AudioEventRecord) -> AudioEvent {
    let metadata = AudioEventMetadata {
        timestamp_ns: event.timestamp_ns,
        sequence: event.sequence,
        dropped_count: event.dropped_count,
        source: event.source,
        backend: event.backend,
        flags: event.flags,
    };
    let device_id = if event.device_id.is_empty() {
        None
    } else {
        Some(context.store_string(&event.device_id))
    };

    match event.kind {
        AudioEventKind::BackendDisconnected => {
            AudioEvent::AudioBackendDisconnectedEvent(AudioBackendDisconnectedEvent {
                kind: context.store_string("backendDisconnected"),
                metadata,
                payload: AudioBackendDisconnectedPayload {
                    stream: event.stream,
                },
            })
        }
        AudioEventKind::BackendReset => {
            AudioEvent::AudioBackendResetEvent(AudioBackendResetEvent {
                kind: context.store_string("backendReset"),
                metadata,
                payload: AudioBackendResetPayload {
                    stream: event.stream,
                },
            })
        }
        AudioEventKind::DefaultCaptureChanged => {
            AudioEvent::AudioDefaultCaptureChangedEvent(AudioDefaultCaptureChangedEvent {
                kind: context.store_string("defaultCaptureChanged"),
                metadata,
                payload: AudioDefaultCaptureChangedPayload { device_id },
            })
        }
        AudioEventKind::DefaultLoopbackChanged => {
            AudioEvent::AudioDefaultLoopbackChangedEvent(AudioDefaultLoopbackChangedEvent {
                kind: context.store_string("defaultLoopbackChanged"),
                metadata,
                payload: AudioDefaultLoopbackChangedPayload { device_id },
            })
        }
        AudioEventKind::DefaultPlaybackChanged => {
            AudioEvent::AudioDefaultPlaybackChangedEvent(AudioDefaultPlaybackChangedEvent {
                kind: context.store_string("defaultPlaybackChanged"),
                metadata,
                payload: AudioDefaultPlaybackChangedPayload { device_id },
            })
        }
        AudioEventKind::DeviceAdded => AudioEvent::AudioDeviceAddedEvent(AudioDeviceAddedEvent {
            kind: context.store_string("deviceAdded"),
            metadata,
            payload: AudioDeviceAddedPayload { device_id },
        }),
        AudioEventKind::DeviceFormatChanged => {
            AudioEvent::AudioDeviceFormatChangedEvent(AudioDeviceFormatChangedEvent {
                kind: context.store_string("deviceFormatChanged"),
                metadata,
                payload: AudioDeviceFormatChangedPayload { device_id },
            })
        }
        AudioEventKind::DeviceRemoved => {
            AudioEvent::AudioDeviceRemovedEvent(AudioDeviceRemovedEvent {
                kind: context.store_string("deviceRemoved"),
                metadata,
                payload: AudioDeviceRemovedPayload { device_id },
            })
        }
        AudioEventKind::DeviceRerouted => {
            AudioEvent::AudioDeviceReroutedEvent(AudioDeviceReroutedEvent {
                kind: context.store_string("deviceRerouted"),
                metadata,
                payload: AudioDeviceReroutedPayload { device_id },
            })
        }
        AudioEventKind::InterruptionBegan => {
            AudioEvent::AudioInterruptionBeganEvent(AudioInterruptionBeganEvent {
                kind: context.store_string("interruptionBegan"),
                metadata,
                payload: AudioInterruptionBeganPayload {
                    stream: event.stream,
                },
            })
        }
        AudioEventKind::InterruptionEnded => {
            AudioEvent::AudioInterruptionEndedEvent(AudioInterruptionEndedEvent {
                kind: context.store_string("interruptionEnded"),
                metadata,
                payload: AudioInterruptionEndedPayload {
                    stream: event.stream,
                },
            })
        }
        AudioEventKind::StreamDeviceChanged => {
            AudioEvent::AudioStreamDeviceChangedEvent(AudioStreamDeviceChangedEvent {
                kind: context.store_string("streamDeviceChanged"),
                metadata,
                payload: AudioStreamDeviceChangedPayload {
                    stream: event.stream,
                    status_flags: event.status_flags,
                    device_id,
                },
            })
        }
        AudioEventKind::StreamStateChanged => {
            AudioEvent::AudioStreamStateChangedEvent(AudioStreamStateChangedEvent {
                kind: context.store_string("streamStateChanged"),
                metadata,
                payload: AudioStreamStateChangedPayload {
                    stream: event.stream,
                    status_flags: event.status_flags,
                },
            })
        }
        AudioEventKind::StreamXRun => AudioEvent::AudioStreamXRunEvent(AudioStreamXRunEvent {
            kind: context.store_string("streamXRun"),
            metadata,
            payload: AudioStreamXRunPayload {
                stream: event.stream,
                status_flags: event.status_flags,
                xrun_count_delta: event.xrun_count_delta,
                device_id,
            },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::diagnostic::PlatformErrorCode;

    /// Build one minimal event binding for queue-behavior tests.
    fn test_binding(
        overflow_policy: AudioEventOverflowPolicy,
        queue_capacity: u32,
    ) -> AudioEventBinding {
        AudioEventBinding {
            options: AudioEventSubscriptionOptions {
                backend: AudioBackend::Null,
                backend_policy: AudioBackendSelectionPolicy::Strict,
                flags: AudioEventSubscriptionFlags(0),
                delivery_mode: AudioEventDeliveryMode::PollOnly,
                overflow_policy,
                stream: None,
                queue_capacity,
                poll_interval_ns: EVENT_POLL_INTERVAL_NS,
            },
            previous_signatures: HashMap::new(),
            previous_default_playback: None,
            previous_default_capture: None,
            previous_default_loopback: None,
            previous_stream_state: None,
            previous_stream_device_id: None,
            previous_stream_xrun_count: 0,
            last_refresh_ns: 0,
            next_sequence: 1,
            dropped_count: 0,
            overflow_error_pending: false,
            pending: VecDeque::new(),
        }
    }

    /// Build one synthetic stream event record for queue-behavior tests.
    fn test_record() -> AudioEventRecord {
        AudioEventRecord {
            kind: AudioEventKind::StreamStateChanged,
            timestamp_ns: 1,
            sequence: 0,
            dropped_count: 0,
            source: AudioEventSource::SyntheticPoll,
            backend: AudioBackend::Null,
            device_id: "audio:null:duplex".to_string(),
            flags: 0,
            status_flags: AudioStreamStatusFlags(0),
            xrun_count_delta: 0,
            stream: None,
        }
    }

    #[test]
    fn test_event_overflow_policy_error_reports_io_busy() {
        let mut binding = test_binding(AudioEventOverflowPolicy::Error, 1);
        push_event_record(&mut binding, test_record());
        push_event_record(&mut binding, test_record());

        let error = take_event_overflow_error(&mut binding, "destack.audio.event.tryRead")
            .expect_err("overflow policy error should report queued overflow");
        let code = error.platform_error().map(|platform| platform.code);
        assert_eq!(code, Some(PlatformErrorCode::IoBusy));

        take_event_overflow_error(&mut binding, "destack.audio.event.tryRead")
            .expect("overflow error should be cleared after one report");
    }

    #[test]
    fn test_event_overflow_drop_oldest_keeps_monotonic_sequence_and_drop_count() {
        let mut binding = test_binding(AudioEventOverflowPolicy::DropOldest, 2);
        for _ in 0..5 {
            push_event_record(&mut binding, test_record());
        }

        assert_eq!(binding.pending.len(), 2);
        assert_eq!(binding.dropped_count, 3);
        assert!(!binding.overflow_error_pending);

        let events = binding.pending.iter().collect::<Vec<_>>();
        assert_eq!(events[0].sequence, 4);
        assert_eq!(events[1].sequence, 5);
        assert_eq!(events[0].dropped_count, 2);
        assert_eq!(events[1].dropped_count, 3);
        assert!(events[0].sequence < events[1].sequence);
        assert!(events[0].dropped_count < events[1].dropped_count);
    }

    #[test]
    fn test_event_overflow_drop_newest_preserves_existing_queue_order() {
        let mut binding = test_binding(AudioEventOverflowPolicy::DropNewest, 2);
        for _ in 0..5 {
            push_event_record(&mut binding, test_record());
        }

        assert_eq!(binding.pending.len(), 2);
        assert_eq!(binding.dropped_count, 3);
        assert!(!binding.overflow_error_pending);
        assert_eq!(binding.next_sequence, 3);

        let events = binding.pending.iter().collect::<Vec<_>>();
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
        assert_eq!(events[0].dropped_count, 0);
        assert_eq!(events[1].dropped_count, 0);
    }

    #[test]
    fn test_event_overflow_error_keeps_existing_queue_and_latches_once() {
        let mut binding = test_binding(AudioEventOverflowPolicy::Error, 2);
        for _ in 0..3 {
            push_event_record(&mut binding, test_record());
        }

        assert_eq!(binding.pending.len(), 2);
        assert_eq!(binding.dropped_count, 1);
        assert!(binding.overflow_error_pending);

        let events = binding.pending.iter().collect::<Vec<_>>();
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
        assert_eq!(events[0].dropped_count, 0);
        assert_eq!(events[1].dropped_count, 0);

        let first_error = take_event_overflow_error(&mut binding, "destack.audio.event.tryRead");
        let first_error = first_error.expect_err("overflow policy should latch one ioBusy report");
        assert_eq!(
            first_error.platform_error().map(|platform| platform.code),
            Some(PlatformErrorCode::IoBusy),
        );

        take_event_overflow_error(&mut binding, "destack.audio.event.tryRead")
            .expect("overflow latch should clear after one report");
    }

    #[test]
    fn test_tracks_device_events_flag_mask() {
        let mut all_events_binding = test_binding(AudioEventOverflowPolicy::DropOldest, 16);
        all_events_binding.options.flags = AudioEventSubscriptionFlags(0);
        assert!(tracks_device_events(&all_events_binding));

        let mut stream_only_binding = test_binding(AudioEventOverflowPolicy::DropOldest, 16);
        stream_only_binding.options.flags = EVENT_SUBSCRIBE_STREAM;
        assert!(!tracks_device_events(&stream_only_binding));

        let mut device_binding = test_binding(AudioEventOverflowPolicy::DropOldest, 16);
        device_binding.options.flags = EVENT_SUBSCRIBE_DEVICE_HOTPLUG;
        assert!(tracks_device_events(&device_binding));
    }

    #[test]
    fn test_native_only_supported_subscription_flags_match_backend_matrix() {
        let null_flags = native_only_supported_subscription_flags(AudioBackend::Null);
        assert_eq!(null_flags, EVENT_SUBSCRIBE_STREAM.0);

        let coreaudio_flags = native_only_supported_subscription_flags(AudioBackend::CoreAudio);
        assert!((coreaudio_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((coreaudio_flags & EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0) != 0);
        assert!((coreaudio_flags & EVENT_SUBSCRIBE_DEFAULT_ROUTE.0) != 0);
        assert!((coreaudio_flags & EVENT_SUBSCRIBE_FORMAT_CHANGE.0) != 0);
        assert!((coreaudio_flags & EVENT_SUBSCRIBE_REROUTE.0) != 0);
        assert_eq!(coreaudio_flags & EVENT_SUBSCRIBE_INTERRUPTION.0, 0);

        let wasapi_flags = native_only_supported_subscription_flags(AudioBackend::Wasapi);
        assert!((wasapi_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((wasapi_flags & EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0) != 0);
        assert!((wasapi_flags & EVENT_SUBSCRIBE_DEFAULT_ROUTE.0) != 0);
        assert!((wasapi_flags & EVENT_SUBSCRIBE_FORMAT_CHANGE.0) != 0);
        assert!((wasapi_flags & EVENT_SUBSCRIBE_REROUTE.0) != 0);
        assert_eq!(wasapi_flags & EVENT_SUBSCRIBE_INTERRUPTION.0, 0);

        let jack_flags = native_only_supported_subscription_flags(AudioBackend::Jack);
        assert!((jack_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((jack_flags & EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0) != 0);
        assert!((jack_flags & EVENT_SUBSCRIBE_FORMAT_CHANGE.0) != 0);
        assert!((jack_flags & EVENT_SUBSCRIBE_REROUTE.0) != 0);
        assert_eq!(jack_flags & EVENT_SUBSCRIBE_DEFAULT_ROUTE.0, 0);
        assert_eq!(jack_flags & EVENT_SUBSCRIBE_INTERRUPTION.0, 0);

        let pulse_flags = native_only_supported_subscription_flags(AudioBackend::PulseAudio);
        assert!((pulse_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((pulse_flags & EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0) != 0);
        assert!((pulse_flags & EVENT_SUBSCRIBE_DEFAULT_ROUTE.0) != 0);
        assert!((pulse_flags & EVENT_SUBSCRIBE_FORMAT_CHANGE.0) != 0);
        assert!((pulse_flags & EVENT_SUBSCRIBE_REROUTE.0) != 0);

        let pipewire_flags = native_only_supported_subscription_flags(AudioBackend::PipeWire);
        assert!((pipewire_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((pipewire_flags & EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0) != 0);
        assert!((pipewire_flags & EVENT_SUBSCRIBE_DEFAULT_ROUTE.0) != 0);
        assert!((pipewire_flags & EVENT_SUBSCRIBE_FORMAT_CHANGE.0) != 0);
        assert!((pipewire_flags & EVENT_SUBSCRIBE_REROUTE.0) != 0);

        let alsa_flags = native_only_supported_subscription_flags(AudioBackend::Alsa);
        assert!((alsa_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((alsa_flags & EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0) != 0);
        assert_eq!(alsa_flags & EVENT_SUBSCRIBE_DEFAULT_ROUTE.0, 0);
        assert_eq!(alsa_flags & EVENT_SUBSCRIBE_FORMAT_CHANGE.0, 0);
        assert_eq!(alsa_flags & EVENT_SUBSCRIBE_REROUTE.0, 0);

        let asio_flags = native_only_supported_subscription_flags(AudioBackend::Asio);
        assert!((asio_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((asio_flags & EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0) != 0);
        assert_eq!(asio_flags & EVENT_SUBSCRIBE_DEFAULT_ROUTE.0, 0);
        assert_eq!(asio_flags & EVENT_SUBSCRIBE_FORMAT_CHANGE.0, 0);
        assert_eq!(asio_flags & EVENT_SUBSCRIBE_REROUTE.0, 0);

        let aaudio_flags = native_only_supported_subscription_flags(AudioBackend::AAudio);
        assert!((aaudio_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((aaudio_flags & EVENT_SUBSCRIBE_BACKEND.0) != 0);

        let opensles_flags = native_only_supported_subscription_flags(AudioBackend::OpenSLES);
        assert!((opensles_flags & EVENT_SUBSCRIBE_STREAM.0) != 0);
        assert!((opensles_flags & EVENT_SUBSCRIBE_BACKEND.0) != 0);
    }
}
