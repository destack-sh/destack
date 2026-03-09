use super::TraceEvent;
use postcard::Error;

/// Decode a trace event from bytes.
pub(crate) fn decode_event(bytes: &[u8]) -> Result<TraceEvent, Error> {
    postcard::from_bytes(bytes)
}
