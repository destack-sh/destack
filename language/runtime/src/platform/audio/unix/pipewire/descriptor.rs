use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::constants::{
    PIPEWIRE_MAX_CHANNELS, PIPEWIRE_MAX_PERIOD_FRAMES, PIPEWIRE_MAX_SAMPLE_RATE,
    PIPEWIRE_MIN_PERIOD_FRAMES, PIPEWIRE_MIN_SAMPLE_RATE, PIPEWIRE_PREFERRED_PERIOD_FRAMES,
    PIPEWIRE_PREFERRED_SAMPLE_RATE,
};
use super::core::{
    channel_layout, channel_mask, pipewire_format_mask, probe_endpoints, require_pipewire_library,
};
use super::ids::{capture_stable_id, duplex_stable_id, loopback_stable_id, playback_stable_id};
use crate::platform::audio as audio_types;

/// Enumerate PipeWire devices from one probed endpoint snapshot.
pub(super) fn enumerate_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    let _ = require_pipewire_library("destack.audio.device.list")?;

    // probe one live endpoint table and return zero rows when host probing has no data
    let Some(endpoints) = probe_endpoints() else {
        return Ok(Vec::new());
    };
    let mut descriptors = Vec::new();

    // emit one playback row for each sink endpoint
    for playback_name in &endpoints.playback_names {
        let is_default = playback_name == &endpoints.default_playback_name;
        descriptors.push(playback_descriptor(playback_name, is_default));
    }

    // emit one capture row for each source endpoint
    for capture_name in &endpoints.capture_names {
        let is_default = capture_name == &endpoints.default_capture_name;
        descriptors.push(capture_descriptor(capture_name, is_default));
    }

    // emit one loopback row for each monitor source endpoint
    for loopback_name in &endpoints.loopback_names {
        let is_default = loopback_name == &endpoints.default_loopback_name;
        descriptors.push(loopback_descriptor(loopback_name, is_default));
    }

    // emit one duplex row for the default sink and source pair
    if !endpoints.playback_names.is_empty() && !endpoints.capture_names.is_empty() {
        descriptors.push(duplex_descriptor(
            &endpoints.default_playback_name,
            &endpoints.default_capture_name,
        ));
    }

    Ok(descriptors)
}

/// Build one shared capability flag mask for one PipeWire stream lane.
fn base_capability_flags() -> audio_types::AudioDeviceCapabilityFlags {
    let flags = audio_core::DEVICE_CAPABILITY_SHARED_MODE.0
        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
        | audio_core::DEVICE_CAPABILITY_REROUTE_EVENTS.0
        | audio_core::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0;

    audio_types::AudioDeviceCapabilityFlags(flags)
}

/// Build one playback descriptor row for one endpoint name.
fn playback_descriptor(name: &str, is_default: bool) -> audio_core::HostDeviceDescriptor {
    let (preferred_layout, preferred_mask, supported_mask, format_mask) = descriptor_profile();
    let capability_flags = base_capability_flags();

    audio_core::HostDeviceDescriptor {
        id: playback_stable_id(name),
        group_id: format!("pipewire-group:{name}"),
        name: format!("PipeWire Playback ({name})"),
        transport: String::from("pipewire"),
        backend: audio_types::AudioBackend::PipeWire,
        direction: audio_types::AudioDeviceDirection::Playback,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK,
        connected: true,
        is_raw: false,
        is_default_playback: is_default,
        is_default_capture: false,
        is_default_loopback: false,
        capability_flags,
        preferred_sample_rate: PIPEWIRE_PREFERRED_SAMPLE_RATE,
        min_sample_rate: PIPEWIRE_MIN_SAMPLE_RATE,
        max_sample_rate: PIPEWIRE_MAX_SAMPLE_RATE,
        preferred_period_frames: PIPEWIRE_PREFERRED_PERIOD_FRAMES,
        min_channels: 1,
        max_channels: PIPEWIRE_MAX_CHANNELS,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: PIPEWIRE_MIN_PERIOD_FRAMES,
        max_period_frames: PIPEWIRE_MAX_PERIOD_FRAMES,
        format_mask,
        share_mode_mask: audio_core::SHARE_MODE_SHARED_BIT,
        is_null: false,
    }
}

/// Build one capture descriptor row for one endpoint name.
fn capture_descriptor(name: &str, is_default: bool) -> audio_core::HostDeviceDescriptor {
    let (preferred_layout, preferred_mask, supported_mask, format_mask) = descriptor_profile();
    let capability_flags = base_capability_flags();

    audio_core::HostDeviceDescriptor {
        id: capture_stable_id(name),
        group_id: format!("pipewire-group:{name}"),
        name: format!("PipeWire Capture ({name})"),
        transport: String::from("pipewire"),
        backend: audio_types::AudioBackend::PipeWire,
        direction: audio_types::AudioDeviceDirection::Capture,
        supported_directions: audio_core::DIRECTION_MASK_CAPTURE,
        connected: true,
        is_raw: false,
        is_default_playback: false,
        is_default_capture: is_default,
        is_default_loopback: false,
        capability_flags,
        preferred_sample_rate: PIPEWIRE_PREFERRED_SAMPLE_RATE,
        min_sample_rate: PIPEWIRE_MIN_SAMPLE_RATE,
        max_sample_rate: PIPEWIRE_MAX_SAMPLE_RATE,
        preferred_period_frames: PIPEWIRE_PREFERRED_PERIOD_FRAMES,
        min_channels: 1,
        max_channels: PIPEWIRE_MAX_CHANNELS,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: PIPEWIRE_MIN_PERIOD_FRAMES,
        max_period_frames: PIPEWIRE_MAX_PERIOD_FRAMES,
        format_mask,
        share_mode_mask: audio_core::SHARE_MODE_SHARED_BIT,
        is_null: false,
    }
}

