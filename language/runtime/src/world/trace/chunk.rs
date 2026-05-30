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
    pub(super) fn new(sequence_start: TraceSequence) -> Self {
        Self {
            header: TraceChunkHeader {
                sequence_start,
                event_count: 0,
            },
            bytes: Vec::new(),
        }
    }

    /// Return whether this chunk should rotate before appending one event.
    pub(super) fn should_rotate_for_event(
        &self,
        encoded_len: u64,
        max_events_per_chunk: usize,
        max_chunk_size_bytes: u64,
    ) -> bool {
        // rotate when event count would exceed the chunk limit
        if self.header.event_count as usize >= max_events_per_chunk {
            return true;
        }

        // rotate when byte length would exceed the chunk limit
        self.byte_length().saturating_add(encoded_len) > max_chunk_size_bytes
    }

    /// Return whether this chunk currently stores any events.
    pub(super) fn is_empty(&self) -> bool {
        self.header.event_count == 0
    }

    /// Compute this chunk payload checksum from scratch.
    pub(super) fn payload_checksum(&self) -> u64 {
        fnv1a_64_update(FNV_OFFSET_BASIS_64, &self.bytes)
    }

    /// Return this chunk payload length in bytes.
    pub(super) fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Return the last sequence number stored in this chunk.
    pub(super) fn sequence_end(&self) -> TraceSequence {
        debug_assert!(!self.is_empty());
        let event_offset = (self.header.event_count - 1) as u64;

        TraceSequence::new(self.header.sequence_start.get() + event_offset)
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
}

impl TracePrefix {
    /// Create one prefix from parent lineage and frozen chunks.
    pub(super) fn new(parent: Option<Arc<TracePrefix>>, chunks: Vec<TraceChunk>) -> Self {
        let parent_chunk_count = parent
            .as_ref()
            .map(|prefix| prefix.chunk_count)
            .unwrap_or(0);
        let local_chunk_count = chunks.len() as u32;

        Self {
            parent,
            chunks: chunks.into_boxed_slice(),
            chunk_count: parent_chunk_count + local_chunk_count,
        }
    }
}
