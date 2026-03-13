use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::trace::codec::decode_event;
use crate::runtime::trace::log::{TraceSequence, TraceState, compute_log_hash};
use crate::runtime::trace::{TraceRecord, TraceTrailer};
use crate::runtime::world::BranchId;

use super::chunk::{TraceBlock, TraceSegment};

/// One cached immutable block range for one cursor.
#[derive(Debug)]
struct TraceBlockRange {
    /// The first global segment index stored in this block.
    start_segment: usize,
    /// The shared immutable block.
    block: Arc<TraceBlock>,
}

/// Cached immutable block path for one trace cursor.
#[derive(Debug, Default)]
struct TraceBlockCache {
    /// The current shared head block this cache was built from.
    head: Option<Arc<TraceBlock>>,
    /// The shared history blocks in oldest-to-newest order.
    blocks: Vec<TraceBlockRange>,
}

/// Trace cursor state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct TraceReadCursor {
    /// Current segment index.
    read_segment: usize,
    /// Current event index inside the segment.
    read_index: usize,
    /// Current byte offset inside the segment payload.
    read_offset: usize,
    /// Next expected sequence number.
    next_sequence: TraceSequence,
    /// Last segment index that was validated.
    validated_segment: Option<usize>,
}

/// Durable cursor state for one trace cursor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct TraceCursorImage {
    /// The captured branch identifier.
    branch_id: BranchId,
    /// The next sequence available in the captured trace image.
    upper_bound: TraceSequence,
    /// The captured reader cursor.
    cursor: TraceReadCursor,
}

/// Trace cursor for a shared trace log.
#[derive(Debug)]
pub struct TraceCursor {
    /// Shared live trace-log state.
    state: Arc<Mutex<TraceState>>,
    /// Cursor state for this reader.
    cursor: Mutex<TraceReadCursor>,
    /// Cached shared block path for segment reads.
    block_cache: Mutex<TraceBlockCache>,
}

impl TraceCursor {
    /// Create a trace cursor for the given trace-log state.
    pub(super) fn new(state: Arc<Mutex<TraceState>>) -> Self {
        Self {
            state,
            cursor: Mutex::new(TraceReadCursor {
                read_segment: 0,
                read_index: 0,
                read_offset: 0,
                next_sequence: TraceSequence::new(0),
                validated_segment: None,
            }),
            block_cache: Mutex::new(TraceBlockCache::default()),
        }
    }

    /// Read the next recorded trace record if available.
    pub(crate) fn next_event(&self) -> RuntimeResult<Option<TraceRecord>> {
        // validate the live trailer hash before consuming one event
        let state = self.state.lock();
        let trailer = state.trailer();
        let log_hash = compute_log_hash(&trailer.segments, &trailer.checkpoints);
        if trailer.log_hash != 0 && trailer.log_hash != log_hash {
            return Err(RuntimeError::TraceMismatch {
                name: "log_hash".to_string(),
            }
            .boxed());
        }

        // refresh the shared block cache for this visible head
        let mut block_cache = self.block_cache.lock();
        self.sync_block_cache(&state, &mut block_cache);

        // read the next event by advancing across the visible segments
        let mut cursor = self.cursor.lock();

        while cursor.read_segment < state.segment_count() {
            let segment_index = cursor.read_segment;
            let segment = self
                .segment(&state, &block_cache, segment_index)
                .ok_or_else(|| {
                    RuntimeError::InconsistentImage {
                        detail: format!("trace segment {segment_index} is missing"),
                    }
                    .boxed()
                })?;

            // validate one segment the first time this cursor reads it
            if cursor.validated_segment != Some(segment_index) {
                validate_segment(segment, trailer)?;
                cursor.validated_segment = Some(segment_index);
            }

            // decode the next event from the current segment
            if cursor.read_index < segment.event_lengths.len() {
                let sequence = cursor.next_sequence;
                if cursor.read_index == 0 && segment.header.sequence_start != sequence {
                    return Err(RuntimeError::TraceMismatch {
                        name: "sequence".to_string(),
                    }
                    .boxed());
                }

                let event_length = segment.event_lengths[cursor.read_index] as usize;
                let start = cursor.read_offset;
                let end = start + event_length;
                let encoded = &segment.data[start..end];
                let event = decode_event(encoded).map_err(|_| {
                    RuntimeError::TraceDecodeFailed {
                        name: "event".to_string(),
                    }
                    .boxed()
                })?;
                if cursor.read_index + 1 == segment.event_lengths.len()
                    && segment.header.sequence_end != sequence
                {
                    return Err(RuntimeError::TraceMismatch {
                        name: "sequence".to_string(),
                    }
                    .boxed());
                }

                cursor.read_index += 1;
                cursor.read_offset = end;
                cursor.next_sequence = sequence.next();

                return Ok(Some(event));
            }

            // otherwise advance into the next segment
            cursor.read_segment += 1;
            cursor.read_index = 0;
            cursor.read_offset = 0;
        }

        Ok(None)
    }

