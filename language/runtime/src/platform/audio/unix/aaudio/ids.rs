use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

use super::constants::{
    AAUDIO_CAPTURE_STABLE_ID_PREFIX, AAUDIO_DUPLEX_STABLE_ID_PREFIX,
    AAUDIO_PLAYBACK_STABLE_ID_PREFIX,
};
use crate::platform::audio as audio_types;

/// One parsed AAudio stable-id payload.
#[derive(Debug, Clone)]
pub(super) struct ParsedAaudioStableId {
    /// Requested direction lane encoded by the stable-id prefix.
    pub(super) lane: audio_types::AudioDeviceDirection,
    /// Playback endpoint token.
    pub(super) playback_name: String,
    /// Capture endpoint token for duplex rows.
    pub(super) capture_name: Option<String>,
}

/// Build one AAudio playback stable id.
pub(super) fn playback_stable_id(device_name: &str) -> String {
    format!("{AAUDIO_PLAYBACK_STABLE_ID_PREFIX}{device_name}")
}

/// Build one AAudio capture stable id.
pub(super) fn capture_stable_id(device_name: &str) -> String {
    format!("{AAUDIO_CAPTURE_STABLE_ID_PREFIX}{device_name}")
}

/// Build one AAudio duplex stable id.
pub(super) fn duplex_stable_id(playback_name: &str, capture_name: &str) -> String {
    format!("{AAUDIO_DUPLEX_STABLE_ID_PREFIX}{playback_name}|{capture_name}")
}

/// Parse one AAudio stable id.
pub(super) fn parse_aaudio_stable_id(stable_id: &str) -> RuntimeResult<ParsedAaudioStableId> {
    if let Some(device_name) = stable_id.strip_prefix(AAUDIO_PLAYBACK_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "aaudio playback id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedAaudioStableId {
            lane: audio_types::AudioDeviceDirection::Playback,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(device_name) = stable_id.strip_prefix(AAUDIO_CAPTURE_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "aaudio capture id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedAaudioStableId {
            lane: audio_types::AudioDeviceDirection::Capture,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(payload) = stable_id.strip_prefix(AAUDIO_DUPLEX_STABLE_ID_PREFIX) {
        let Some((playback_name, capture_name)) = payload.split_once('|') else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "aaudio duplex id must include playback and capture endpoint names",
            ))
            .boxed());
        };

        if playback_name.is_empty() || capture_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "aaudio duplex id must include non-empty playback and capture names",
            ))
            .boxed());
        }

        return Ok(ParsedAaudioStableId {
            lane: audio_types::AudioDeviceDirection::Duplex,
            playback_name: playback_name.to_string(),
            capture_name: Some(capture_name.to_string()),
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "aaudio id must start with aaudio:playback:, aaudio:capture:, or aaudio:duplex:",
    ))
    .boxed())
}

/// Validate one AAudio stable-id lane against one requested stream direction.
pub(super) fn validate_aaudio_stable_id_direction(
    parsed: &ParsedAaudioStableId,
    direction: audio_types::AudioDeviceDirection,
) -> RuntimeResult<()> {
    if parsed.lane != direction {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "aaudio stable-id lane does not match requested stream direction",
        ))
        .boxed());
    }

    Ok(())
}
