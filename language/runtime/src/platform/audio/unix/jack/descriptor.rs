use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::codec::sample_format_bit;
use crate::platform::core as core_platform;

use super::core::{JackEndpointSnapshot, channel_layout, channel_mask, probe_jack_endpoints};
use super::ids::{capture_stable_id, duplex_stable_id, playback_stable_id};
use crate::platform::audio as audio_types;

/// Enumerate JACK devices through one native endpoint probe.
pub(super) fn enumerate_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    let snapshot = probe_jack_endpoints("destack.audio.device.list")?;
    let mut descriptors = Vec::new();

    // emit one playback descriptor row when one physical sink exists
    if !snapshot.playback_sinks.is_empty() {
        let playback_channels = snapshot.playback_sinks.len().min(u16::MAX as usize) as u16;
        descriptors.push(playback_descriptor(&snapshot, playback_channels));
    }

    // emit one capture descriptor row when one physical source exists
    if !snapshot.capture_sources.is_empty() {
        let capture_channels = snapshot.capture_sources.len().min(u16::MAX as usize) as u16;
        descriptors.push(capture_descriptor(&snapshot, capture_channels));
    }

    // emit one duplex descriptor row when both lanes exist
    if !snapshot.playback_sinks.is_empty() && !snapshot.capture_sources.is_empty() {
        let duplex_channels = snapshot
            .playback_sinks
            .len()
            .min(snapshot.capture_sources.len())
            .min(u16::MAX as usize) as u16;

        descriptors.push(duplex_descriptor(&snapshot, duplex_channels));
    }

    if descriptors.is_empty() {
        return Err(core_platform::io_not_found(
            "destack.audio.device.list",
            "JACK reported no physical audio endpoints",
        ));
    }

    Ok(descriptors)
}

/// Build one baseline JACK capability mask.
fn jack_capability_flags(is_duplex: bool) -> audio_types::AudioDeviceCapabilityFlags {
    let mut flags = audio_core::DEVICE_CAPABILITY_SHARED_MODE.0
        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
        | audio_core::DEVICE_CAPABILITY_REROUTE_EVENTS.0;

    if is_duplex {
        flags |= audio_core::DEVICE_CAPABILITY_FULL_DUPLEX.0;
    }

    audio_types::AudioDeviceCapabilityFlags(flags)
}

/// Build one JACK playback descriptor row.
fn playback_descriptor(
    snapshot: &JackEndpointSnapshot,
    channels: u16,
) -> audio_core::HostDeviceDescriptor {
    let preferred_layout = channel_layout(channels.max(1));
    let preferred_mask = channel_mask(channels.max(1));

    audio_core::HostDeviceDescriptor {
        id: playback_stable_id("system"),
        group_id: String::from("jack-group:system"),
        name: String::from("JACK Playback"),
        transport: String::from("jack"),
        backend: audio_types::AudioBackend::Jack,
        direction: audio_types::AudioDeviceDirection::Playback,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK,
        connected: true,
        is_raw: false,
        is_default_playback: true,
        is_default_capture: false,
        is_default_loopback: false,
        capability_flags: jack_capability_flags(false),
        preferred_sample_rate: snapshot.sample_rate,
        min_sample_rate: snapshot.sample_rate,
        max_sample_rate: snapshot.sample_rate,
        preferred_period_frames: snapshot.period_frames,
        min_channels: 1,
        max_channels: channels.max(1),
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: channel_mask(channels.max(1)),
        min_period_frames: snapshot.period_frames,
        max_period_frames: snapshot.period_frames,
        format_mask: sample_format_bit(audio_types::AudioSampleFormat::F32),
        share_mode_mask: audio_core::SHARE_MODE_SHARED_BIT,
        is_null: false,
    }
}

/// Build one JACK capture descriptor row.
fn capture_descriptor(
    snapshot: &JackEndpointSnapshot,
    channels: u16,
) -> audio_core::HostDeviceDescriptor {
    let preferred_layout = channel_layout(channels.max(1));
    let preferred_mask = channel_mask(channels.max(1));

    audio_core::HostDeviceDescriptor {
        id: capture_stable_id("system"),
        group_id: String::from("jack-group:system"),
        name: String::from("JACK Capture"),
        transport: String::from("jack"),
        backend: audio_types::AudioBackend::Jack,
        direction: audio_types::AudioDeviceDirection::Capture,
        supported_directions: audio_core::DIRECTION_MASK_CAPTURE,
        connected: true,
        is_raw: false,
        is_default_playback: false,
        is_default_capture: true,
        is_default_loopback: false,
        capability_flags: jack_capability_flags(false),
        preferred_sample_rate: snapshot.sample_rate,
        min_sample_rate: snapshot.sample_rate,
        max_sample_rate: snapshot.sample_rate,
        preferred_period_frames: snapshot.period_frames,
        min_channels: 1,
        max_channels: channels.max(1),
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: channel_mask(channels.max(1)),
        min_period_frames: snapshot.period_frames,
        max_period_frames: snapshot.period_frames,
        format_mask: sample_format_bit(audio_types::AudioSampleFormat::F32),
        share_mode_mask: audio_core::SHARE_MODE_SHARED_BIT,
        is_null: false,
    }
}

/// Build one JACK duplex descriptor row.
fn duplex_descriptor(
    snapshot: &JackEndpointSnapshot,
    channels: u16,
) -> audio_core::HostDeviceDescriptor {
    let preferred_layout = channel_layout(channels.max(1));
    let preferred_mask = channel_mask(channels.max(1));

    audio_core::HostDeviceDescriptor {
        id: duplex_stable_id("system", "system"),
        group_id: String::from("jack-group:system"),
        name: String::from("JACK Duplex"),
        transport: String::from("jack"),
        backend: audio_types::AudioBackend::Jack,
        direction: audio_types::AudioDeviceDirection::Duplex,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK
            | audio_core::DIRECTION_MASK_CAPTURE
            | audio_core::DIRECTION_MASK_DUPLEX,
        connected: true,
        is_raw: false,
        is_default_playback: true,
        is_default_capture: true,
        is_default_loopback: false,
        capability_flags: jack_capability_flags(true),
        preferred_sample_rate: snapshot.sample_rate,
        min_sample_rate: snapshot.sample_rate,
        max_sample_rate: snapshot.sample_rate,
        preferred_period_frames: snapshot.period_frames,
        min_channels: 1,
        max_channels: channels.max(1),
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: channel_mask(channels.max(1)),
        min_period_frames: snapshot.period_frames,
        max_period_frames: snapshot.period_frames,
        format_mask: sample_format_bit(audio_types::AudioSampleFormat::F32),
        share_mode_mask: audio_core::SHARE_MODE_SHARED_BIT,
        is_null: false,
    }
}
