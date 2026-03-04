#[cfg(not(target_os = "macos"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "macos")]
use super::constants::{
    K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE, K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_ALIVE,
    K_AUDIO_DEVICE_PROPERTY_DEVICE_UID, K_AUDIO_DEVICE_PROPERTY_MODEL_UID,
    K_AUDIO_DEVICE_PROPERTY_NOMINAL_SAMPLE_RATE, K_AUDIO_DEVICE_PROPERTY_TRANSPORT_TYPE,
    K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE,
    K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE,
    K_AUDIO_HARDWARE_PROPERTY_DEFAULT_SYSTEM_OUTPUT_DEVICE, K_AUDIO_OBJECT_PROPERTY_NAME,
    K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT,
    K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT, K_FALLBACK_PERIOD_FRAMES, K_FALLBACK_SAMPLE_RATE,
};
#[cfg(target_os = "macos")]
use super::format::{
    channel_layout, channel_mask, direction_from_channels, scope_for_direction, transport_name,
};
#[cfg(target_os = "macos")]
use super::probe::probe_loopback_support;
#[cfg(target_os = "macos")]
use super::property::{
    default_device_id, device_ids, get_buffer_frame_size_range, get_cfstring_optional,
    get_sample_rate_range, get_scalar_optional, get_stream_channel_count, hog_mode_allowed,
    rate_to_u32,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;
#[cfg(target_os = "macos")]
use crate::platform::core as core_platform;

/// Enumerate one normalized CoreAudio device list on macOS.
#[cfg(target_os = "macos")]
fn enumerate_host_devices_macos() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    // load the current system device list
    let device_ids = device_ids()?;
    if device_ids.is_empty() {
        return Err(core_platform::io_not_found(
            "destack.audio.device.list",
            "CoreAudio reported no devices",
        ));
    }

    // resolve system defaults once before per-device normalization
    let default_output = default_device_id(K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE);
    let default_input = default_device_id(K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE);
    let default_system_output =
        default_device_id(K_AUDIO_HARDWARE_PROPERTY_DEFAULT_SYSTEM_OUTPUT_DEVICE);
    let supports_hog_mode = hog_mode_allowed();
    let mut descriptors = Vec::with_capacity(device_ids.len());

    // normalize each CoreAudio device into one host descriptor row
    for device_id in device_ids {
        // probe directional channel counts and skip unusable rows
        let playback_channels =
            get_stream_channel_count(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_OUTPUT).unwrap_or(0);
        let capture_channels =
            get_stream_channel_count(device_id, K_AUDIO_OBJECT_PROPERTY_SCOPE_INPUT).unwrap_or(0);
        let direction = match direction_from_channels(playback_channels, capture_channels) {
            Some(value) => value,
            None => continue,
        };

        // resolve rate and period ranges for the selected direction scope
        let scope = scope_for_direction(direction);
        let nominal_rate = get_scalar_optional::<f64>(
            device_id,
            K_AUDIO_DEVICE_PROPERTY_NOMINAL_SAMPLE_RATE,
            scope,
        )
        .and_then(rate_to_u32)
        .unwrap_or(K_FALLBACK_SAMPLE_RATE);
        let sample_rate_range =
            get_sample_rate_range(device_id, scope).unwrap_or((nominal_rate, nominal_rate));
        let preferred_period =
            get_scalar_optional::<u32>(device_id, K_AUDIO_DEVICE_PROPERTY_BUFFER_FRAME_SIZE, scope)
                .unwrap_or(K_FALLBACK_PERIOD_FRAMES);
        let period_range = get_buffer_frame_size_range(device_id, scope)
            .unwrap_or((preferred_period, preferred_period));

        // resolve identity and transport metadata
        let channel_cap = playback_channels.max(capture_channels).max(1);
        let name = get_cfstring_optional(
            device_id,
            K_AUDIO_OBJECT_PROPERTY_NAME,
            K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        )
        .unwrap_or_else(|| format!("CoreAudio Device {device_id}"));
        let uid = get_cfstring_optional(
            device_id,
            K_AUDIO_DEVICE_PROPERTY_DEVICE_UID,
            K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        )
        .unwrap_or_else(|| format!("{device_id}"));
        let model_uid = get_cfstring_optional(
            device_id,
            K_AUDIO_DEVICE_PROPERTY_MODEL_UID,
            K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        )
        .unwrap_or_else(|| uid.clone());
        let transport_type = get_scalar_optional::<u32>(
            device_id,
            K_AUDIO_DEVICE_PROPERTY_TRANSPORT_TYPE,
            K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        )
        .unwrap_or(0);
        let is_alive = get_scalar_optional::<u32>(
            device_id,
            K_AUDIO_DEVICE_PROPERTY_DEVICE_IS_ALIVE,
            K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
        )
        .unwrap_or(1)
            != 0;

        // derive stable identifiers and default flags
        let stable_id = format!("coreaudio:{uid}");
        let stable_group_id = format!("coreaudio-group:{model_uid}");
        let is_default_playback =
            default_output == Some(device_id) || default_system_output == Some(device_id);
        let is_default_capture = default_input == Some(device_id);
        let is_default_loopback = default_system_output == Some(device_id);

        // compute derived capability and format metadata
        let supports_loopback = playback_channels > 0 && probe_loopback_support(device_id);
        let min_sample_rate = sample_rate_range.0.max(1);
        let max_sample_rate = sample_rate_range.1.max(min_sample_rate);
        let min_period_frames = period_range.0.max(1);
        let max_period_frames = period_range.1.max(min_period_frames);
        let preferred_layout = channel_layout(channel_cap);
        let preferred_mask = channel_mask(channel_cap);

        // start with common runtime capabilities
        let mut capability_flags = audio_core::DEVICE_CAPABILITY_SHARED_MODE.0
            | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
            | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
            | audio_core::DEVICE_CAPABILITY_DEVICE_CLOCK.0
            | audio_core::DEVICE_CAPABILITY_REROUTE_EVENTS.0;
        if supports_hog_mode {
            capability_flags |= audio_core::DEVICE_CAPABILITY_EXCLUSIVE_MODE.0;
        }

        // encode supported directions and direction-specific capabilities
        let mut supported_directions = 0u32;
        if playback_channels > 0 {
            supported_directions |= audio_core::DIRECTION_MASK_PLAYBACK;
            capability_flags |= audio_core::DEVICE_CAPABILITY_SCHEDULED_WRITE.0;
            if supports_loopback {
                supported_directions |= audio_core::DIRECTION_MASK_LOOPBACK;
                capability_flags |= audio_core::DEVICE_CAPABILITY_LOOPBACK.0;
            }
        }
        if capture_channels > 0 {
            supported_directions |= audio_core::DIRECTION_MASK_CAPTURE;
        }
        if direction == audio_core::AudioDeviceDirection::Duplex {
            capability_flags |= audio_core::DEVICE_CAPABILITY_FULL_DUPLEX.0;
            supported_directions |= audio_core::DIRECTION_MASK_DUPLEX;
        }

        // emit one normalized descriptor row
        descriptors.push(audio_core::HostDeviceDescriptor {
            id: stable_id,
            group_id: stable_group_id,
            name,
            transport: transport_name(transport_type).to_string(),
            backend: audio_core::AudioBackend::CoreAudio,
            direction,
            supported_directions,
            connected: is_alive,
            is_raw: false,
            is_default_playback,
            is_default_capture,
            is_default_loopback,
            capability_flags: audio_core::AudioDeviceCapabilityFlags(capability_flags),
            preferred_sample_rate: nominal_rate,
            min_sample_rate,
            max_sample_rate,
            preferred_period_frames: preferred_period.max(min_period_frames),
            min_channels: 1,
            max_channels: channel_cap,
            preferred_layout,
            preferred_channel_mask: preferred_mask,
            supported_channel_mask: preferred_mask,
            min_period_frames,
            max_period_frames,
            format_mask: audio_core::all_sample_format_mask(),
            share_mode_mask: if supports_hog_mode {
                audio_core::SHARE_MODE_SHARED_BIT | audio_core::SHARE_MODE_EXCLUSIVE_BIT
            } else {
                audio_core::SHARE_MODE_SHARED_BIT
            },
            is_null: false,
        });
    }

    // ensure one usable row exists after normalization
    if descriptors.is_empty() {
        return Err(core_platform::io_not_found(
            "destack.audio.device.list",
            "CoreAudio reported no usable devices",
        ));
    }

    Ok(descriptors)
}

/// Enumerate CoreAudio devices.
pub(crate) fn enumerate_host_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    #[cfg(target_os = "macos")]
    {
        enumerate_host_devices_macos()
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(backend_not_supported(
            "destack.audio.device.list",
            "coreaudio",
        ))
    }
}
