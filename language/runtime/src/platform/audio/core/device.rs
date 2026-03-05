use super::*;
use crate::platform::audio::host;

/// Return whether one backend exposes loopback streams in the common surface.
pub(crate) fn backend_supports_loopback(backend: AudioBackend) -> bool {
    matches!(
        backend,
        AudioBackend::Null
            | AudioBackend::Wasapi
            | AudioBackend::CoreAudio
            | AudioBackend::PipeWire
            | AudioBackend::PulseAudio
    )
}

/// Return whether one backend exposes exclusive-mode endpoints.
fn backend_supports_exclusive_mode(backend: AudioBackend) -> bool {
    matches!(
        backend,
        AudioBackend::Wasapi
            | AudioBackend::CoreAudio
            | AudioBackend::Asio
            | AudioBackend::Alsa
            | AudioBackend::AAudio
    )
}

/// Return whether one backend exposes device-clock or hardware timestamp support.
pub(crate) fn backend_supports_device_clock(backend: AudioBackend) -> bool {
    matches!(
        backend,
        AudioBackend::Null | AudioBackend::Wasapi | AudioBackend::CoreAudio
    )
}

/// Return one stream-option support mask for one backend.
pub(crate) fn supported_backend_stream_flags(backend: AudioBackend) -> AudioSupportedStreamFlags {
    let mut flags = KNOWN_SUPPORTED_STREAM_FLAGS_MASK;
    if backend != AudioBackend::Asio {
        flags &= !SUPPORTED_STREAM_FLAG_NON_INTERLEAVED.0;
    }

    AudioSupportedStreamFlags(flags)
}

/// Return one stream-requirement support mask for one backend.
pub(crate) fn supported_backend_stream_requirement_flags(
    backend: AudioBackend,
) -> AudioSupportedStreamRequirementFlags {
    let mut flags = 0u32;
    if backend == AudioBackend::Asio {
        flags |= SUPPORTED_STREAM_REQUIREMENT_NON_INTERLEAVED.0;
    }

    if backend == AudioBackend::CoreAudio || backend == AudioBackend::Wasapi {
        flags |= SUPPORTED_STREAM_REQUIREMENT_SCHEDULED_WRITE.0;
        flags |= SUPPORTED_STREAM_REQUIREMENT_HARDWARE_TIMESTAMPS.0;
    }

    flags |= SUPPORTED_STREAM_REQUIREMENT_PAUSE.0;

    if backend == AudioBackend::Asio {
        flags |= SUPPORTED_STREAM_REQUIREMENT_BIT_EXACT_PCM.0;
    }

    AudioSupportedStreamRequirementFlags(flags)
}

/// Return one event-subscription support mask for one backend.
pub(crate) fn supported_backend_event_subscription_flags(
    backend: AudioBackend,
) -> AudioSupportedEventSubscriptionFlags {
    let mut flags = KNOWN_SUPPORTED_EVENT_SUBSCRIPTION_FLAGS_MASK;
    if backend == AudioBackend::Null {
        return AudioSupportedEventSubscriptionFlags(flags);
    }

    if backend == AudioBackend::Jack {
        flags &= !SUPPORTED_EVENT_SUBSCRIPTION_DEFAULT_ROUTE.0;
    }

    AudioSupportedEventSubscriptionFlags(flags)
}

/// Return one stream-clock support mask for one device descriptor.
fn supported_stream_clock_domains_for_device(
    info: &HostDeviceDescriptor,
) -> AudioSupportedStreamClockDomains {
    let mut flags = SUPPORTED_STREAM_CLOCK_MONOTONIC.0
        | SUPPORTED_STREAM_CLOCK_WALL.0
        | SUPPORTED_STREAM_CLOCK_CALLBACK.0;

    let supports_hardware_clock = backend_supports_device_clock(info.backend);
    if supports_hardware_clock {
        flags |= SUPPORTED_STREAM_CLOCK_DEVICE.0;
    }

    if supports_hardware_clock {
        if matches!(
            info.direction,
            AudioDeviceDirection::Capture
                | AudioDeviceDirection::Duplex
                | AudioDeviceDirection::Loopback
        ) {
            flags |= SUPPORTED_STREAM_CLOCK_INPUT_ADC.0;
        }

        if matches!(
            info.direction,
            AudioDeviceDirection::Playback
                | AudioDeviceDirection::Duplex
                | AudioDeviceDirection::Loopback
        ) {
            flags |= SUPPORTED_STREAM_CLOCK_OUTPUT_DAC.0;
        }
    }

    AudioSupportedStreamClockDomains(flags)
}

