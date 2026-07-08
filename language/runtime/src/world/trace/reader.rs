use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::BranchId;
use crate::world::trace::TraceEntry;

use super::chunk::TraceChunk;
use super::store::{TraceSequence, TraceState};

/// Trace cursor state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct TraceReadCursor {
    /// Current chunk index.
    read_chunk: usize,
    /// Current entry index inside the chunk.
    read_index: usize,
    /// Current byte offset inside the chunk payload.
    read_offset: usize,
    /// Next expected sequence number.
    next_sequence: TraceSequence,
    /// Last chunk index that was validated.
    validated_chunk: Option<usize>,
}

impl TraceReadCursor {
    /// Return whether this cursor points at a stored entry in the chunk.
    fn has_entry_in(&self, chunk: &TraceChunk) -> bool {
        self.read_index < chunk.header.entry_count as usize
    }

    /// Validate cursor sequence invariants for one chunk.
    fn validate_sequence(&self, chunk: &TraceChunk) -> RuntimeResult<()> {
        if self.read_index == 0 && chunk.header.sequence_start != self.next_sequence {
            return Err(RuntimeError::trace_mismatch("sequence".to_string()).boxed());
        }

        if self.read_index + 1 == chunk.header.entry_count as usize
            && chunk.sequence_end() != self.next_sequence
        {
            return Err(RuntimeError::trace_mismatch("sequence".to_string()).boxed());
        }

        Ok(())
    }

    /// Advance this cursor past one decoded entry.
    fn advance_entry(&mut self, end_offset: usize) -> RuntimeResult<TraceSequence> {
        let sequence = self.next_sequence;
        self.read_index += 1;
        self.read_offset = end_offset;
        self.next_sequence = sequence.next()?;

        Ok(sequence)
    }

    /// Advance this cursor to the next chunk.
    fn advance_chunk(&mut self) {
        self.read_chunk += 1;
        self.read_index = 0;
        self.read_offset = 0;
    }
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
        }
    }

    /// Read the next recorded trace entry if available.
    pub(crate) fn next_entry(&mut self) -> RuntimeResult<Option<TraceEntry>> {
        let state = self.state.lock();

        // scan chunks until one entry is available
        while self.cursor.read_chunk < state.chunk_count() {
            let chunk_index = self.cursor.read_chunk;
            let chunk = state.chunk(chunk_index).ok_or_else(|| {
                RuntimeError::inconsistent_image(format!("trace chunk {chunk_index} is missing"))
                    .boxed()
            })?;

            // validate chunk offsets once per cursor
            if self.cursor.validated_chunk != Some(chunk_index) {
                chunk.validate_entries()?;
                self.cursor.validated_chunk = Some(chunk_index);
            }

            // decode the current entry
            if self.cursor.has_entry_in(chunk) {
                self.cursor.validate_sequence(chunk)?;
                let (trace, end_offset) = chunk.trace_at(self.cursor.read_offset)?;
                let sequence = self.cursor.advance_entry(end_offset)?;

                return Ok(Some(TraceEntry { sequence, trace }));
            }

            // otherwise advance to the next chunk
            self.cursor.advance_chunk();
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

        self.cursor = Self::seek_chunk(&state, sequence)?;

        Ok(())
    }

    /// Seek one cursor directly to the chunk containing the requested sequence.
    fn seek_chunk(state: &TraceState, sequence: TraceSequence) -> RuntimeResult<TraceReadCursor> {
        // completed chunks
        for (chunk_index, chunk) in state.sealed_chunks().iter().enumerate() {
            if !chunk.contains_sequence(sequence) {
                continue;
            }

            let chunk_offset = chunk.sequence_offset(sequence)?;

            return Ok(TraceReadCursor {
                read_chunk: chunk_index,
                read_index: chunk_offset.0,
                read_offset: chunk_offset.1,
                next_sequence: sequence,
                validated_chunk: None,
            });
        }

        // active tail
        if let Some(chunk) = state.active_chunk()
            && chunk.contains_sequence(sequence)
        {
            let chunk_offset = chunk.sequence_offset(sequence)?;

            return Ok(TraceReadCursor {
                read_chunk: state.sealed_chunks().len(),
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
