use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::BranchId;
use crate::world::trace::{
    TRACE_DEFAULT_MAX_CHUNK_SIZE_BYTES, TRACE_DEFAULT_MAX_ENTRIES_PER_CHUNK, TraceCheckpointIndex,
    TraceCursor, TraceHeader, TraceTag,
};

use super::chunk::TraceChunk;
use super::constants::{TRACE_ENTRY_LENGTH_BYTES, TRACE_ENTRY_TAG_BYTES};
use super::file::TraceFile;

/// Sequence number for entries within a trace log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TraceSequence(u64);

impl TraceSequence {
    /// Create a new trace sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw sequence number.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next sequence number.
    pub fn next(self) -> RuntimeResult<Self> {
        let value = self.0.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "trace sequence space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(Self(value))
    }
}

/// One live trace-log state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceState {
    /// Trace log header metadata.
    header: TraceHeader,
    /// Checkpoints anchored in this trace.
    checkpoints: Vec<TraceCheckpointIndex>,
    /// Next sequence number to assign.
    next_sequence: TraceSequence,
    /// Completed chunks in trace order.
    sealed: Vec<TraceChunk>,
    /// Current mutable append chunk.
    active: TraceChunk,
}

impl TraceState {
    /// Return the active branch identifier.
    pub(super) fn branch_id(&self) -> BranchId {
        self.header.branch_id
    }

    /// Return the next trace sequence number.
    pub(crate) fn next_sequence(&self) -> TraceSequence {
        self.next_sequence
    }

    /// Return the completed chunks.
    pub(super) fn sealed_chunks(&self) -> &[TraceChunk] {
        &self.sealed
    }

    /// Return the active mutable chunk when it stores entries.
    pub(super) fn active_chunk(&self) -> Option<&TraceChunk> {
        (!self.active.is_empty()).then_some(&self.active)
    }

    /// Return the total number of visible chunks.
    pub(super) fn chunk_count(&self) -> usize {
        let active_chunk_count = usize::from(!self.active.is_empty());

        self.sealed.len() + active_chunk_count
    }

    /// Return one visible chunk by stable chunk index.
    pub(super) fn chunk(&self, index: usize) -> Option<&TraceChunk> {
        if index < self.sealed.len() {
            self.sealed.get(index)
        } else if index == self.sealed.len() {
            self.active_chunk()
        } else {
            None
        }
    }

    /// Finalize one non-empty active chunk into the completed chunk list.
    fn seal_active_chunk(&mut self, next_sequence_start: TraceSequence) {
        // skip sealing empty chunks
        if self.active.is_empty() {
            self.active = TraceChunk::new(next_sequence_start);
            return;
        }

        let sealed_chunk =
            std::mem::replace(&mut self.active, TraceChunk::new(next_sequence_start));
        self.sealed.push(sealed_chunk);
    }

    /// Return the visible chunks in trace-file order.
    fn chunks(&self) -> Vec<TraceChunk> {
        let mut chunks = Vec::with_capacity(self.chunk_count());
        chunks.extend(self.sealed.iter().cloned());

        if let Some(active) = self.active_chunk() {
            chunks.push(active.clone());
        }

        chunks
    }
}

/// Backing store for deterministic trace entries.
#[derive(Debug, Clone)]
pub(crate) struct TraceStore {
    /// Shared trace store state.
    state: Arc<Mutex<TraceState>>,
}

impl TraceStore {
    /// Create a trace store with an explicit header.
    pub(crate) fn new(mut header: TraceHeader) -> Self {
        // normalize chunk limits
        if header.max_entries_per_chunk == 0 {
            header.max_entries_per_chunk = TRACE_DEFAULT_MAX_ENTRIES_PER_CHUNK;
        }
        if header.max_chunk_size_bytes == 0 {
            header.max_chunk_size_bytes = TRACE_DEFAULT_MAX_CHUNK_SIZE_BYTES;
        }

        let next_sequence = TraceSequence::new(0);

        Self {
            state: Arc::new(Mutex::new(TraceState {
                header,
                checkpoints: Vec::new(),
                next_sequence,
                sealed: Vec::new(),
                active: TraceChunk::new(next_sequence),
            })),
        }
    }

