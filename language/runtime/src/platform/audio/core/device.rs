use super::*;
use crate::platform::audio::host;

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
        | DEVICE_CAPABILITY_REROUTE_EVENTS.0
        | DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0;
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

/// Enumerate all devices visible to the request.
pub(crate) fn enumerate_devices_for_request(
    request: AudioDeviceListRequest,
) -> RuntimeResult<Vec<HostDeviceDescriptor>> {
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
