use postcard::Error;

use super::ReplayEvent;

/// Encode a replay event to bytes.
pub fn encode_event(event: &ReplayEvent) -> Result<Vec<u8>, Error> {
    postcard::to_allocvec(event)
}

/// Decode a replay event from bytes.
pub fn decode_event(bytes: &[u8]) -> Result<ReplayEvent, Error> {
    postcard::from_bytes(bytes)
}
