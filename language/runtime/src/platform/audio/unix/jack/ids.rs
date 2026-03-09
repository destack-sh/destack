use super::constants::{
    JACK_CAPTURE_STABLE_ID_PREFIX, JACK_DUPLEX_STABLE_ID_PREFIX, JACK_PLAYBACK_STABLE_ID_PREFIX,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, audio as audio_types};

/// One parsed JACK stable-id payload.
#[derive(Debug, Clone)]
pub(super) struct ParsedJackStableId {
    /// Requested direction lane encoded by the stable-id prefix.
    pub(super) lane: audio_types::AudioDeviceDirection,
    /// Playback endpoint token.
    pub(super) playback_name: String,
    /// Capture endpoint token for duplex rows.
    pub(super) capture_name: Option<String>,
}

/// Build one JACK playback stable id.
pub(super) fn playback_stable_id(device_name: &str) -> String {
    format!("{JACK_PLAYBACK_STABLE_ID_PREFIX}{device_name}")
}

/// Build one JACK capture stable id.
pub(super) fn capture_stable_id(device_name: &str) -> String {
    format!("{JACK_CAPTURE_STABLE_ID_PREFIX}{device_name}")
}

/// Build one JACK duplex stable id.
pub(super) fn duplex_stable_id(playback_name: &str, capture_name: &str) -> String {
    format!("{JACK_DUPLEX_STABLE_ID_PREFIX}{playback_name}|{capture_name}")
}

/// Parse one JACK stable id.
pub(super) fn parse_jack_stable_id(stable_id: &str) -> RuntimeResult<ParsedJackStableId> {
    if let Some(device_name) = stable_id.strip_prefix(JACK_PLAYBACK_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "jack playback id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedJackStableId {
            lane: audio_types::AudioDeviceDirection::Playback,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(device_name) = stable_id.strip_prefix(JACK_CAPTURE_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "jack capture id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedJackStableId {
            lane: audio_types::AudioDeviceDirection::Capture,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(payload) = stable_id.strip_prefix(JACK_DUPLEX_STABLE_ID_PREFIX) {
        let Some((playback_name, capture_name)) = payload.split_once('|') else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "jack duplex id must include playback and capture endpoint names",
            ))
            .boxed());
        };

        if playback_name.is_empty() || capture_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "jack duplex id must include non-empty playback and capture names",
            ))
            .boxed());
        }

        return Ok(ParsedJackStableId {
            lane: audio_types::AudioDeviceDirection::Duplex,
            playback_name: playback_name.to_string(),
            capture_name: Some(capture_name.to_string()),
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "jack id must start with jack:playback:, jack:capture:, or jack:duplex:",
    ))
    .boxed())
}

/// Validate one JACK stable-id lane against one requested stream direction.
pub(super) fn validate_jack_stable_id_direction(
    parsed: &ParsedJackStableId,
    direction: audio_types::AudioDeviceDirection,
) -> RuntimeResult<()> {
    if parsed.lane != direction {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "jack stable-id lane does not match requested stream direction",
        ))
        .boxed());
    }

    Ok(())
}
