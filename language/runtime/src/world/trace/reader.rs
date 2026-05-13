use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::BranchId;
use crate::world::trace::log::{TraceSequence, TraceState, compute_log_hash};
use crate::world::trace::{TraceRecord, TraceTrailer};

use super::chunk::{TraceChunk, TraceChunkChain};

/// One cached immutable chain range for one cursor.
#[derive(Debug)]
struct TraceChunkRange {
    /// First global chunk index stored in this chain node.
    start_chunk: usize,
    /// Shared immutable chunk chain.
    chain: Arc<TraceChunkChain>,
}

/// Cached immutable chunk chain path for one trace cursor.
#[derive(Debug, Default)]
struct TraceChunkCache {
    /// Current shared head chain this cache was built from.
    head: Option<Arc<TraceChunkChain>>,
    /// Shared history ranges in oldest-to-newest order.
    ranges: Vec<TraceChunkRange>,
}

/// Trace cursor state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct TraceReadCursor {
    /// Current chunk index.
    read_chunk: usize,
    /// Current event index inside the chunk.
    read_index: usize,
    /// Current byte offset inside the chunk payload.
    read_offset: usize,
    /// Next expected sequence number.
    next_sequence: TraceSequence,
    /// Last chunk index that was validated.
    validated_chunk: Option<usize>,
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
    state: Arc<parking_lot::Mutex<TraceState>>,
    /// Cursor state for this reader.
    cursor: TraceReadCursor,
    /// Cached shared chain path for chunk reads.
    chunk_cache: TraceChunkCache,
}

impl TraceCursor {
    /// Create a trace cursor for the given trace-log state.
    pub(super) fn new(state: Arc<parking_lot::Mutex<TraceState>>) -> Self {
        Self {
            state,
            cursor: TraceReadCursor {
                read_chunk: 0,
                read_index: 0,
                read_offset: 0,
                next_sequence: TraceSequence::new(0),
                validated_chunk: None,
            },
            chunk_cache: TraceChunkCache::default(),
        }
    }