/// Build one loopback descriptor row for one monitor endpoint name.
fn loopback_descriptor(name: &str, is_default: bool) -> audio_core::HostDeviceDescriptor {
    let (preferred_layout, preferred_mask, supported_mask, format_mask) = descriptor_profile();
    let capability_flags = audio_types::AudioDeviceCapabilityFlags(
        base_capability_flags().0 | audio_core::DEVICE_CAPABILITY_LOOPBACK.0,
    );

    audio_core::HostDeviceDescriptor {
        id: loopback_stable_id(name),
        group_id: format!("pipewire-group:{name}"),
        name: format!("PipeWire Loopback ({name})"),
        transport: String::from("pipewire"),
        backend: audio_types::AudioBackend::PipeWire,
        direction: audio_types::AudioDeviceDirection::Loopback,
        supported_directions: audio_core::DIRECTION_MASK_LOOPBACK,
        connected: true,
        is_raw: false,
        is_default_playback: false,
        is_default_capture: false,
        is_default_loopback: is_default,
        capability_flags,
        preferred_sample_rate: PIPEWIRE_PREFERRED_SAMPLE_RATE,
        min_sample_rate: PIPEWIRE_MIN_SAMPLE_RATE,
        max_sample_rate: PIPEWIRE_MAX_SAMPLE_RATE,
        preferred_period_frames: PIPEWIRE_PREFERRED_PERIOD_FRAMES,
        min_channels: 1,
        max_channels: PIPEWIRE_MAX_CHANNELS,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: PIPEWIRE_MIN_PERIOD_FRAMES,
        max_period_frames: PIPEWIRE_MAX_PERIOD_FRAMES,
        format_mask,
        share_mode_mask: audio_core::SHARE_MODE_SHARED_BIT,
        is_null: false,
    }
}

/// Build one duplex descriptor row for one playback and capture endpoint pair.
fn duplex_descriptor(playback_name: &str, capture_name: &str) -> audio_core::HostDeviceDescriptor {
    let (preferred_layout, preferred_mask, supported_mask, format_mask) = descriptor_profile();
    let capability_flags = audio_types::AudioDeviceCapabilityFlags(
        base_capability_flags().0 | audio_core::DEVICE_CAPABILITY_FULL_DUPLEX.0,
    );

    audio_core::HostDeviceDescriptor {
        id: duplex_stable_id(playback_name, capture_name),
        group_id: format!("pipewire-group:{playback_name}|{capture_name}"),
        name: format!("PipeWire Duplex ({playback_name}, {capture_name})"),
        transport: String::from("pipewire"),
        backend: audio_types::AudioBackend::PipeWire,
        direction: audio_types::AudioDeviceDirection::Duplex,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK
            | audio_core::DIRECTION_MASK_CAPTURE
            | audio_core::DIRECTION_MASK_DUPLEX,
        connected: true,
        is_raw: false,
        is_default_playback: true,
        is_default_capture: true,
        is_default_loopback: false,
        capability_flags,
        preferred_sample_rate: PIPEWIRE_PREFERRED_SAMPLE_RATE,
        min_sample_rate: PIPEWIRE_MIN_SAMPLE_RATE,
        max_sample_rate: PIPEWIRE_MAX_SAMPLE_RATE,
        preferred_period_frames: PIPEWIRE_PREFERRED_PERIOD_FRAMES,
        min_channels: 1,
        max_channels: PIPEWIRE_MAX_CHANNELS,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: PIPEWIRE_MIN_PERIOD_FRAMES,
        max_period_frames: PIPEWIRE_MAX_PERIOD_FRAMES,
        format_mask,
        share_mode_mask: audio_core::SHARE_MODE_SHARED_BIT,
        is_null: false,
    }
}

/// Build one shared descriptor profile for PipeWire rows.
fn descriptor_profile() -> (audio_types::AudioChannelLayout, u64, u64, u32) {
    let channels = 2u16;
    let preferred_layout = channel_layout(channels);
    let preferred_mask = channel_mask(channels);
    let supported_mask = channel_mask(PIPEWIRE_MAX_CHANNELS);
    let format_mask = pipewire_format_mask();

    (
        preferred_layout,
        preferred_mask,
        supported_mask,
        format_mask,
    )
}
