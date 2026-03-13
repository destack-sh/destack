use std::sync::Arc;

use super::{TraceSegmentHeader, TraceSequence};
use destack_core::{FNV_OFFSET_BASIS_64, fnv1a_64_update};
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

    /// Return whether this segment currently stores any events.
    pub(super) fn is_empty(&self) -> bool {
        self.event_lengths.is_empty()
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

/// One immutable shared block of trace segments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceBlock {
    /// The older block in the shared history chain.
    pub(super) parent: Option<Arc<TraceBlock>>,
    /// The segments stored in this block.
    pub(super) segments: Box<[TraceSegment]>,
    /// The total number of segments reachable through this block.
    pub(super) segment_count: u32,
    /// The total byte length reachable through this block.
    pub(super) byte_count: u64,
}

#[allow(dead_code)]
impl TraceBlock {
    /// Create one shared block from parent history and frozen segments.
    pub(super) fn new(parent: Option<Arc<TraceBlock>>, segments: Vec<TraceSegment>) -> Self {
        let parent_segment_count = parent
            .as_ref()
            .map(|block| block.segment_count)
            .unwrap_or(0);
        let parent_byte_count = parent.as_ref().map(|block| block.byte_count).unwrap_or(0);
        let local_segment_count = segments.len() as u32;
        let local_byte_count: u64 = segments
            .iter()
            .map(|segment| segment.header.byte_length)
            .sum();

        Self {
            parent,
            segments: segments.into_boxed_slice(),
            segment_count: parent_segment_count + local_segment_count,
            byte_count: parent_byte_count + local_byte_count,
        }
    }

    /// Report whether this block chain contains the target head.
    pub(crate) fn contains(block: &Arc<Self>, target: &Arc<Self>) -> bool {
        let mut current = Some(block);
        while let Some(block) = current {
            if Arc::ptr_eq(block, target) {
                return true;
            }
            current = block.parent.as_ref();
        }
        false
    }
}
