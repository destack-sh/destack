use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;

use super::constants::{
    OPENSLES_CAPTURE_STABLE_ID_PREFIX, OPENSLES_DUPLEX_STABLE_ID_PREFIX,
    OPENSLES_PLAYBACK_STABLE_ID_PREFIX,
};

/// One parsed OpenSL ES stable-id payload.
#[derive(Debug, Clone)]
pub(super) struct ParsedOpenslesStableId {
    /// Requested direction lane encoded by the stable-id prefix.
    pub(super) lane: audio_core::AudioDeviceDirection,
    /// Playback endpoint token.
    pub(super) playback_name: String,
    /// Capture endpoint token for duplex rows.
    pub(super) capture_name: Option<String>,
}

/// Build one OpenSL ES playback stable id.
pub(super) fn playback_stable_id(device_name: &str) -> String {
    format!("{OPENSLES_PLAYBACK_STABLE_ID_PREFIX}{device_name}")
}

/// Build one OpenSL ES capture stable id.
pub(super) fn capture_stable_id(device_name: &str) -> String {
    format!("{OPENSLES_CAPTURE_STABLE_ID_PREFIX}{device_name}")
}

/// Build one OpenSL ES duplex stable id.
pub(super) fn duplex_stable_id(playback_name: &str, capture_name: &str) -> String {
    format!("{OPENSLES_DUPLEX_STABLE_ID_PREFIX}{playback_name}|{capture_name}")
}

/// Parse one OpenSL ES stable id.
pub(super) fn parse_opensles_stable_id(stable_id: &str) -> RuntimeResult<ParsedOpenslesStableId> {
    if let Some(device_name) = stable_id.strip_prefix(OPENSLES_PLAYBACK_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "opensles playback id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedOpenslesStableId {
            lane: audio_core::AudioDeviceDirection::Playback,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(device_name) = stable_id.strip_prefix(OPENSLES_CAPTURE_STABLE_ID_PREFIX) {
        if device_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "opensles capture id must include one non-empty endpoint name",
            ))
            .boxed());
        }

        return Ok(ParsedOpenslesStableId {
            lane: audio_core::AudioDeviceDirection::Capture,
            playback_name: device_name.to_string(),
            capture_name: None,
        });
    }

    if let Some(payload) = stable_id.strip_prefix(OPENSLES_DUPLEX_STABLE_ID_PREFIX) {
        let Some((playback_name, capture_name)) = payload.split_once('|') else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "opensles duplex id must include playback and capture endpoint names",
            ))
            .boxed());
        };

        if playback_name.is_empty() || capture_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "opensles duplex id must include non-empty playback and capture names",
            ))
            .boxed());
        }

        return Ok(ParsedOpenslesStableId {
            lane: audio_core::AudioDeviceDirection::Duplex,
            playback_name: playback_name.to_string(),
            capture_name: Some(capture_name.to_string()),
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "opensles id must start with opensles:playback:, opensles:capture:, or opensles:duplex:",
    ))
    .boxed())
}

/// Validate one OpenSL ES stable-id lane against one requested stream direction.
pub(super) fn validate_opensles_stable_id_direction(
    parsed: &ParsedOpenslesStableId,
    direction: audio_core::AudioDeviceDirection,
) -> RuntimeResult<()> {
    if parsed.lane != direction {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "id",
            "opensles stable-id lane does not match requested stream direction",
        ))
        .boxed());
    }

    Ok(())
}
