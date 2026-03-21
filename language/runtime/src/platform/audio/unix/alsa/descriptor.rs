use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::core::{
    AlsaDeviceProfile, AlsaHintRow, channel_layout, channel_mask, enumerate_hint_rows,
    probe_row_profiles, transport_from_device_name,
};
use super::ids::{capture_stable_id, duplex_stable_id, playback_stable_id};
use crate::platform::audio as audio_types;

/// Enumerate ALSA devices and normalize them into runtime descriptors.
pub(super) fn enumerate_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    let rows = enumerate_hint_rows()?;
    let mut descriptors = Vec::new();

    let mut default_playback_assigned = false;
    let mut default_capture_assigned = false;
    let mut default_duplex_assigned = false;

    // probe each hinted row and expand it into one or more descriptors
    for row in rows {
        let (playback_profile, capture_profile) = probe_row_profiles(&row);
        if playback_profile.is_none() && capture_profile.is_none() {
            continue;
        }

        if let Some(profile) = playback_profile.as_ref() {
            let is_default_playback = !default_playback_assigned;
            if is_default_playback {
                default_playback_assigned = true;
            }

            descriptors.push(playback_descriptor_from_row(
                &row,
                profile,
                is_default_playback,
            ));
        }

        if let Some(profile) = capture_profile.as_ref() {
            let is_default_capture = !default_capture_assigned;
            if is_default_capture {
                default_capture_assigned = true;
            }

            descriptors.push(capture_descriptor_from_row(
                &row,
                profile,
                is_default_capture,
            ));
        }

        if let (Some(playback_profile), Some(capture_profile)) =
            (playback_profile.as_ref(), capture_profile.as_ref())
            && let Some(descriptor) = duplex_descriptor_from_row(
                &row,
                playback_profile,
                capture_profile,
                !default_duplex_assigned,
            )
        {
            default_duplex_assigned = true;
            descriptors.push(descriptor);
        }
    }

    Ok(descriptors)
}

/// Build one capability flag mask for one ALSA profile.
fn capability_flags(
    profile: &AlsaDeviceProfile,
    supports_full_duplex: bool,
) -> audio_types::AudioDeviceCapabilityFlags {
    let mut flags = audio_core::DEVICE_CAPABILITY_SHARED_MODE.0
        | audio_core::DEVICE_CAPABILITY_STREAM_VOLUME.0
        | audio_core::DEVICE_CAPABILITY_STREAM_MUTE.0
        | audio_core::DEVICE_CAPABILITY_REROUTE_EVENTS.0
        | audio_core::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0;

    if profile.supports_exclusive {
        flags |= audio_core::DEVICE_CAPABILITY_EXCLUSIVE_MODE.0;
    }

    if supports_full_duplex {
        flags |= audio_core::DEVICE_CAPABILITY_FULL_DUPLEX.0;
    }

    audio_types::AudioDeviceCapabilityFlags(flags)
}

/// Build one share-mode mask for one ALSA profile.
fn share_mode_mask(profile: &AlsaDeviceProfile) -> u32 {
    let mut mask = audio_core::SHARE_MODE_SHARED_BIT;
    if profile.supports_exclusive {
        mask |= audio_core::SHARE_MODE_EXCLUSIVE_BIT;
    }

    mask
}

/// Build one playback descriptor for one ALSA row.
fn playback_descriptor_from_row(
    row: &AlsaHintRow,
    profile: &AlsaDeviceProfile,
    is_default_playback: bool,
) -> audio_core::HostDeviceDescriptor {
    let preferred_layout = channel_layout(profile.preferred_channels);
    let preferred_mask = channel_mask(profile.preferred_channels);
    let supported_mask = channel_mask(profile.max_channels);

    audio_core::HostDeviceDescriptor {
        id: playback_stable_id(&row.name),
        group_id: format!("alsa-group:{}", row.name),
        name: row.description.clone(),
        transport: transport_from_device_name(&row.name).to_string(),
        backend: audio_types::AudioBackend::Alsa,
        direction: audio_types::AudioDeviceDirection::Playback,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK,
        connected: true,
        is_raw: row.name.starts_with("hw:"),
        is_default_playback,
        is_default_capture: false,
        is_default_loopback: false,
        capability_flags: capability_flags(profile, false),
        preferred_sample_rate: profile.preferred_sample_rate,
        min_sample_rate: profile.min_sample_rate,
        max_sample_rate: profile.max_sample_rate,
        preferred_period_frames: profile.preferred_period_frames,
        min_channels: profile.min_channels,
        max_channels: profile.max_channels,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: profile.min_period_frames,
        max_period_frames: profile.max_period_frames,
        format_mask: profile.format_mask,
        share_mode_mask: share_mode_mask(profile),
        is_null: false,
    }
}