/// Return one device-open support mask for one device descriptor.
pub(crate) fn supported_backend_device_open_flags(backend: AudioBackend) -> AudioDeviceOpenFlags {
    let mut flags = DEVICE_OPEN_FOLLOW_DEFAULT_ROUTE.0
        | DEVICE_OPEN_LOW_LATENCY.0
        | DEVICE_OPEN_REALTIME_THREAD.0;

    if backend == AudioBackend::Wasapi {
        flags |= DEVICE_OPEN_RAW.0;
    }

    if backend == AudioBackend::Alsa {
        flags |= BACKEND_OPEN_ALSA_NO_RESAMPLE.0;
    }

    if backend == AudioBackend::Jack {
        flags |= BACKEND_OPEN_JACK_NO_AUTOCONNECT.0;
    }

    AudioDeviceOpenFlags(flags)
}

/// Return one device-list support mask for one backend.
pub(crate) fn supported_backend_device_list_flags(backend: AudioBackend) -> AudioDeviceListFlags {
    let mut flags = DEVICE_LIST_INCLUDE_DISCONNECTED.0;

    if backend_supports_loopback(backend) {
        flags |= DEVICE_LIST_INCLUDE_LOOPBACK.0;
    }

    if backend == AudioBackend::Wasapi {
        flags |= DEVICE_LIST_INCLUDE_RAW.0;
    }

    AudioDeviceListFlags(flags)
}

/// Return one stream-clock support mask for one backend.
pub(crate) fn supported_backend_stream_clock_domains(
    backend: AudioBackend,
) -> AudioSupportedStreamClockDomains {
    let mut flags = SUPPORTED_STREAM_CLOCK_MONOTONIC.0
        | SUPPORTED_STREAM_CLOCK_WALL.0
        | SUPPORTED_STREAM_CLOCK_CALLBACK.0;

    if backend_supports_device_clock(backend) {
        flags |= SUPPORTED_STREAM_CLOCK_DEVICE.0;
        flags |= SUPPORTED_STREAM_CLOCK_INPUT_ADC.0;
        flags |= SUPPORTED_STREAM_CLOCK_OUTPUT_DAC.0;
    }

    AudioSupportedStreamClockDomains(flags)
}

/// Return one device-open support mask for one device descriptor.
fn supported_device_open_flags_for_device(info: &HostDeviceDescriptor) -> AudioDeviceOpenFlags {
    let mut flags = supported_backend_device_open_flags(info.backend).0;
    if !info.is_raw {
        flags &= !DEVICE_OPEN_RAW.0;
    }

    AudioDeviceOpenFlags(flags)
}

/// Build one synthetic null device entry.
pub(crate) fn null_device(direction: AudioDeviceDirection) -> HostDeviceDescriptor {
    let (name, id) = match direction {
        AudioDeviceDirection::Playback => ("Null playback device", "audio:null:playback"),
        AudioDeviceDirection::Capture => ("Null capture device", "audio:null:capture"),
        AudioDeviceDirection::Duplex => ("Null duplex device", "audio:null:duplex"),
        AudioDeviceDirection::Loopback => ("Null loopback device", "audio:null:loopback"),
    };

    let mut capability_flags = DEVICE_CAPABILITY_SHARED_MODE.0
        | DEVICE_CAPABILITY_STREAM_VOLUME.0
        | DEVICE_CAPABILITY_STREAM_MUTE.0
        | DEVICE_CAPABILITY_DEVICE_CLOCK.0
        | DEVICE_CAPABILITY_INTERRUPTION_EVENTS.0
        | DEVICE_CAPABILITY_REROUTE_EVENTS.0
        | DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0;

    if direction == AudioDeviceDirection::Playback || direction == AudioDeviceDirection::Duplex {
        capability_flags |= DEVICE_CAPABILITY_SCHEDULED_WRITE.0;
    }

    if direction == AudioDeviceDirection::Loopback {
        capability_flags |= DEVICE_CAPABILITY_LOOPBACK.0;
    }

    if direction == AudioDeviceDirection::Duplex {
        capability_flags |= DEVICE_CAPABILITY_FULL_DUPLEX.0;
    }

    let supported_directions = match direction {
        AudioDeviceDirection::Playback => DIRECTION_MASK_PLAYBACK,
        AudioDeviceDirection::Capture => DIRECTION_MASK_CAPTURE,
        AudioDeviceDirection::Duplex => {
            DIRECTION_MASK_PLAYBACK | DIRECTION_MASK_CAPTURE | DIRECTION_MASK_DUPLEX
        }
        AudioDeviceDirection::Loopback => DIRECTION_MASK_CAPTURE | DIRECTION_MASK_LOOPBACK,
    };

    HostDeviceDescriptor {
        id: id.to_string(),
        group_id: "audio:null:group".to_string(),
        name: name.to_string(),
        transport: "null".to_string(),
        backend: AudioBackend::Null,
        direction,
        supported_directions,
        connected: true,
        is_raw: false,
        is_default_playback: false,
        is_default_capture: false,
        is_default_loopback: direction == AudioDeviceDirection::Loopback,
        capability_flags: AudioDeviceCapabilityFlags(capability_flags),
        preferred_sample_rate: NULL_DEVICE_PREFERRED_SAMPLE_RATE,
        min_sample_rate: NULL_DEVICE_MIN_SAMPLE_RATE,
        max_sample_rate: NULL_DEVICE_MAX_SAMPLE_RATE,
        preferred_period_frames: NULL_DEVICE_PREFERRED_PERIOD_FRAMES,
        min_channels: 1,
        max_channels: 8,
        preferred_layout: AudioChannelLayout::Stereo,
        preferred_channel_mask: NULL_DEVICE_PREFERRED_CHANNEL_MASK,
        supported_channel_mask: NULL_DEVICE_SUPPORTED_CHANNEL_MASK,
        min_period_frames: MIN_STREAM_PERIOD_FRAMES,
        max_period_frames: NULL_DEVICE_MAX_PERIOD_FRAMES,
        format_mask: all_sample_format_mask(),
        share_mode_mask: SHARE_MODE_SHARED_BIT,
        is_null: true,
    }
}

