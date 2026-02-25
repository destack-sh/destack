use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::constants::ASIO_MAX_PROBED_CHANNELS;
use super::host::{
    channel_layout, channel_mask, enumerate_registered_drivers, probe_device_profile,
};
use super::ids::{capture_stable_id, duplex_stable_id, playback_stable_id};

/// Enumerate ASIO drivers and normalize them into host descriptors.
pub(super) fn enumerate_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    let rows = enumerate_registered_drivers();
    if rows.is_empty() {
        return Err(audio_core::audio_not_found(
            "destack.audio.device.list",
            "no ASIO drivers are registered",
        ));
    }

    let mut descriptors = Vec::new();

    for (index, row) in rows.iter().enumerate() {
        // probe one device profile for this registered driver
        let profile = probe_device_profile(row);

        let is_default = index == 0;
        let minimum_channels = profile
            .input_channels
            .min(profile.output_channels)
            .clamp(1, ASIO_MAX_PROBED_CHANNELS);
        let maximum_channels = profile
            .input_channels
            .max(profile.output_channels)
            .clamp(1, ASIO_MAX_PROBED_CHANNELS);

        let preferred_channels = if profile.output_channels > 0 {
            profile.output_channels
        } else {
            profile.input_channels
        }
        .clamp(1, ASIO_MAX_PROBED_CHANNELS);

        // append one playback descriptor row when output channels exist
        if profile.output_channels > 0 {
            descriptors.push(audio_core::HostDeviceDescriptor {
                id: playback_stable_id(&row.key_name),
                group_id: format!("asio-group:{}", row.key_name),
                name: row.display_name.clone(),
                transport: "asio".to_string(),
                backend: audio_core::AudioBackend::Asio,
                direction: audio_core::AudioDeviceDirection::Playback,
                supported_directions: audio_core::DIRECTION_MASK_PLAYBACK,
                connected: true,
                is_raw: true,
                is_default_playback: is_default,
                is_default_capture: false,
                is_default_loopback: false,
                capability_flags: audio_core::AudioDeviceCapabilityFlags(
                    audio_core::DEVICE_CAPABILITY_EXCLUSIVE_MODE.0
                        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
                        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
                        | audio_core::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0,
                ),
                preferred_sample_rate: profile.preferred_sample_rate,
                min_sample_rate: profile.min_sample_rate,
                max_sample_rate: profile.max_sample_rate,
                preferred_period_frames: profile.preferred_period_frames,
                min_channels: 1,
                max_channels: profile.output_channels.clamp(1, ASIO_MAX_PROBED_CHANNELS),
                preferred_layout: channel_layout(profile.output_channels.max(1)),
                preferred_channel_mask: channel_mask(profile.output_channels.max(1)),
                supported_channel_mask: channel_mask(profile.output_channels.max(1)),
                min_period_frames: profile.min_period_frames,
                max_period_frames: profile.max_period_frames,
                format_mask: profile.format_mask,
                share_mode_mask: audio_core::SHARE_MODE_EXCLUSIVE_BIT,
                is_null: false,
            });
        }

        // append one capture descriptor row when input channels exist
        if profile.input_channels > 0 {
            descriptors.push(audio_core::HostDeviceDescriptor {
                id: capture_stable_id(&row.key_name),
                group_id: format!("asio-group:{}", row.key_name),
                name: row.display_name.clone(),
                transport: "asio".to_string(),
                backend: audio_core::AudioBackend::Asio,
                direction: audio_core::AudioDeviceDirection::Capture,
                supported_directions: audio_core::DIRECTION_MASK_CAPTURE,
                connected: true,
                is_raw: true,
                is_default_playback: false,
                is_default_capture: is_default,
                is_default_loopback: false,
                capability_flags: audio_core::AudioDeviceCapabilityFlags(
                    audio_core::DEVICE_CAPABILITY_EXCLUSIVE_MODE.0
                        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
                        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
                        | audio_core::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0,
                ),
                preferred_sample_rate: profile.preferred_sample_rate,
                min_sample_rate: profile.min_sample_rate,
                max_sample_rate: profile.max_sample_rate,
                preferred_period_frames: profile.preferred_period_frames,
                min_channels: 1,
                max_channels: profile.input_channels.clamp(1, ASIO_MAX_PROBED_CHANNELS),
                preferred_layout: channel_layout(profile.input_channels.max(1)),
                preferred_channel_mask: channel_mask(profile.input_channels.max(1)),
                supported_channel_mask: channel_mask(profile.input_channels.max(1)),
                min_period_frames: profile.min_period_frames,
                max_period_frames: profile.max_period_frames,
                format_mask: profile.format_mask,
                share_mode_mask: audio_core::SHARE_MODE_EXCLUSIVE_BIT,
                is_null: false,
            });
        }

        // append one duplex descriptor row when both lanes are present
        if profile.input_channels > 0 && profile.output_channels > 0 {
            descriptors.push(audio_core::HostDeviceDescriptor {
                id: duplex_stable_id(&row.key_name),
                group_id: format!("asio-group:{}", row.key_name),
                name: row.display_name.clone(),
                transport: "asio".to_string(),
                backend: audio_core::AudioBackend::Asio,
                direction: audio_core::AudioDeviceDirection::Duplex,
                supported_directions: audio_core::DIRECTION_MASK_DUPLEX,
                connected: true,
                is_raw: true,
                is_default_playback: is_default,
                is_default_capture: is_default,
                is_default_loopback: false,
                capability_flags: audio_core::AudioDeviceCapabilityFlags(
                    audio_core::DEVICE_CAPABILITY_EXCLUSIVE_MODE.0
                        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
                        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
                        | audio_core::DEVICE_CAPABILITY_FULL_DUPLEX.0
                        | audio_core::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0,
                ),
                preferred_sample_rate: profile.preferred_sample_rate,
                min_sample_rate: profile.min_sample_rate,
                max_sample_rate: profile.max_sample_rate,
                preferred_period_frames: profile.preferred_period_frames,
                min_channels: minimum_channels,
                max_channels: maximum_channels,
                preferred_layout: channel_layout(preferred_channels),
                preferred_channel_mask: channel_mask(preferred_channels),
                supported_channel_mask: channel_mask(maximum_channels),
                min_period_frames: profile.min_period_frames,
                max_period_frames: profile.max_period_frames,
                format_mask: profile.format_mask,
                share_mode_mask: audio_core::SHARE_MODE_EXCLUSIVE_BIT,
                is_null: false,
            });
        }
    }

    if descriptors.is_empty() {
        return Err(audio_core::audio_not_found(
            "destack.audio.device.list",
            "ASIO driver set contains no usable endpoints",
        ));
    }

    Ok(descriptors)
}