    /// Return the trace store header.
    pub(crate) fn header(&self) -> TraceHeader {
        let state = self.state.lock();

        state.header.clone()
    }

    /// Set the current branch identifier on the trace header.
    pub(crate) fn set_branch_id(&self, branch_id: BranchId) {
        let mut state = self.state.lock();
        state.header.branch_id = branch_id;
    }

    /// Create a trace cursor for this store.
    pub(crate) fn reader(&self) -> TraceCursor {
        TraceCursor::new(self.state.clone())
    }

    /// Return the next sequence number.
    pub(crate) fn next_sequence(&self) -> TraceSequence {
        let state = self.state.lock();

        state.next_sequence
    }

    /// Capture one full trace file.
    pub(super) fn file(&self) -> TraceFile {
        let mut state = self.state.lock();

        // finalize the active chunk before capturing a stable file
        let next_sequence = state.next_sequence;
        state.seal_active_chunk(next_sequence);

        TraceFile::new(
            state.header.clone(),
            state.chunks(),
            state.checkpoints.clone(),
        )
    }

    /// Restore one full trace file.
    pub(super) fn restore_file(&self, file: TraceFile) -> RuntimeResult<()> {
        file.validate()?;

        let mut current = self.state.lock();
        let next_sequence = file.next_sequence()?;
        let (header, chunks, trailer) = file.into_parts();
        let checkpoints = trailer.checkpoints;

        *current = TraceState {
            header,
            checkpoints,
            next_sequence,
            sealed: chunks,
            active: TraceChunk::new(next_sequence),
        };

        Ok(())
    }

    /// Append one tagged trace payload to the store.
    pub(crate) fn record_encoded(
        &self,
        tag: TraceTag,
        encoded: &[u8],
    ) -> RuntimeResult<TraceSequence> {
        let payload_len = encoded.len() as u64;
        let encoded_len = payload_len + TRACE_ENTRY_TAG_BYTES as u64;
        if encoded_len > u32::MAX as u64 {
            return Err(RuntimeError::trace_encode_failed("entry".to_string()).boxed());
        }
        let record_len = encoded_len + TRACE_ENTRY_LENGTH_BYTES as u64;

        let mut state = self.state.lock();
        let sequence = state.next_sequence;
        let next_sequence = sequence.get().checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "trace sequence space exhausted".to_string(),
            }
            .boxed()
        })?;
        state.next_sequence = TraceSequence::new(next_sequence);

        // rotate the active chunk before appending when it is full
        let is_rotation_required = !state.active.is_empty()
            && state.active.should_rotate_for_entry(
                record_len,
                state.header.max_entries_per_chunk as usize,
                state.header.max_chunk_size_bytes,
            );
        if is_rotation_required {
            state.seal_active_chunk(sequence);
        }

        // append the encoded entry to the active chunk
        let chunk = &mut state.active;
        let start = chunk.bytes.len();
        let payload_start = start + TRACE_ENTRY_LENGTH_BYTES;
        let end = start + record_len as usize;
        chunk.bytes.resize(end, 0);
        chunk.bytes[start..payload_start].copy_from_slice(&(encoded_len as u32).to_le_bytes());
        chunk.bytes[payload_start] = tag.byte();
        chunk.bytes[payload_start + TRACE_ENTRY_TAG_BYTES..end].copy_from_slice(encoded);
        chunk.header.entry_count = chunk.header.entry_count.checked_add(1).ok_or_else(|| {
            RuntimeError::Internal {
                message: "trace chunk entry count space exhausted".to_string(),
            }
            .boxed()
        })?;

        Ok(sequence)
    }

    /// Record a checkpoint index entry with an explicit sequence boundary.
    pub(crate) fn record_checkpoint_exact(
        &self,
        checkpoint: TraceCheckpointIndex,
    ) -> RuntimeResult<()> {
        let sequence = checkpoint.sequence.get();

        // append the checkpoint entry in sequence order
        let mut state = self.state.lock();
        let insert_index = state
            .checkpoints
            .partition_point(|entry| entry.sequence.get() <= sequence);
        state.checkpoints.insert(insert_index, checkpoint);

        Ok(())
    }
}
