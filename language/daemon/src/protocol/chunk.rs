use super::{
    DaemonNotification, PayloadChunkNotification, PayloadFormat, PayloadId, ProtocolLimits,
    ProtocolMessage, ProtocolNotification,
};

/// Default maximum inline payload size in bytes.
pub const DEFAULT_INLINE_PAYLOAD_MAX_BYTES: usize = 8 * 1024 * 1024;
/// Bytes reserved for inline payload metadata.
pub const INLINE_PAYLOAD_OVERHEAD_BYTES: usize = 512;
/// Bytes reserved for payload chunk headroom.
pub const PAYLOAD_CHUNK_HEADROOM_BYTES: usize = 256;

/// Compute the max inline payload size for negotiated limits.
pub fn inline_payload_max_bytes(limits: ProtocolLimits) -> usize {
    // derive the max payload limit
    let max_payload_bytes = max_payload_bytes(limits);

    // reserve space for inline metadata
    let budget = max_payload_bytes.saturating_sub(INLINE_PAYLOAD_OVERHEAD_BYTES);

    // clamp against the default inline limit
    DEFAULT_INLINE_PAYLOAD_MAX_BYTES.min(budget)
}

/// Compute the payload chunk size for negotiated limits.
pub fn payload_chunk_bytes(limits: ProtocolLimits) -> usize {
    // derive the max payload limit
    let max_payload_bytes = max_payload_bytes(limits);

    // reserve space for chunk metadata
    let overhead = payload_chunk_overhead_bytes();
    max_payload_bytes.saturating_sub(overhead)
}

/// Return the negotiated payload limit as an in-process size.
fn max_payload_bytes(limits: ProtocolLimits) -> usize {
    // clamp the payload limit to usize
    usize::try_from(limits.max_payload_bytes).unwrap_or(usize::MAX)
}

/// Return the encoded overhead reserved for payload chunks.
fn payload_chunk_overhead_bytes() -> usize {
    // build a minimal payload chunk message
    let notification = PayloadChunkNotification {
        id: PayloadId::new(0),
        format: PayloadFormat::Postcard,
        index: 0,
        total: 1,
        bytes: Vec::new(),
        done: true,
    };
    let message = ProtocolMessage::Notification(Box::new(ProtocolNotification {
        payload: DaemonNotification::PayloadChunk(notification),
    }));

    // compute the encoded size with extra headroom
    let base = postcard::to_allocvec(&message).map_or(0, |bytes| bytes.len());
    base + PAYLOAD_CHUNK_HEADROOM_BYTES
}