/// Build one capture descriptor for one ALSA row.
fn capture_descriptor_from_row(
    row: &AlsaHintRow,
    profile: &AlsaDeviceProfile,
    is_default_capture: bool,
) -> audio_core::HostDeviceDescriptor {
    let preferred_layout = channel_layout(profile.preferred_channels);
    let preferred_mask = channel_mask(profile.preferred_channels);
    let supported_mask = channel_mask(profile.max_channels);

    audio_core::HostDeviceDescriptor {
        id: capture_stable_id(&row.name),
        group_id: format!("alsa-group:{}", row.name),
        name: row.description.clone(),
        transport: transport_from_device_name(&row.name).to_string(),
        backend: audio_types::AudioBackend::Alsa,
        direction: audio_types::AudioDeviceDirection::Capture,
        supported_directions: audio_core::DIRECTION_MASK_CAPTURE,
        connected: true,
        is_raw: row.name.starts_with("hw:"),
        is_default_playback: false,
        is_default_capture,
        is_default_loopback: false,
        capability_flags: capability_flags(profile, false),
        preferred_sample_rate: profile.preferred_sample_rate,
        min_sample_rate: profile.min_sample_rate,
        max_sample_rate: profile.max_sample_rate,
        preferred_period_frames: profile.preferred_period_frames,
        min_channels: profile.min_channels,
        max_channels: profile.max_channels,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames: profile.min_period_frames,
        max_period_frames: profile.max_period_frames,
        format_mask: profile.format_mask,
        share_mode_mask: share_mode_mask(profile),
        is_null: false,
    }
}

/// Build one duplex descriptor when playback and capture ranges intersect.
fn duplex_descriptor_from_row(
    row: &AlsaHintRow,
    playback_profile: &AlsaDeviceProfile,
    capture_profile: &AlsaDeviceProfile,
    is_default_duplex: bool,
) -> Option<audio_core::HostDeviceDescriptor> {
    let min_sample_rate = playback_profile
        .min_sample_rate
        .max(capture_profile.min_sample_rate);
    let max_sample_rate = playback_profile
        .max_sample_rate
        .min(capture_profile.max_sample_rate);
    if max_sample_rate < min_sample_rate {
        return None;
    }

    let min_channels = playback_profile
        .min_channels
        .max(capture_profile.min_channels);
    let max_channels = playback_profile
        .max_channels
        .min(capture_profile.max_channels);
    if max_channels < min_channels {
        return None;
    }

    let min_period_frames = playback_profile
        .min_period_frames
        .max(capture_profile.min_period_frames);
    let max_period_frames = playback_profile
        .max_period_frames
        .min(capture_profile.max_period_frames);
    if max_period_frames < min_period_frames {
        return None;
    }

    let preferred_sample_rate = playback_profile
        .preferred_sample_rate
        .clamp(min_sample_rate, max_sample_rate);
    let preferred_channels = if (2u16 >= min_channels) && (2u16 <= max_channels) {
        2
    } else {
        min_channels
    };
    let preferred_period_frames = playback_profile
        .preferred_period_frames
        .clamp(min_period_frames, max_period_frames);

    let preferred_layout = channel_layout(preferred_channels);
    let preferred_mask = channel_mask(preferred_channels);
    let supported_mask = channel_mask(max_channels);

    let mut format_mask = playback_profile.format_mask & capture_profile.format_mask;
    if format_mask == 0 {
        format_mask = playback_profile.format_mask | capture_profile.format_mask;
    }

    let supports_exclusive =
        playback_profile.supports_exclusive && capture_profile.supports_exclusive;
    let mut share_mask = audio_core::SHARE_MODE_SHARED_BIT;
    if supports_exclusive {
        share_mask |= audio_core::SHARE_MODE_EXCLUSIVE_BIT;
    }

    let profile = AlsaDeviceProfile {
        preferred_sample_rate,
        min_sample_rate,
        max_sample_rate,
        preferred_period_frames,
        min_period_frames,
        max_period_frames,
        preferred_channels,
        min_channels,
        max_channels,
        format_mask,
        supports_pause: playback_profile.supports_pause && capture_profile.supports_pause,
        supports_exclusive,
    };

    Some(audio_core::HostDeviceDescriptor {
        id: duplex_stable_id(&row.name, &row.name),
        group_id: format!("alsa-group:{}", row.name),
        name: format!("{} (duplex)", row.description),
        transport: transport_from_device_name(&row.name).to_string(),
        backend: audio_types::AudioBackend::Alsa,
        direction: audio_types::AudioDeviceDirection::Duplex,
        supported_directions: audio_core::DIRECTION_MASK_PLAYBACK
            | audio_core::DIRECTION_MASK_CAPTURE
            | audio_core::DIRECTION_MASK_DUPLEX,
        connected: true,
        is_raw: row.name.starts_with("hw:"),
        is_default_playback: is_default_duplex,
        is_default_capture: is_default_duplex,
        is_default_loopback: false,
        capability_flags: capability_flags(&profile, true),
        preferred_sample_rate,
        min_sample_rate,
        max_sample_rate,
        preferred_period_frames,
        min_channels,
        max_channels,
        preferred_layout,
        preferred_channel_mask: preferred_mask,
        supported_channel_mask: supported_mask,
        min_period_frames,
        max_period_frames,
        format_mask,
        share_mode_mask: share_mask,
        is_null: false,
    })
}