/// Validate and normalize one device-open option payload for one backend.
pub(crate) fn normalize_device_open_options(
    options: AudioDeviceOpenOptions,
    backend: AudioBackend,
    operation: &'static str,
) -> RuntimeResult<AudioDeviceOpenOptions> {
    // reject unknown device-open flag bits
    let unknown_open_flags = options.flags.0 & !KNOWN_DEVICE_OPEN_FLAGS_MASK;
    if unknown_open_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.flags",
            format!("options.flags contains unknown bits: 0x{unknown_open_flags:08x}"),
        ))
        .boxed());
    }

    // gate raw mode to backends that expose raw endpoints
    if (options.flags.0 & DEVICE_OPEN_RAW.0) != 0 && backend != AudioBackend::Wasapi {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} device open flag: raw",
        )))
        .boxed());
    }

    // require explicit loopback direction for loopback-only requests
    if options.direction == AudioDeviceDirection::Loopback && !backend_supports_loopback(backend) {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} loopback direction",
        )))
        .boxed());
    }

    // gate exclusive mode to backends that expose it
    if options.share_mode == AudioShareMode::Exclusive && !backend_supports_exclusive_mode(backend)
    {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} share mode: exclusive",
        )))
        .boxed());
    }

    Ok(options)
}

/// Validate that requested device-open flags are supported by the target device row.
pub(crate) fn ensure_device_open_flags_supported(
    options: AudioDeviceOpenOptions,
    info: &HostDeviceDescriptor,
    operation: &'static str,
) -> RuntimeResult<()> {
    let supported_flags = supported_device_open_flags_for_device(info);
    let unsupported_flags = options.flags.0 & !supported_flags.0;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} device open flags unsupported by device: 0x{unsupported_flags:08x}",
        )))
        .boxed());
    }

    Ok(())
}

/// Enumerate all devices visible to the request.
pub(crate) fn enumerate_devices_for_request(
    request: AudioDeviceListRequest,
) -> RuntimeResult<Vec<HostDeviceDescriptor>> {
    // reject unknown device-list flag bits
    let unknown_list_flags = request.flags.0 & !KNOWN_DEVICE_LIST_FLAGS_MASK;
    if unknown_list_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "request.flags",
            format!("request.flags contains unknown bits: 0x{unknown_list_flags:08x}"),
        ))
        .boxed());
    }

    let backend = host::resolve_requested_backend(
        request.backend,
        request.backend_policy,
        "destack.audio.device.list",
    )?;
    let supported_list_flags = supported_backend_device_list_flags(backend);
    let unsupported_list_flags = request.flags.0 & !supported_list_flags.0;
    if unsupported_list_flags != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "destack.audio.device.list unsupported request flags: 0x{unsupported_list_flags:08x}",
        )))
        .boxed());
    }
    let include_disconnected = (request.flags.0 & DEVICE_LIST_INCLUDE_DISCONNECTED.0) != 0;
    let include_raw = (request.flags.0 & DEVICE_LIST_INCLUDE_RAW.0) != 0;
    let include_loopback = (request.flags.0 & DEVICE_LIST_INCLUDE_LOOPBACK.0) != 0;

    let mut devices = Vec::new();

    if backend == AudioBackend::Null {
        devices.push(null_device(AudioDeviceDirection::Playback));
        devices.push(null_device(AudioDeviceDirection::Capture));
        devices.push(null_device(AudioDeviceDirection::Duplex));
        devices.push(null_device(AudioDeviceDirection::Loopback));
    } else {
        devices.extend(host::enumerate_host_devices(backend)?);
    }

    let include_direction = |device: &HostDeviceDescriptor| -> bool {
        if !include_disconnected && !device.connected {
            return false;
        }

        if !include_raw && device.is_raw {
            return false;
        }

        match request.direction {
            AudioDeviceDirection::Playback => {
                supports_device_open_direction(device, AudioDeviceDirection::Playback)
            }
            AudioDeviceDirection::Capture => {
                if device.direction == AudioDeviceDirection::Loopback && !include_loopback {
                    return false;
                }

                supports_device_open_direction(device, AudioDeviceDirection::Capture)
            }
            AudioDeviceDirection::Duplex => {
                supports_device_open_direction(device, AudioDeviceDirection::Duplex)
            }
            AudioDeviceDirection::Loopback => {
                supports_device_open_direction(device, AudioDeviceDirection::Loopback)
            }
        }
    };

    devices.retain(include_direction);

    Ok(devices)
}

