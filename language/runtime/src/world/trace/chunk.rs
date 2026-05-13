use std::sync::Arc;

use super::{TraceChunkHeader, TraceSequence};
use destack_core::{FNV_OFFSET_BASIS_64, fnv1a_64_update};
use serde::{Deserialize, Serialize};

/// Trace chunk payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceChunk {
    /// Chunk header metadata.
    pub(super) header: TraceChunkHeader,
    /// Chunk payload bytes.
    pub(super) data: Vec<u8>,
    /// Recorded event lengths in append order.
    pub(super) event_lengths: Vec<u32>,
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

    /// Return whether this chunk currently stores any events.
    pub(super) fn is_empty(&self) -> bool {
        self.event_lengths.is_empty()
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

/// One immutable shared chain node of trace chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceChunkChain {
    /// Older chunk chain node.
    pub(super) parent: Option<Arc<TraceChunkChain>>,
    /// Chunks stored in this chain node.
    pub(super) chunks: Box<[TraceChunk]>,
    /// Total number of chunks reachable through this chain.
    pub(super) chunk_count: u32,
    /// Total byte length reachable through this chain.
    pub(super) byte_count: u64,
}

#[allow(dead_code)]
impl TraceChunkChain {
    /// Create one chain node from parent history and frozen chunks.
    pub(super) fn new(parent: Option<Arc<TraceChunkChain>>, chunks: Vec<TraceChunk>) -> Self {
        let parent_chunk_count = parent.as_ref().map(|chain| chain.chunk_count).unwrap_or(0);
        let parent_byte_count = parent.as_ref().map(|chain| chain.byte_count).unwrap_or(0);
        let local_chunk_count = chunks.len() as u32;
        let local_byte_count: u64 = chunks.iter().map(|chunk| chunk.header.byte_length).sum();

        Self {
            parent,
            chunks: chunks.into_boxed_slice(),
            chunk_count: parent_chunk_count + local_chunk_count,
            byte_count: parent_byte_count + local_byte_count,
        }
    }

    /// Report whether this chain contains the target head.
    pub(crate) fn contains(chain: &Arc<Self>, target: &Arc<Self>) -> bool {
        let mut current = Some(chain);
        while let Some(chain) = current {
            if Arc::ptr_eq(chain, target) {
                return true;
            }
            current = chain.parent.as_ref();
        }
        false
    }
}
