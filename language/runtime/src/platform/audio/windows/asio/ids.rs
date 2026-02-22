use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

use super::constants::{
    ASIO_CAPTURE_STABLE_ID_PREFIX, ASIO_DUPLEX_STABLE_ID_PREFIX, ASIO_PLAYBACK_STABLE_ID_PREFIX,
};
use super::core::{AsioDirectionLane, ParsedAsioStableId};

/// Build one ASIO playback stable id from one key name.
pub(super) fn playback_stable_id(key_name: &str) -> String {
    format!("{ASIO_PLAYBACK_STABLE_ID_PREFIX}{key_name}")
}

/// Build one ASIO capture stable id from one key name.
pub(super) fn capture_stable_id(key_name: &str) -> String {
    format!("{ASIO_CAPTURE_STABLE_ID_PREFIX}{key_name}")
}

/// Build one ASIO duplex stable id from one key name.
pub(super) fn duplex_stable_id(key_name: &str) -> String {
    format!("{ASIO_DUPLEX_STABLE_ID_PREFIX}{key_name}")
}

/// Parse one ASIO stable id into one typed payload.
pub(super) fn parse_stable_id(id: &str) -> RuntimeResult<ParsedAsioStableId> {
    // parse one playback lane id
    if let Some(key_name) = id.strip_prefix(ASIO_PLAYBACK_STABLE_ID_PREFIX) {
        if key_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "asio playback stable id is missing driver key",
            ))
            .boxed());
        }

        return Ok(ParsedAsioStableId {
            lane: AsioDirectionLane::Playback,
            key_name: key_name.to_string(),
        });
    }

    // parse one capture lane id
    if let Some(key_name) = id.strip_prefix(ASIO_CAPTURE_STABLE_ID_PREFIX) {
        if key_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "asio capture stable id is missing driver key",
            ))
            .boxed());
        }

        return Ok(ParsedAsioStableId {
            lane: AsioDirectionLane::Capture,
            key_name: key_name.to_string(),
        });
    }

    // parse one duplex lane id
    if let Some(key_name) = id.strip_prefix(ASIO_DUPLEX_STABLE_ID_PREFIX) {
        if key_name.is_empty() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "id",
                "asio duplex stable id is missing driver key",
            ))
            .boxed());
        }

        return Ok(ParsedAsioStableId {
            lane: AsioDirectionLane::Duplex,
            key_name: key_name.to_string(),
        });
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "id",
        "unrecognized ASIO stable id prefix",
    ))
    .boxed())
}