/// Return one direction-bit selector.
pub(crate) fn direction_mask_for(direction: AudioDeviceDirection) -> u32 {
    match direction {
        AudioDeviceDirection::Playback => DIRECTION_MASK_PLAYBACK,
        AudioDeviceDirection::Capture => DIRECTION_MASK_CAPTURE,
        AudioDeviceDirection::Duplex => DIRECTION_MASK_DUPLEX,
        AudioDeviceDirection::Loopback => DIRECTION_MASK_LOOPBACK,
    }
}

/// Return whether one device supports one open direction.
pub(crate) fn supports_device_open_direction(
    device: &HostDeviceDescriptor,
    direction: AudioDeviceDirection,
) -> bool {
    let mask = direction_mask_for(direction);
    if (device.supported_directions & mask) == 0 {
        return false;
    }

    if direction == AudioDeviceDirection::Loopback {
        return (device.capability_flags.0 & DEVICE_CAPABILITY_LOOPBACK.0) != 0;
    }

    true
}

/// Return one descriptor snapshot for one open device handle.
pub(crate) fn descriptor_from_info(
    binding: &BindingCallContext,
    info: &HostDeviceDescriptor,
) -> AudioDeviceDescriptor {
    AudioDeviceDescriptor {
        id: binding.store_string(&info.id),
        group_id: binding.store_string(&info.group_id),
        name: binding.store_string(&info.name),
        transport: binding.store_string(&info.transport),
        backend: info.backend,
        direction: info.direction,
        connected: info.connected,
        is_raw: info.is_raw,
        is_default_playback: info.is_default_playback,
        is_default_capture: info.is_default_capture,
        is_default_loopback: info.is_default_loopback,
        capability_flags: info.capability_flags,
        supported_device_open_flags: supported_device_open_flags_for_device(info),
        supported_stream_flags: supported_backend_stream_flags(info.backend),
        supported_stream_requirement_flags: supported_backend_stream_requirement_flags(
            info.backend,
        ),
        supported_event_subscription_flags: supported_backend_event_subscription_flags(
            info.backend,
        ),
        supported_stream_clock_domains: supported_stream_clock_domains_for_device(info),
        preferred_sample_rate: info.preferred_sample_rate,
        min_sample_rate: info.min_sample_rate,
        max_sample_rate: info.max_sample_rate,
        preferred_period_frames: info.preferred_period_frames,
        min_channels: info.min_channels,
        max_channels: info.max_channels,
        preferred_layout: info.preferred_layout,
        preferred_channel_mask: info.preferred_channel_mask,
        supported_channel_mask: info.supported_channel_mask,
        min_period_frames: info.min_period_frames,
        max_period_frames: info.max_period_frames,
        format_mask: info.format_mask,
        share_mode_mask: info.share_mode_mask,
    }
}

/// Return one descriptor snapshot for one open device handle.
pub(crate) fn descriptor_from_binding(
    binding_2: &BindingCallContext,
    binding: &AudioDeviceBinding,
) -> AudioDeviceDescriptor {
    let mut info = binding.info.clone();
    info.direction = binding.opened_direction;
    descriptor_from_info(binding_2, &info)
}

/// Build one stream-device descriptor using the handle open direction.
pub(crate) fn stream_device_from_binding(binding: &AudioDeviceBinding) -> HostDeviceDescriptor {
    let mut device = binding.info.clone();
    device.direction = binding.opened_direction;
    device
}
