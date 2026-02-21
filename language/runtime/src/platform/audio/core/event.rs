use super::*;
use crate::platform::audio::host;

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

/// Push one device-level monitor event.
fn push_device_event(
    binding: &mut AudioEventBinding,
    kind: AudioEventKind,
    timestamp_ns: u64,
    backend: AudioBackend,
    device_id: String,
) {
    binding.pending.push_back(AudioEventRecord {
        kind,
        timestamp_ns,
        backend,
        device_id,
        flags: 0,
        status_flags: AudioStreamStatusFlags(0),
        xrun_count_delta: 0,
        has_stream: false,
        stream: resource::AudioStreamHandle(resource::ResourceId(0)),
    });
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
    binding.pending.push_back(AudioEventRecord {
        kind,
        timestamp_ns,
        backend,
        device_id,
        flags: 0,
        status_flags,
        xrun_count_delta,
        has_stream: true,
        stream,
    });
}

/// Refresh pending events for one monitor binding.
pub(crate) fn refresh_event_queue(
    context: &BindingCallContext,
    binding: &mut AudioEventBinding,
) -> RuntimeResult<()> {
    let backend = host::resolve_requested_backend(
        binding.options.backend,
        binding.options.backend_policy,
        "destack.audio.event.open",
    )?;

    // monitor all host-visible devices, not only duplex rows
    let devices = enumerate_monitor_devices(backend)?;
    let now = host_monotonic_nanos();

    let mut current_signatures = HashMap::new();
    let mut current_ids = HashSet::new();
    let mut default_playback = None;
    let mut default_capture = None;
    let mut default_loopback = None;

    for device in &devices {
        current_ids.insert(device.id.clone());
        current_signatures.insert(device.id.clone(), device_signature(device));
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

    for id in current_ids.iter() {
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

        if binding.previous_signatures.get(id) != current_signatures.get(id) {
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
        if !current_ids.contains(id)
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

    if binding.previous_default_playback != default_playback
        && event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEFAULT_ROUTE)
    {
        push_device_event(
            binding,
            AudioEventKind::DefaultPlaybackChanged,
            now,
            backend,
            default_playback.clone().unwrap_or_default(),
        );
    }

    if binding.previous_default_capture != default_capture
        && event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEFAULT_ROUTE)
    {
        push_device_event(
            binding,
            AudioEventKind::DefaultCaptureChanged,
            now,
            backend,
            default_capture.clone().unwrap_or_default(),
        );
    }

    if binding.previous_default_loopback != default_loopback
        && event_subscription_enabled(binding, EVENT_SUBSCRIBE_DEFAULT_ROUTE)
    {
        push_device_event(
            binding,
            AudioEventKind::DefaultLoopbackChanged,
            now,
            backend,
            default_loopback.clone().unwrap_or_default(),
        );
    }

    if binding.options.has_stream && event_subscription_enabled(binding, EVENT_SUBSCRIBE_STREAM) {
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
                }

                if let Some(previous_device_id) = &binding.previous_stream_device_id
                    && previous_device_id != &stream_device_id
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

                if stream_state.xrun_count > binding.previous_stream_xrun_count {
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
                if event_subscription_enabled(binding, EVENT_SUBSCRIBE_BACKEND) {
                    push_stream_event(
                        binding,
                        AudioEventKind::BackendDisconnected,
                        now,
                        backend,
                        String::new(),
                        AudioStreamStatusFlags(0),
                        0,
                        stream_handle,
                    );
                }
            }
        }
    }

    binding.previous_signatures = current_signatures;
    binding.previous_default_playback = default_playback;
    binding.previous_default_capture = default_capture;
    binding.previous_default_loopback = default_loopback;

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
