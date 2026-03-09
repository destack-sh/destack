use super::{TraceSegmentHeader, TraceSequence};
use destack_base::{FNV_OFFSET_BASIS_64, fnv1a_64_update};
use serde::{Deserialize, Serialize};

/// Trace segment payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceSegment {
    /// Segment header metadata.
    pub(super) header: TraceSegmentHeader,
    /// Segment payload bytes.
    pub(super) data: Vec<u8>,
    /// Recorded event lengths in append order.
    pub(super) event_lengths: Vec<u32>,
}

impl TraceSegment {
    /// Create one empty segment with initialized metadata.
    pub(super) fn new(index: u32, sequence_start: TraceSequence) -> Self {
        Self {
            header: TraceSegmentHeader {
                index,
                sequence_start,
                sequence_end: sequence_start,
                event_count: 0,
                byte_length: 0,
                checksum: FNV_OFFSET_BASIS_64,
            },
            data: Vec::new(),
            event_lengths: Vec::new(),
        }
    }

    /// Return whether this segment should rotate before appending one event.
    pub(super) fn should_rotate_for_event(
        &self,
        encoded_len: u64,
        max_events_per_chunk: usize,
        max_chunk_bytes: u64,
    ) -> bool {
        // rotate when event count would exceed the segment limit
        if self.event_lengths.len() >= max_events_per_chunk {
            return true;
        }

        // rotate when byte length would exceed the segment limit
        self.header.byte_length.saturating_add(encoded_len) > max_chunk_bytes
    }

    /// Update this segment checksum with one encoded payload range.
    pub(super) fn update_checksum_for_range(&mut self, start: usize, end: usize) {
        let bytes = &self.data[start..end];
        self.header.checksum = fnv1a_64_update(self.header.checksum, bytes);
    }

    /// Compute this segment payload checksum from scratch.
    pub(super) fn payload_checksum(&self) -> u64 {
        fnv1a_64_update(FNV_OFFSET_BASIS_64, &self.data)
    }
}
