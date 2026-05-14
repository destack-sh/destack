use std::sync::Arc;

use super::{TraceChunkHeader, TraceSequence};
use destack_core::{FNV_OFFSET_BASIS_64, fnv1a_64_update};
use serde::{Deserialize, Serialize};

/// Byte length of one trace event length prefix.
pub(super) const TRACE_EVENT_LENGTH_BYTES: usize = std::mem::size_of::<u32>();

/// Trace chunk payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceChunk {
    /// Chunk header metadata.
    pub(super) header: TraceChunkHeader,
    /// Chunk payload bytes.
    pub(super) bytes: Vec<u8>,
}

impl TraceChunk {
    /// Create one empty chunk with initialized metadata.
    pub(super) fn new(index: u32, sequence_start: TraceSequence) -> Self {
        Self {
            header: TraceChunkHeader {
                index,
                sequence_start,
                sequence_end: sequence_start,
                event_count: 0,
                byte_length: 0,
                checksum: FNV_OFFSET_BASIS_64,
            },
            bytes: Vec::new(),
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
        if self.header.event_count as usize >= max_events_per_chunk {
            return true;
        }

        // rotate when byte length would exceed the chunk limit
        self.header.byte_length.saturating_add(encoded_len) > max_chunk_bytes
    }

    /// Return whether this chunk currently stores any events.
    pub(super) fn is_empty(&self) -> bool {
        self.header.event_count == 0
    }

    /// Update this chunk checksum with one encoded payload range.
    pub(super) fn update_checksum_for_range(&mut self, start: usize, end: usize) {
        let bytes = &self.bytes[start..end];
        self.header.checksum = fnv1a_64_update(self.header.checksum, bytes);
    }

    /// Compute this chunk payload checksum from scratch.
    pub(super) fn payload_checksum(&self) -> u64 {
        fnv1a_64_update(FNV_OFFSET_BASIS_64, &self.bytes)
    }
}

/// One immutable shared trace prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TracePrefix {
    /// Older trace prefix.
    pub(super) parent: Option<Arc<TracePrefix>>,
    /// Chunks stored in this prefix.
    pub(super) chunks: Box<[TraceChunk]>,
    /// Total number of chunks reachable through this prefix.
    pub(super) chunk_count: u32,
    /// Total byte length reachable through this prefix.
    pub(super) byte_count: u64,
}

impl TracePrefix {
    /// Create one prefix from parent history and frozen chunks.
    pub(super) fn new(parent: Option<Arc<TracePrefix>>, chunks: Vec<TraceChunk>) -> Self {
        let parent_chunk_count = parent
            .as_ref()
            .map(|prefix| prefix.chunk_count)
            .unwrap_or(0);
        let parent_byte_count = parent.as_ref().map(|prefix| prefix.byte_count).unwrap_or(0);
        let local_chunk_count = chunks.len() as u32;
        let local_byte_count: u64 = chunks.iter().map(|chunk| chunk.header.byte_length).sum();

        Self {
            parent,
            chunks: chunks.into_boxed_slice(),
            chunk_count: parent_chunk_count + local_chunk_count,
            byte_count: parent_byte_count + local_byte_count,
        }
    }
}
