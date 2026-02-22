use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;

use super::constants::{
    PULSEAUDIO_CAPTURE_STABLE_ID_PREFIX, PULSEAUDIO_DUPLEX_STABLE_ID_PREFIX,
    PULSEAUDIO_LOOPBACK_STABLE_ID_PREFIX, PULSEAUDIO_PLAYBACK_STABLE_ID_PREFIX,
};

/// One parsed PulseAudio stable-id payload.
#[derive(Debug, Clone)]
pub(super) struct ParsedPulseaudioStableId {
    /// Requested direction lane encoded by the stable-id prefix.
    pub(super) lane: audio_core::AudioDeviceDirection,
    /// Playback endpoint token.
    pub(super) playback_name: String,
    /// Capture endpoint token for duplex rows.
    pub(super) capture_name: Option<String>,
}

/// Build one PulseAudio playback stable id.
pub(super) fn playback_stable_id(device_name: &str) -> String {
    format!("{PULSEAUDIO_PLAYBACK_STABLE_ID_PREFIX}{device_name}")
}

/// Build one PulseAudio capture stable id.
pub(super) fn capture_stable_id(device_name: &str) -> String {
    format!("{PULSEAUDIO_CAPTURE_STABLE_ID_PREFIX}{device_name}")
}

/// Build one PulseAudio duplex stable id.
pub(super) fn duplex_stable_id(playback_name: &str, capture_name: &str) -> String {
    format!("{PULSEAUDIO_DUPLEX_STABLE_ID_PREFIX}{playback_name}|{capture_name}")
}

/// Build one PulseAudio loopback stable id.
pub(super) fn loopback_stable_id(device_name: &str) -> String {
    format!("{PULSEAUDIO_LOOPBACK_STABLE_ID_PREFIX}{device_name}")
}

/// Parse one PulseAudio stable id.
pub(super) fn parse_pulseaudio_stable_id(
    stable_id: &str,
) -> RuntimeResult<ParsedPulseaudioStableId> {
    if let Some(device_name) = stable_id.strip_prefix(PULSEAUDIO_PLAYBACK_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "pulseaudio playback id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedPulseaudioStableId {
            lane: audio_core::AudioDeviceDirection::Playback,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(device_name) = stable_id.strip_prefix(PULSEAUDIO_CAPTURE_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "pulseaudio capture id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedPulseaudioStableId {
            lane: audio_core::AudioDeviceDirection::Capture,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(payload) = stable_id.strip_prefix(PULSEAUDIO_DUPLEX_STABLE_ID_PREFIX) {
        let Some((playback_name, capture_name)) = payload.split_once('|') else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "pulseaudio duplex id must include playback and capture endpoint names",
            ))
            .boxed());
        };

        if playback_name.is_empty() || capture_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "pulseaudio duplex id must include non-empty playback and capture names",
            ))
            .boxed());
        }

        return Ok(ParsedPulseaudioStableId {
            lane: audio_core::AudioDeviceDirection::Duplex,
            playback_name: playback_name.to_string(),
            capture_name: Some(capture_name.to_string()),
        });
    }

    if let Some(device_name) = stable_id.strip_prefix(PULSEAUDIO_LOOPBACK_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "pulseaudio loopback id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedPulseaudioStableId {
            lane: audio_core::AudioDeviceDirection::Loopback,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "pulseaudio id must start with pulseaudio:playback:, pulseaudio:capture:, pulseaudio:duplex:, or pulseaudio:loopback:",
    ))
    .boxed())
}

/// Validate one PulseAudio stable-id lane against one requested stream direction.
pub(super) fn validate_pulseaudio_stable_id_direction(
    parsed: &ParsedPulseaudioStableId,
    direction: audio_core::AudioDeviceDirection,
) -> RuntimeResult<()> {
    if parsed.lane != direction {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "pulseaudio stable-id lane does not match requested stream direction",
        ))
        .boxed());
    }

    Ok(())
}
