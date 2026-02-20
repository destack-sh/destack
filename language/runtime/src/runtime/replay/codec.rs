use postcard::Error;
use postcard::experimental::serialized_size;

use super::ReplayEvent;

/// Encode a replay event to bytes.
pub fn encode_event(event: &ReplayEvent) -> Result<Vec<u8>, Error> {
    let size = serialized_size(event)?;
    let mut buffer = vec![0u8; size];
    postcard::to_slice(event, &mut buffer)?;
    Ok(buffer)
}

/// Decode a replay event from bytes.
pub fn decode_event(bytes: &[u8]) -> Result<ReplayEvent, Error> {
    postcard::from_bytes(bytes)
}
