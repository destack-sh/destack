use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::constants::{
    AAUDIO_DEFAULT_ENDPOINT_NAME, AAUDIO_MAX_CHANNELS, AAUDIO_MAX_PERIOD_FRAMES,
    AAUDIO_MAX_SAMPLE_RATE, AAUDIO_MIN_PERIOD_FRAMES, AAUDIO_MIN_SAMPLE_RATE,
    AAUDIO_PREFERRED_PERIOD_FRAMES, AAUDIO_PREFERRED_SAMPLE_RATE,
};
use super::core::{aaudio_format_mask, channel_layout, channel_mask, require_aaudio_library};
use super::ids::{capture_stable_id, duplex_stable_id, playback_stable_id};

/// Enumerate AAudio devices from one default endpoint profile.
pub(super) fn enumerate_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    let _ = require_aaudio_library("destack.audio.device.list")?;

    let channels = 2u16;
    let preferred_layout = channel_layout(channels);
    let preferred_mask = channel_mask(channels);
    let supported_mask = channel_mask(AAUDIO_MAX_CHANNELS);
    let format_mask = aaudio_format_mask();
    let share_mode_mask = audio_core::SHARE_MODE_SHARED_BIT | audio_core::SHARE_MODE_EXCLUSIVE_BIT;
    let base_flags = audio_core::DEVICE_CAPABILITY_SHARED_MODE.0
        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
        | audio_core::DEVICE_CAPABILITY_REROUTE_EVENTS.0
        | audio_core::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0;
    #[cfg(all(target_os = "android", feature = "audio-aaudio"))]
    let base_flags = base_flags | audio_core::DEVICE_CAPABILITY_EXCLUSIVE_MODE.0;

    let playback = audio_core::HostDeviceDescriptor {
        id: playback_stable_id(AAUDIO_DEFAULT_ENDPOINT_NAME),
        group_id: String::from("aaudio-group:default"),
        name: String::from("AAudio Playback"),
        transport: String::from("aaudio"),
        backend: audio_core::AudioBackend::AAudio,
        direction: audio_core::AudioDeviceDirection::Playback,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK,
        connected: true,
        is_raw: false,
        is_default_playback: true,
        is_default_capture: false,
        is_default_loopback: false,
        capability_flags: audio_core::AudioDeviceCapabilityFlags(base_flags),
        preferred_sample_rate: AAUDIO_PREFERRED_SAMPLE_RATE,
        min_sample_rate: AAUDIO_MIN_SAMPLE_RATE,
        max_sample_rate: AAUDIO_MAX_SAMPLE_RATE,
        preferred_period_frames: AAUDIO_PREFERRED_PERIOD_FRAMES,
        min_channels: 1,
        max_channels: AAUDIO_MAX_CHANNELS,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: AAUDIO_MIN_PERIOD_FRAMES,
        max_period_frames: AAUDIO_MAX_PERIOD_FRAMES,
        format_mask,
        share_mode_mask,
        is_null: false,
    };

    let capture = audio_core::HostDeviceDescriptor {
        id: capture_stable_id(AAUDIO_DEFAULT_ENDPOINT_NAME),
        group_id: String::from("aaudio-group:default"),
        name: String::from("AAudio Capture"),
        transport: String::from("aaudio"),
        backend: audio_core::AudioBackend::AAudio,
        direction: audio_core::AudioDeviceDirection::Capture,
        supported_directions: audio_core::DIRECTION_MASK_CAPTURE,
        connected: true,
        is_raw: false,
        is_default_playback: false,
        is_default_capture: true,
        is_default_loopback: false,
        capability_flags: audio_core::AudioDeviceCapabilityFlags(base_flags),
        preferred_sample_rate: AAUDIO_PREFERRED_SAMPLE_RATE,
        min_sample_rate: AAUDIO_MIN_SAMPLE_RATE,
        max_sample_rate: AAUDIO_MAX_SAMPLE_RATE,
        preferred_period_frames: AAUDIO_PREFERRED_PERIOD_FRAMES,
        min_channels: 1,
        max_channels: AAUDIO_MAX_CHANNELS,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: AAUDIO_MIN_PERIOD_FRAMES,
        max_period_frames: AAUDIO_MAX_PERIOD_FRAMES,
        format_mask,
        share_mode_mask,
        is_null: false,
    };

    let duplex = audio_core::HostDeviceDescriptor {
        id: duplex_stable_id(AAUDIO_DEFAULT_ENDPOINT_NAME, AAUDIO_DEFAULT_ENDPOINT_NAME),
        group_id: String::from("aaudio-group:default"),
        name: String::from("AAudio Duplex"),
        transport: String::from("aaudio"),
        backend: audio_core::AudioBackend::AAudio,
        direction: audio_core::AudioDeviceDirection::Duplex,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK
            | audio_core::DIRECTION_MASK_CAPTURE
            | audio_core::DIRECTION_MASK_DUPLEX,
        connected: true,
        is_raw: false,
        is_default_playback: true,
        is_default_capture: true,
        is_default_loopback: false,
        capability_flags: audio_core::AudioDeviceCapabilityFlags(
            base_flags | audio_core::DEVICE_CAPABILITY_FULL_DUPLEX.0,
        ),
        preferred_sample_rate: AAUDIO_PREFERRED_SAMPLE_RATE,
        min_sample_rate: AAUDIO_MIN_SAMPLE_RATE,
        max_sample_rate: AAUDIO_MAX_SAMPLE_RATE,
        preferred_period_frames: AAUDIO_PREFERRED_PERIOD_FRAMES,
        min_channels: 1,
        max_channels: AAUDIO_MAX_CHANNELS,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: AAUDIO_MIN_PERIOD_FRAMES,
        max_period_frames: AAUDIO_MAX_PERIOD_FRAMES,
        format_mask,
        share_mode_mask,
        is_null: false,
    };

    Ok(vec![playback, capture, duplex])
}