    /// Read the next recorded trace record if available.
    pub(crate) fn next_event(&mut self) -> RuntimeResult<Option<TraceRecord>> {
        // validate the live trailer hash before consuming one event
        let state = self.state.lock();
        let trailer = state.trailer();
        let log_hash = compute_log_hash(&trailer.chunks, &trailer.checkpoints);
        if trailer.log_hash != 0 && trailer.log_hash != log_hash {
            return Err(RuntimeError::TraceMismatch {
                name: "log_hash".to_string(),
            }
            .boxed());
        }

        // refresh the shared chunk cache for this visible head
        Self::sync_chunk_cache(&state, &mut self.chunk_cache);

        // read the next event by advancing across the visible chunks
        let cursor = &mut self.cursor;

        while cursor.read_chunk < state.chunk_count() {
            let chunk_index = cursor.read_chunk;
            let chunk = Self::chunk(&state, &self.chunk_cache, chunk_index).ok_or_else(|| {
                RuntimeError::InconsistentImage {
                    detail: format!("trace chunk {chunk_index} is missing"),
                }
                .boxed()
            })?;

            // validate one chunk the first time this cursor reads it
            if cursor.validated_chunk != Some(chunk_index) {
                validate_chunk(chunk, trailer)?;
                cursor.validated_chunk = Some(chunk_index);
            }

            // decode the next event from the current chunk
            if cursor.read_index < chunk.event_lengths.len() {
                let sequence = cursor.next_sequence;
                if cursor.read_index == 0 && chunk.header.sequence_start != sequence {
                    return Err(RuntimeError::TraceMismatch {
                        name: "sequence".to_string(),
                    }
                    .boxed());
                }

                let event_length = chunk.event_lengths[cursor.read_index] as usize;
                let start = cursor.read_offset;
                let end = start + event_length;
                let encoded = &chunk.data[start..end];
                let event = postcard::from_bytes(encoded).map_err(|_| {
                    RuntimeError::TraceDecodeFailed {
                        name: "event".to_string(),
                    }
                    .boxed()
                })?;
                if cursor.read_index + 1 == chunk.event_lengths.len()
                    && chunk.header.sequence_end != sequence
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

            // otherwise advance into the next chunk
            cursor.read_chunk += 1;
            cursor.read_index = 0;
            cursor.read_offset = 0;
        }

        Ok(None)
    }

    /// Capture the reader cursor state.
    pub(crate) fn capture_image(&self) -> TraceCursorImage {
        let state = self.state.lock();

        TraceCursorImage {
            branch_id: state.branch_id(),
            upper_bound: state.next_sequence(),
            cursor: self.cursor,
        }
    }

    /// Restore the reader cursor state.
    pub(crate) fn restore_image(&mut self, image: TraceCursorImage) -> RuntimeResult<()> {
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

        self.cursor = image.cursor;
        self.chunk_cache = TraceChunkCache::default();

        Ok(())
    }

    /// Return the next sequence visible through this cursor.
    pub(crate) fn sequence(&self) -> TraceSequence {
        self.cursor.next_sequence
    }

    /// Seek this cursor to one sequence boundary.
    pub(crate) fn seek_sequence(&mut self, sequence: TraceSequence) -> RuntimeResult<()> {
        // reject seeks beyond the visible log tail
        let state = self.state.lock();
        if sequence.get() > state.next_sequence().get() {
            return Err(RuntimeError::TraceMismatch {
                name: "sequence".to_string(),
            }
            .boxed());
        }

        // refresh the shared chunk cache before scanning from the start
        Self::sync_chunk_cache(&state, &mut self.chunk_cache);
        self.cursor = Self::seek_chunk(&state, &self.chunk_cache, sequence)?;

        Ok(())
    }

    /// Rebuild the cached chunk path when the shared head changes.
    fn sync_chunk_cache(state: &TraceState, cache: &mut TraceChunkCache) {
        let Some(head) = state.head() else {
            cache.head = None;
            cache.ranges.clear();
            return;
        };

        if cache
            .head
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, head))
        {
            return;
        }

        let mut chains = Vec::new();
        let mut current = Some(head.clone());

        // collect newest-to-oldest first
        while let Some(chain) = current {
            current = chain.parent.clone();
            chains.push(chain);
        }

        // reverse into oldest-to-newest order for sequential reads
        chains.reverse();

        let mut next_start_chunk = 0usize;
        let ranges = chains
            .into_iter()
            .map(|chain| {
                let range = TraceChunkRange {
                    start_chunk: next_start_chunk,
                    chain: chain.clone(),
                };
                next_start_chunk += chain.chunks.len();
                range
            })
            .collect();

