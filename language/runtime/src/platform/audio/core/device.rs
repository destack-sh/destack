use super::*;
use crate::platform::audio::host;

/// Mask for all recognized backend-open flags.
const KNOWN_BACKEND_OPEN_FLAGS_MASK: u64 = BACKEND_OPEN_WASAPI_EVENT_CALLBACK.0
    | BACKEND_OPEN_REQUIRE_EXCLUSIVE.0
    | BACKEND_OPEN_REQUIRE_LOOPBACK.0
    | BACKEND_OPEN_JACK_NO_AUTOCONNECT.0
    | BACKEND_OPEN_ALSA_NO_RESAMPLE.0
    | BACKEND_OPEN_COREAUDIO_HOG_MODE.0
    | BACKEND_OPEN_REQUIRE_HARDWARE_TIMESTAMPS.0
    | BACKEND_OPEN_REQUIRE_BIT_EXACT_PCM.0;

/// Return whether one backend exposes loopback streams in the common surface.
fn backend_supports_loopback(backend: AudioBackend) -> bool {
    matches!(
        backend,
        AudioBackend::Null
            | AudioBackend::Wasapi
            | AudioBackend::CoreAudio
            | AudioBackend::PipeWire
            | AudioBackend::PulseAudio
    )
}

/// Return whether one backend exposes device-clock or hardware timestamp support.
fn backend_supports_device_clock(backend: AudioBackend) -> bool {
    matches!(
        backend,
        AudioBackend::Null | AudioBackend::Wasapi | AudioBackend::CoreAudio
    )
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

/// Return whether one backend-open flag bit is enabled.
fn backend_open_flag_enabled(flags: AudioBackendOpenFlags, flag: AudioBackendOpenFlags) -> bool {
    (flags.0 & flag.0) != 0
}

/// Validate and normalize one device-open option payload for one backend.
pub(crate) fn normalize_device_open_options(
    mut options: AudioDeviceOpenOptions,
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

    // reject device-open flags that are not wired in host implementations yet
    if options.flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} device open flags are not implemented yet",
        )))
        .boxed());
    }

    // reject unknown backend flag bits
    let unknown_flags = options.backend_flags.0 & !KNOWN_BACKEND_OPEN_FLAGS_MASK;
    if unknown_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "options.backendFlags",
            format!("options.backendFlags contains unknown bits: 0x{unknown_flags:016x}"),
        ))
        .boxed());
    }

    // gate backend-specific flags to the backends that implement them
    if backend_open_flag_enabled(options.backend_flags, BACKEND_OPEN_WASAPI_EVENT_CALLBACK)
        && backend != AudioBackend::Wasapi
    {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} backend flag: wasapi event callback",
        )))
        .boxed());
    }

    // gate backend-specific flags to the backends that implement them
    if backend_open_flag_enabled(options.backend_flags, BACKEND_OPEN_JACK_NO_AUTOCONNECT)
        && backend != AudioBackend::Jack
    {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} backend flag: jack no autoconnect",
        )))
        .boxed());
    }

    if backend_open_flag_enabled(options.backend_flags, BACKEND_OPEN_ALSA_NO_RESAMPLE)
        && backend != AudioBackend::Alsa
    {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} backend flag: alsa no resample",
        )))
        .boxed());
    }

    if backend_open_flag_enabled(options.backend_flags, BACKEND_OPEN_COREAUDIO_HOG_MODE)
        && backend != AudioBackend::CoreAudio
    {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} backend flag: coreaudio hog mode",
        )))
        .boxed());
    }

    if backend_open_flag_enabled(
        options.backend_flags,
        BACKEND_OPEN_REQUIRE_HARDWARE_TIMESTAMPS,
    ) {
        if !backend_supports_device_clock(backend) {
            return Err(RuntimeError::from(PlatformError::not_supported(format!(
                "{operation} backend flag: hardware timestamps",
            )))
            .boxed());
        }
    }

    if backend_open_flag_enabled(options.backend_flags, BACKEND_OPEN_REQUIRE_BIT_EXACT_PCM) {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation} backend flag: bit exact pcm",
        )))
        .boxed());
    }

    // require explicit loopback direction for loopback-only requests
    if backend_open_flag_enabled(options.backend_flags, BACKEND_OPEN_REQUIRE_LOOPBACK) {
        if options.direction != AudioDeviceDirection::Loopback {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "options.direction",
                "loopback backend flag requires options.direction loopback",
            ))
            .boxed());
        }

        if !backend_supports_loopback(backend) {
            return Err(RuntimeError::from(PlatformError::not_supported(format!(
                "{operation} backend flag: loopback",
            )))
            .boxed());
        }
    }

    // force exclusive share mode for exclusive and hog-mode requests
    if backend_open_flag_enabled(options.backend_flags, BACKEND_OPEN_REQUIRE_EXCLUSIVE)
        || backend_open_flag_enabled(options.backend_flags, BACKEND_OPEN_COREAUDIO_HOG_MODE)
    {
        options.share_mode = AudioShareMode::Exclusive;
    }

    Ok(options)
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

    // reject list flags that are not wired in host implementations yet
    if request.flags.0 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.device.list flags",
        ))
        .boxed());
    }

    let backend = host::resolve_requested_backend(
        request.backend,
        request.backend_policy,
        "destack.audio.device.list",
    )?;

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
        match request.direction {
            AudioDeviceDirection::Playback => {
                supports_device_open_direction(device, AudioDeviceDirection::Playback)
            }
            AudioDeviceDirection::Capture => {
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
    context: &BindingCallContext,
    info: &HostDeviceDescriptor,
) -> AudioDeviceDescriptor {
    AudioDeviceDescriptor {
        id: context.store_string(&info.id),
        group_id: context.store_string(&info.group_id),
        name: context.store_string(&info.name),
        transport: context.store_string(&info.transport),
        backend: info.backend,
        direction: info.direction,
        connected: info.connected,
        is_raw: info.is_raw,
        is_default_playback: info.is_default_playback,
        is_default_capture: info.is_default_capture,
        is_default_loopback: info.is_default_loopback,
        capability_flags: info.capability_flags,
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
    context: &BindingCallContext,
    binding: &AudioDeviceBinding,
) -> AudioDeviceDescriptor {
    let mut descriptor = descriptor_from_info(context, &binding.info);
    descriptor.direction = binding.opened_direction;
    descriptor
}

/// Build one stream-device descriptor using the handle open direction.
pub(crate) fn stream_device_from_binding(binding: &AudioDeviceBinding) -> HostDeviceDescriptor {
    let mut device = binding.info.clone();
    device.direction = binding.opened_direction;
    device
}
