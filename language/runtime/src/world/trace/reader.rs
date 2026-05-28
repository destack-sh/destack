use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::BranchId;
use crate::world::trace::TraceRecord;
use crate::world::trace::log::{TraceSequence, TraceState};

use super::chunk::{TRACE_EVENT_LENGTH_BYTES, TraceChunk, TracePrefix};

/// One cached immutable prefix range for one cursor.
#[derive(Debug)]
struct TracePrefixRange {
    /// First global chunk index stored in this prefix.
    start_chunk: usize,
    /// Shared immutable trace prefix.
    prefix: Arc<TracePrefix>,
}

/// Cached immutable prefix path for one trace cursor.
#[derive(Debug, Default)]
struct TracePrefixCache {
    /// Current shared prefix this cache was built from.
    head: Option<Arc<TracePrefix>>,
    /// Shared history ranges in oldest-to-newest order.
    ranges: Vec<TracePrefixRange>,
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
    /// Cached shared prefix path for chunk reads.
    prefix_cache: TracePrefixCache,
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
            prefix_cache: TracePrefixCache::default(),
        }
    }

    /// Read the next recorded trace record if available.
    pub(crate) fn next_event(&mut self) -> RuntimeResult<Option<TraceRecord>> {
        let state = self.state.lock();

        // refresh the shared prefix cache for this visible head
        Self::sync_prefix_cache(&state, &mut self.prefix_cache);

        // read the next event by advancing across the visible chunks
        let cursor = &mut self.cursor;

        while cursor.read_chunk < state.chunk_count() {
            let chunk_index = cursor.read_chunk;
            let chunk = Self::chunk(&state, &self.prefix_cache, chunk_index).ok_or_else(|| {
                RuntimeError::inconsistent_image(format!("trace chunk {chunk_index} is missing"))
                    .boxed()
            })?;

            // validate one chunk the first time this cursor reads it
            if cursor.validated_chunk != Some(chunk_index) {
                validate_chunk(chunk)?;
                cursor.validated_chunk = Some(chunk_index);
            }

            // decode the next event from the current chunk
            if cursor.read_index < chunk.header.event_count as usize {
                let sequence = cursor.next_sequence;
                if cursor.read_index == 0 && chunk.header.sequence_start != sequence {
                    return Err(RuntimeError::trace_mismatch("sequence".to_string()).boxed());
                }

                let start = cursor.read_offset;
                let length_end = start + TRACE_EVENT_LENGTH_BYTES;
                if length_end > chunk.bytes.len() {
                    return Err(RuntimeError::trace_mismatch("chunk_length".to_string()).boxed());
                }

                let mut event_length = [0u8; 4];
                event_length.copy_from_slice(&chunk.bytes[start..length_end]);
                let event_length = u32::from_le_bytes(event_length) as usize;
                let end = length_end + event_length;
                if end > chunk.bytes.len() {
                    return Err(RuntimeError::trace_mismatch("chunk_length".to_string()).boxed());
                }
                let encoded = &chunk.bytes[length_end..end];
                let event = postcard::from_bytes(encoded)
                    .map_err(|_| RuntimeError::trace_decode_failed("event".to_string()).boxed())?;
                if cursor.read_index + 1 == chunk.header.event_count as usize
                    && chunk.sequence_end() != sequence
                {
                    return Err(RuntimeError::trace_mismatch("sequence".to_string()).boxed());
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
            return Err(RuntimeError::trace_mismatch("branch".to_string()).boxed());
        }
        if image.cursor.next_sequence.get() > image.upper_bound.get() {
            return Err(RuntimeError::trace_mismatch("sequence".to_string()).boxed());
        }
        if image.upper_bound != state.next_sequence() {
            return Err(RuntimeError::trace_mismatch("upper_bound".to_string()).boxed());
        }

        self.cursor = image.cursor;
        self.prefix_cache = TracePrefixCache::default();

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
            return Err(RuntimeError::trace_mismatch("sequence".to_string()).boxed());
        }

        // refresh the shared prefix cache before scanning from the start
        Self::sync_prefix_cache(&state, &mut self.prefix_cache);
        self.cursor = Self::seek_chunk(&state, &self.prefix_cache, sequence)?;

        Ok(())
    }

    /// Rebuild the cached chunk path when the shared head changes.
    fn sync_prefix_cache(state: &TraceState, cache: &mut TracePrefixCache) {
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

        let mut prefixes = Vec::new();
        let mut current = Some(head.clone());

        // collect newest-to-oldest first
        while let Some(prefix) = current {
            current = prefix.parent.clone();
            prefixes.push(prefix);
        }

        // reverse into oldest-to-newest order for sequential reads
        prefixes.reverse();

        let mut next_start_chunk = 0usize;
        let ranges = prefixes
            .into_iter()
            .map(|prefix| {
                let range = TracePrefixRange {
                    start_chunk: next_start_chunk,
                    prefix: prefix.clone(),
                };
                next_start_chunk += prefix.chunks.len();
                range
            })
            .collect();

        cache.head = Some(head.clone());
        cache.ranges = ranges;
    }

    /// Return one visible chunk by stable global index.
    fn chunk<'a>(
        state: &'a TraceState,
        cache: &'a TracePrefixCache,
        index: usize,
    ) -> Option<&'a TraceChunk> {
        // read shared immutable history first
        for range in &cache.ranges {
            let range_end = range.start_chunk + range.prefix.chunks.len();

            if index < range_end {
                return range.prefix.chunks.get(index - range.start_chunk);
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
        cache: &TracePrefixCache,
        sequence: TraceSequence,
    ) -> RuntimeResult<TraceReadCursor> {
        // shared immutable history
        for range in &cache.ranges {
            for (local_chunk_index, chunk) in range.prefix.chunks.iter().enumerate() {
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
    if chunk.header.event_count == 0 {
        return false;
    }

    let sequence_value = sequence.get();
    let sequence_start = chunk.header.sequence_start.get();
    let sequence_end = chunk.sequence_end().get();

    sequence_value >= sequence_start && sequence_value <= sequence_end
}

/// Return the event index and byte offset for one sequence inside one chunk.
fn sequence_offset_in_chunk(
    sequence: TraceSequence,
    chunk: &TraceChunk,
) -> RuntimeResult<(usize, usize)> {
    let sequence_start = chunk.header.sequence_start.get();
    let sequence_value = sequence.get();
    let relative_index = sequence_value
        .checked_sub(sequence_start)
        .ok_or_else(|| RuntimeError::trace_mismatch("sequence".to_string()).boxed())?
        as usize;

    let read_offset = event_offset_in_chunk(chunk, relative_index)?;

    Ok((relative_index, read_offset))
}

/// Validate chunk integrity against stored metadata.
fn validate_chunk(chunk: &TraceChunk) -> RuntimeResult<()> {
    if event_offset_in_chunk(chunk, chunk.header.event_count as usize)? != chunk.bytes.len() {
        return Err(RuntimeError::trace_mismatch("chunk_offsets".to_string()).boxed());
    }

    Ok(())
}

/// Return the byte offset for one event index inside one chunk.
fn event_offset_in_chunk(chunk: &TraceChunk, event_index: usize) -> RuntimeResult<usize> {
    let mut offset = 0usize;

    for _ in 0..event_index {
        let length_end = offset + TRACE_EVENT_LENGTH_BYTES;
        if length_end > chunk.bytes.len() {
            return Err(RuntimeError::trace_mismatch("offset".to_string()).boxed());
        }

        let mut event_length = [0u8; 4];
        event_length.copy_from_slice(&chunk.bytes[offset..length_end]);
        let event_length = u32::from_le_bytes(event_length) as usize;
        offset = length_end
            .checked_add(event_length)
            .ok_or_else(|| RuntimeError::trace_mismatch("offset".to_string()).boxed())?;
    }

    Ok(offset)
}
