use super::constants::{
    ALSA_CAPTURE_STABLE_ID_PREFIX, ALSA_DUPLEX_STABLE_ID_PREFIX, ALSA_PLAYBACK_STABLE_ID_PREFIX,
};
use super::core::{AlsaDirectionLane, ParsedAlsaStableId};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, audio as audio_types};

/// Build one normalized ALSA playback stable id.
pub(super) fn playback_stable_id(device_name: &str) -> String {
    format!("{ALSA_PLAYBACK_STABLE_ID_PREFIX}{device_name}")
}

/// Build one normalized ALSA capture stable id.
pub(super) fn capture_stable_id(device_name: &str) -> String {
    format!("{ALSA_CAPTURE_STABLE_ID_PREFIX}{device_name}")
}

/// Build one normalized ALSA duplex stable id.
pub(super) fn duplex_stable_id(playback_name: &str, capture_name: &str) -> String {
    format!("{ALSA_DUPLEX_STABLE_ID_PREFIX}{playback_name}|{capture_name}")
}

/// Parse one normalized ALSA stable id.
pub(super) fn parse_stable_id(stable_id: &str) -> RuntimeResult<ParsedAlsaStableId> {
    if let Some(device_name) = stable_id.strip_prefix(ALSA_PLAYBACK_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "alsa playback id must include one non-empty pcm name",
            ))
            .boxed());
        }

        return Ok(ParsedAlsaStableId {
            lane: AlsaDirectionLane::Playback,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(device_name) = stable_id.strip_prefix(ALSA_CAPTURE_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "alsa capture id must include one non-empty pcm name",
            ))
            .boxed());
        }

        return Ok(ParsedAlsaStableId {
            lane: AlsaDirectionLane::Capture,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(payload) = stable_id.strip_prefix(ALSA_DUPLEX_STABLE_ID_PREFIX) {
        let Some((playback_name, capture_name)) = payload.split_once('|') else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "alsa duplex id must include playback and capture pcm names",
            ))
            .boxed());
        };

        if playback_name.is_empty() || capture_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "alsa duplex id must include non-empty playback and capture names",
            ))
            .boxed());
        }

        return Ok(ParsedAlsaStableId {
            lane: AlsaDirectionLane::Duplex,
            playback_name: playback_name.to_string(),
            capture_name: Some(capture_name.to_string()),
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "alsa id must start with alsa:playback:, alsa:capture:, or alsa:duplex:",
    ))
    .boxed())
}

/// Validate one parsed stable-id lane against one requested stream direction.
pub(super) fn validate_stable_id_direction(
    parsed: &ParsedAlsaStableId,
    direction: audio_types::AudioDeviceDirection,
) -> RuntimeResult<()> {
    match (parsed.lane, direction) {
        (AlsaDirectionLane::Playback, audio_types::AudioDeviceDirection::Playback)
        | (AlsaDirectionLane::Capture, audio_types::AudioDeviceDirection::Capture)
        | (AlsaDirectionLane::Duplex, audio_types::AudioDeviceDirection::Duplex) => Ok(()),
        _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "alsa stable-id lane does not match requested stream direction",
        ))
        .boxed()),
    }
}
