use super::TraceRecord;
use postcard::Error;

/// Decode one trace record from bytes.
pub(crate) fn decode_event(bytes: &[u8]) -> Result<TraceRecord, Error> {
    postcard::from_bytes(bytes)
}