    /// Capture the reader cursor state.
    pub(crate) fn capture_image(&self) -> TraceCursorImage {
        let cursor = *self.cursor.lock();
        let state = self.state.lock();

        TraceCursorImage {
            branch_id: state.branch_id(),
            upper_bound: state.next_sequence(),
            cursor,
        }
    }

    /// Restore the reader cursor state.
    pub(crate) fn restore_image(&self, image: TraceCursorImage) -> RuntimeResult<()> {
        let state = self.state.lock();
        if state.branch_id() != image.branch_id {
            return Err(RuntimeError::TraceMismatch {
                name: "branch".to_string(),
            }
            .boxed());
        }
        if image.cursor.next_sequence.get() > image.upper_bound.get() {
            return Err(RuntimeError::TraceMismatch {
                name: "sequence".to_string(),
            }
            .boxed());
        }
        if image.upper_bound != state.next_sequence() {
            return Err(RuntimeError::TraceMismatch {
                name: "upper_bound".to_string(),
            }
            .boxed());
        }

        let mut cursor = self.cursor.lock();
        *cursor = image.cursor;

        // drop the cached block path so the next read rebuilds against the restored head
        let mut block_cache = self.block_cache.lock();
        *block_cache = TraceBlockCache::default();

        Ok(())
    }

    /// Return the next sequence visible through this cursor.
    pub(crate) fn sequence(&self) -> TraceSequence {
        self.cursor.lock().next_sequence
    }

    /// Seek this cursor to one sequence boundary.
    pub(crate) fn seek_sequence(&self, sequence: TraceSequence) -> RuntimeResult<()> {
        // reject seeks beyond the visible log tail
        let state = self.state.lock();
        if sequence.get() > state.next_sequence().get() {
            return Err(RuntimeError::TraceMismatch {
                name: "sequence".to_string(),
            }
            .boxed());
        }

        // refresh the shared block cache before scanning from the start
        let mut block_cache = self.block_cache.lock();
        self.sync_block_cache(&state, &mut block_cache);

        let mut cursor = self.cursor.lock();
        *cursor = self.seek_segment(&state, &block_cache, sequence)?;

        Ok(())
    }

    /// Rebuild the cached block path when the shared head changes.
    fn sync_block_cache(&self, state: &TraceState, cache: &mut TraceBlockCache) {
        let Some(head) = state.head() else {
            cache.head = None;
            cache.blocks.clear();
            return;
        };

        if cache
            .head
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, head))
        {
            return;
        }

        let mut blocks = Vec::new();
        let mut current = Some(head.clone());

        // collect newest-to-oldest first
        while let Some(block) = current {
            current = block.parent.clone();
            blocks.push(block);
        }

        // reverse into oldest-to-newest order for sequential reads
        blocks.reverse();

        let mut next_start_segment = 0usize;
        let blocks = blocks
            .into_iter()
            .map(|block| {
                let range = TraceBlockRange {
                    start_segment: next_start_segment,
                    block: block.clone(),
                };
                next_start_segment += block.segments.len();
                range
            })
            .collect();

        cache.head = Some(head.clone());
        cache.blocks = blocks;
    }

    /// Return one visible segment by stable global index.
    fn segment<'a>(
        &self,
        state: &'a TraceState,
        cache: &'a TraceBlockCache,
        index: usize,
    ) -> Option<&'a TraceSegment> {
        // read shared immutable history first
        for block in &cache.blocks {
            let block_end = block.start_segment + block.block.segments.len();

            if index < block_end {
                return block.block.segments.get(index - block.start_segment);
            }
        }

        let segment_index = index.saturating_sub(state.head_segment_count());

        if segment_index < state.sealed_tail().len() {
            return state.sealed_tail().get(segment_index);
        }

        if segment_index == state.sealed_tail().len() {
            return state.active_segment();
        }

        None
    }

    /// Seek one cursor directly to the segment containing the requested sequence.
    fn seek_segment(
        &self,
        state: &TraceState,
        cache: &TraceBlockCache,
        sequence: TraceSequence,
    ) -> RuntimeResult<TraceReadCursor> {
        // shared immutable history
        for block in &cache.blocks {
            for (local_segment_index, segment) in block.block.segments.iter().enumerate() {
                if !sequence_in_segment(sequence, segment) {
                    continue;
                }

                let read_segment = block.start_segment + local_segment_index;
                let segment_offset = sequence_offset_in_segment(sequence, segment)?;

                return Ok(TraceReadCursor {
                    read_segment,
                    read_index: segment_offset.0,
                    read_offset: segment_offset.1,
                    next_sequence: sequence,
                    validated_segment: None,
                });
            }
        }

        let tail_start = state.head_segment_count();

        // local sealed tail
        for (local_segment_index, segment) in state.sealed_tail().iter().enumerate() {
            if !sequence_in_segment(sequence, segment) {
                continue;
            }

            let read_segment = tail_start + local_segment_index;
            let segment_offset = sequence_offset_in_segment(sequence, segment)?;

            return Ok(TraceReadCursor {
                read_segment,
                read_index: segment_offset.0,
                read_offset: segment_offset.1,
                next_sequence: sequence,
                validated_segment: None,
            });
        }

        // active tail
        if let Some(segment) = state.active_segment()
            && sequence_in_segment(sequence, segment)
        {
            let read_segment = tail_start + state.sealed_tail().len();
            let segment_offset = sequence_offset_in_segment(sequence, segment)?;

            return Ok(TraceReadCursor {
                read_segment,
                read_index: segment_offset.0,
                read_offset: segment_offset.1,
                next_sequence: sequence,
                validated_segment: None,
            });
        }

        // fall through to the visible end boundary
        Ok(TraceReadCursor {
            read_segment: state.segment_count(),
            read_index: 0,
            read_offset: 0,
            next_sequence: state.next_sequence(),
            validated_segment: None,
        })
    }
}

