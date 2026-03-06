use super::ReplayEvent;
use postcard::Error;

/// Decode a replay event from bytes.
pub(crate) fn decode_event(bytes: &[u8]) -> Result<ReplayEvent, Error> {
    postcard::from_bytes(bytes)
}
