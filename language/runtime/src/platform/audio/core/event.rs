use super::*;
use crate::platform::audio::host;

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

    // reject unknown subscription flags
    let unknown_flags = options.flags.0 & !KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK;
    if unknown_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.flags",
            format!("options.flags contains unknown bits: 0x{unknown_flags:08x}"),
        ))
        .boxed());
    }

    // require explicit stream target when stream related flags are requested
    let stream_flags = options.flags.0 & STREAM_EVENT_SUBSCRIPTION_FLAGS_MASK;
    if stream_flags != 0 && !options.has_stream {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.hasStream",
            "stream subscription flags require options.hasStream true",
        ))
        .boxed());
    }

    // ensure the stream target exists and matches the selected backend
    if options.has_stream {
        let stream = resolve_stream_binding(context, options.stream, operation)?;
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
    if options.has_stream {
        let stream = resolve_stream_binding(context, options.stream, "destack.audio.event.open")?;
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
        pending: VecDeque::new(),
    })
}

/// Push one device-level monitor event.
fn push_event_record(binding: &mut AudioEventBinding, event: AudioEventRecord) {
    let queue_capacity = binding.options.queue_capacity.max(1) as usize;

    // keep one bounded queue by dropping the oldest events first
    while binding.pending.len() >= queue_capacity {
        let _ = binding.pending.pop_front();
    }

    binding.pending.push_back(event);
}

/// Push one device-level monitor event.
fn push_device_event(
    binding: &mut AudioEventBinding,
    kind: AudioEventKind,
    timestamp_ns: u64,
    backend: AudioBackend,
    device_id: String,
) {
    push_event_record(
        binding,
        AudioEventRecord {
            kind,
            timestamp_ns,
            backend,
            device_id,
            flags: 0,
            status_flags: AudioStreamStatusFlags(0),
            xrun_count_delta: 0,
            has_stream: false,
            stream: resource::AudioStreamHandle(resource::ResourceId(0)),
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
            backend,
            device_id,
            flags: 0,
            status_flags,
            xrun_count_delta,
            has_stream: true,
            stream,
        },
    );
}

/// Refresh pending events for one monitor binding.
pub(crate) fn refresh_event_queue(
    context: &BindingCallContext,
    binding: &mut AudioEventBinding,
) -> RuntimeResult<()> {
    let now = host_monotonic_nanos();
    let poll_interval = binding.options.poll_interval_ns.max(1);

    // refresh events only after one poll interval
    if now.saturating_sub(binding.last_refresh_ns) < poll_interval {
        return Ok(());
    }

    let backend = binding.options.backend;
    let snapshot = monitor_snapshot(backend)?;

    for id in &snapshot.ids {
        if !binding.previous_signatures.contains_key(id) {
            if event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEVICE_HOTPLUG) {
                push_device_event(
                    binding,
                    AudioEventKind::DeviceAdded,
                    now,
                    backend,
                    id.clone(),
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
                );
            }

            if event_subscription_enabled(binding, EVENT_SUBSCRIBE_REROUTE) {
                push_device_event(
                    binding,
                    AudioEventKind::DeviceRerouted,
                    now,
                    backend,
                    id.clone(),
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
        );
    }

    let should_track_stream = binding.options.has_stream
        && (event_subscription_enabled(binding, EVENT_SUBSCRIBE_STREAM)
            || event_subscription_enabled(binding, EVENT_SUBSCRIBE_INTERRUPTION)
            || event_subscription_enabled(binding, EVENT_SUBSCRIBE_BACKEND));
    if should_track_stream {
        let stream_handle = binding.options.stream;
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

    binding.previous_signatures = snapshot.signatures;
    binding.previous_default_playback = snapshot.default_playback;
    binding.previous_default_capture = snapshot.default_capture;
    binding.previous_default_loopback = snapshot.default_loopback;
    binding.last_refresh_ns = now;

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
    AudioEvent {
        kind: event.kind,
        timestamp_ns: event.timestamp_ns,
        backend: event.backend,
        flags: event.flags,
        status_flags: event.status_flags,
        xrun_count_delta: event.xrun_count_delta,
        has_device_id: !event.device_id.is_empty(),
        device_id: context.store_string(&event.device_id),
        has_stream: event.has_stream,
        stream: event.stream,
    }
}