/// Return whether one sequence falls within one segment.
fn sequence_in_segment(sequence: TraceSequence, segment: &TraceSegment) -> bool {
    if segment.event_lengths.is_empty() {
        return false;
    }

    let sequence_value = sequence.get();
    let sequence_start = segment.header.sequence_start.get();
    let sequence_end = segment.header.sequence_end.get();

    sequence_value >= sequence_start && sequence_value <= sequence_end
}

/// Return the event index and byte offset for one sequence inside one segment.
fn sequence_offset_in_segment(
    sequence: TraceSequence,
    segment: &TraceSegment,
) -> RuntimeResult<(usize, usize)> {
    let sequence_start = segment.header.sequence_start.get();
    let sequence_value = sequence.get();
    let relative_index = sequence_value.checked_sub(sequence_start).ok_or_else(|| {
        RuntimeError::TraceMismatch {
            name: "sequence".to_string(),
        }
        .boxed()
    })? as usize;

    let read_offset =
        segment
            .event_lengths
            .iter()
            .take(relative_index)
            .try_fold(0usize, |offset, length| {
                offset.checked_add(*length as usize).ok_or_else(|| {
                    RuntimeError::TraceMismatch {
                        name: "offset".to_string(),
                    }
                    .boxed()
                })
            })?;

    Ok((relative_index, read_offset))
}

/// Validate segment integrity against stored metadata.
fn validate_segment(segment: &TraceSegment, trailer: &TraceTrailer) -> RuntimeResult<()> {
    if segment.header.event_count as usize != segment.event_lengths.len() {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_events".to_string(),
        }
        .boxed());
    }
    if segment.header.byte_length as usize != segment.data.len() {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_length".to_string(),
        }
        .boxed());
    }
    let total_event_bytes: usize = segment
        .event_lengths
        .iter()
        .map(|value| *value as usize)
        .sum();
    if total_event_bytes != segment.data.len() {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_offsets".to_string(),
        }
        .boxed());
    }

    // validate the segment payload checksum
    let checksum = segment.payload_checksum();
    if checksum != segment.header.checksum {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_checksum".to_string(),
        }
        .boxed());
    }

    // validate the corresponding trailer entry
    let entry = trailer
        .segments
        .iter()
        .find(|entry| entry.index == segment.header.index)
        .ok_or_else(|| {
            RuntimeError::TraceMismatch {
                name: "chunk_index".to_string(),
            }
            .boxed()
        })?;
    if entry.length != segment.header.byte_length {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_length".to_string(),
        }
        .boxed());
    }
    if entry.checksum != segment.header.checksum {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_checksum".to_string(),
        }
        .boxed());
    }

    Ok(())
}