        cache.head = Some(head.clone());
        cache.ranges = ranges;
    }

    /// Return one visible chunk by stable global index.
    fn chunk<'a>(
        state: &'a TraceState,
        cache: &'a TraceChunkCache,
        index: usize,
    ) -> Option<&'a TraceChunk> {
        // read shared immutable history first
        for range in &cache.ranges {
            let range_end = range.start_chunk + range.chain.chunks.len();

            if index < range_end {
                return range.chain.chunks.get(index - range.start_chunk);
            }
        }

        let chunk_index = index.saturating_sub(state.head_chunk_count());

        if chunk_index < state.sealed_tail().len() {
            return state.sealed_tail().get(chunk_index);
        }

        if chunk_index == state.sealed_tail().len() {
            return state.active_chunk();
        }

        None
    }

    /// Seek one cursor directly to the chunk containing the requested sequence.
    fn seek_chunk(
        state: &TraceState,
        cache: &TraceChunkCache,
        sequence: TraceSequence,
    ) -> RuntimeResult<TraceReadCursor> {
        // shared immutable history
        for range in &cache.ranges {
            for (local_chunk_index, chunk) in range.chain.chunks.iter().enumerate() {
                if !sequence_in_chunk(sequence, chunk) {
                    continue;
                }

                let read_chunk = range.start_chunk + local_chunk_index;
                let chunk_offset = sequence_offset_in_chunk(sequence, chunk)?;

                return Ok(TraceReadCursor {
                    read_chunk,
                    read_index: chunk_offset.0,
                    read_offset: chunk_offset.1,
                    next_sequence: sequence,
                    validated_chunk: None,
                });
            }
        }

        let tail_start = state.head_chunk_count();

        // local sealed tail
        for (local_chunk_index, chunk) in state.sealed_tail().iter().enumerate() {
            if !sequence_in_chunk(sequence, chunk) {
                continue;
            }

            let read_chunk = tail_start + local_chunk_index;
            let chunk_offset = sequence_offset_in_chunk(sequence, chunk)?;

            return Ok(TraceReadCursor {
                read_chunk,
                read_index: chunk_offset.0,
                read_offset: chunk_offset.1,
                next_sequence: sequence,
                validated_chunk: None,
            });
        }

        // active tail
        if let Some(chunk) = state.active_chunk()
            && sequence_in_chunk(sequence, chunk)
        {
            let read_chunk = tail_start + state.sealed_tail().len();
            let chunk_offset = sequence_offset_in_chunk(sequence, chunk)?;

            return Ok(TraceReadCursor {
                read_chunk,
                read_index: chunk_offset.0,
                read_offset: chunk_offset.1,
                next_sequence: sequence,
                validated_chunk: None,
            });
        }

        // fall through to the visible end boundary
        Ok(TraceReadCursor {
            read_chunk: state.chunk_count(),
            read_index: 0,
            read_offset: 0,
            next_sequence: state.next_sequence(),
            validated_chunk: None,
        })
    }
}

/// Return whether one sequence falls within one chunk.
fn sequence_in_chunk(sequence: TraceSequence, chunk: &TraceChunk) -> bool {
    if chunk.event_lengths.is_empty() {
        return false;
    }

    let sequence_value = sequence.get();
    let sequence_start = chunk.header.sequence_start.get();
    let sequence_end = chunk.header.sequence_end.get();

    sequence_value >= sequence_start && sequence_value <= sequence_end
}

/// Return the event index and byte offset for one sequence inside one chunk.
fn sequence_offset_in_chunk(
    sequence: TraceSequence,
    chunk: &TraceChunk,
) -> RuntimeResult<(usize, usize)> {
    let sequence_start = chunk.header.sequence_start.get();
    let sequence_value = sequence.get();
    let relative_index = sequence_value.checked_sub(sequence_start).ok_or_else(|| {
        RuntimeError::TraceMismatch {
            name: "sequence".to_string(),
        }
        .boxed()
    })? as usize;

    let read_offset =
        chunk
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

/// Validate chunk integrity against stored metadata.
fn validate_chunk(chunk: &TraceChunk, trailer: &TraceTrailer) -> RuntimeResult<()> {
    if chunk.header.event_count as usize != chunk.event_lengths.len() {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_events".to_string(),
        }
        .boxed());
    }
    if chunk.header.byte_length as usize != chunk.data.len() {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_length".to_string(),
        }
        .boxed());
    }
    let total_event_bytes: usize = chunk
        .event_lengths
        .iter()
        .map(|value| *value as usize)
        .sum();
    if total_event_bytes != chunk.data.len() {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_offsets".to_string(),
        }
        .boxed());
    }

    // validate the chunk payload checksum
    let checksum = chunk.payload_checksum();
    if checksum != chunk.header.checksum {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_checksum".to_string(),
        }
        .boxed());
    }

    // validate the corresponding trailer entry
    let entry = trailer
        .chunks
        .iter()
        .find(|entry| entry.index == chunk.header.index)
        .ok_or_else(|| {
            RuntimeError::TraceMismatch {
                name: "chunk_index".to_string(),
            }
            .boxed()
        })?;
    if entry.length != chunk.header.byte_length {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_length".to_string(),
        }
        .boxed());
    }
    if entry.checksum != chunk.header.checksum {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_checksum".to_string(),
        }
        .boxed());
    }

    Ok(())
}
