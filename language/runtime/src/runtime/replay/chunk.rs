use super::{LogSequence, ReplayChunkHeader};
use destack_base::{FNV_OFFSET_BASIS_64, fnv1a_64_update};

/// Replay log chunk payload.
#[derive(Debug, Clone)]
pub(super) struct ReplayChunk {
    /// Chunk header metadata.
    pub(super) header: ReplayChunkHeader,
    /// Chunk payload bytes.
    pub(super) data: Vec<u8>,
    /// Recorded event lengths in append order.
    pub(super) event_lengths: Vec<u32>,
}

impl ReplayChunk {
    /// Create one empty chunk with initialized metadata.
    pub(super) fn new(index: u32, sequence_start: LogSequence) -> Self {
        Self {
            header: ReplayChunkHeader {
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

    /// Return whether this chunk should rotate before appending one event.
    pub(super) fn should_rotate_for_event(
        &self,
        encoded_len: u64,
        max_events_per_chunk: usize,
        max_chunk_bytes: u64,
    ) -> bool {
        // rotate when event count would exceed the chunk limit
        if self.event_lengths.len() >= max_events_per_chunk {
            return true;
        }

        // rotate when byte length would exceed the chunk limit
        self.header.byte_length.saturating_add(encoded_len) > max_chunk_bytes
    }

    /// Update this chunk checksum with one encoded payload range.
    pub(super) fn update_checksum_for_range(&mut self, start: usize, end: usize) {
        let bytes = &self.data[start..end];
        self.header.checksum = fnv1a_64_update(self.header.checksum, bytes);
    }

    /// Compute this chunk payload checksum from scratch.
    pub(super) fn payload_checksum(&self) -> u64 {
        fnv1a_64_update(FNV_OFFSET_BASIS_64, &self.data)
    }
}
