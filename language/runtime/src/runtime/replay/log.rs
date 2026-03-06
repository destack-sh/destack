use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{
    ReplayCheckpointIndex, ReplayChunkIndex, ReplayEvent, ReplayHeader, ReplayLogReader,
    ReplayTrailer,
};
use destack_base::{FNV_OFFSET_BASIS_128, fnv1a_128_update};
use postcard::experimental::serialized_size;

use super::chunk::ReplayChunk;

/// Identifier for a replay branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(u128);

impl BranchId {
    /// Create a new branch identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw branch identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Identifier for a replay checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(u128);

impl CheckpointId {
    /// Create a new checkpoint identifier.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Return the raw checkpoint identifier value.
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Default maximum number of events in a chunk.
const DEFAULT_MAX_EVENTS_PER_CHUNK: usize = 1024;
/// Default maximum chunk size in bytes.
const DEFAULT_MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// Sequence number for events within a replay log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogSequence(u64);

impl LogSequence {
    /// Create a new log sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw sequence number.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next sequence number.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// In-memory replay log state.
#[derive(Debug, Clone)]
pub(super) struct ReplayLogState {
    /// Replay log header metadata.
    header: ReplayHeader,
    /// Replay log trailer metadata.
    trailer: ReplayTrailer,
    /// Branch identifier for this log stream.
    branch_id: BranchId,
    /// Maximum number of events per chunk.
    max_events_per_chunk: usize,
    /// Maximum chunk size in bytes.
    max_chunk_bytes: u64,
    /// Next sequence number to assign.
    next_sequence: LogSequence,
    /// Next chunk offset for trailer entries.
    next_offset: u64,
    /// Recorded chunks for replay.
    pub(super) chunks: Vec<ReplayChunk>,
}

impl ReplayLogState {
    /// Return the replay log trailer.
    pub(super) fn trailer(&self) -> &ReplayTrailer {
        &self.trailer
    }
}

/// Record and replay log for deterministic execution.
#[derive(Debug, Clone)]
pub struct ReplayLog {
    // NOTE #Incomplete: persist chunks to disk and stream across threads
    /// Shared replay log state.
    state: Arc<Mutex<ReplayLogState>>,
}

impl Default for ReplayLog {
    fn default() -> Self {
        Self::new(ReplayHeader::default())
    }
}

impl ReplayLog {
    /// Create a replay log with an explicit header.
    pub fn new(mut header: ReplayHeader) -> Self {
        // normalize chunk limits
        if header.max_events_per_chunk == 0 {
            header.max_events_per_chunk = DEFAULT_MAX_EVENTS_PER_CHUNK as u32;
        }
        if header.max_chunk_bytes == 0 {
            header.max_chunk_bytes = DEFAULT_MAX_CHUNK_BYTES;
        }

        let max_events_per_chunk = header.max_events_per_chunk as usize;
        let max_chunk_bytes = header.max_chunk_bytes;

        // seed the first chunk and trailer entry
        let chunk = ReplayChunk::new(0, LogSequence::new(0));
        let trailer = ReplayTrailer {
            chunks: vec![ReplayChunkIndex {
                index: 0,
                offset: 0,
                length: 0,
                checksum: 0,
            }],
            checkpoints: Vec::new(),
            log_hash: 0,
        };

        Self {
            state: Arc::new(Mutex::new(ReplayLogState {
                header,
                trailer,
                branch_id: BranchId::new(0),
                max_events_per_chunk,
                max_chunk_bytes,
                next_sequence: LogSequence::new(0),
                next_offset: 0,
                chunks: vec![chunk],
            })),
        }
    }

    /// Return the replay log header.
    pub fn header(&self) -> ReplayHeader {
        // lock state for reading
        let state = self.state.lock();
        state.header.clone()
    }

    /// Return the replay log trailer.
    pub fn trailer(&self) -> ReplayTrailer {
        // lock state for reading
        let state = self.state.lock();
        let mut trailer = state.trailer.clone();
        trailer.log_hash = compute_log_hash(&trailer.chunks, &trailer.checkpoints);
        trailer
    }

    /// Return the current branch identifier.
    pub fn branch_id(&self) -> BranchId {
        // lock state for reading
        let state = self.state.lock();
        state.branch_id
    }

    /// Create a replay reader for this log.
    pub fn reader(&self) -> ReplayLogReader {
        ReplayLogReader::new(self.state.clone())
    }

    /// Return the next sequence number.
    pub fn next_sequence(&self) -> LogSequence {
        // lock state for reading
        let state = self.state.lock();
        state.next_sequence
    }

    /// Record an event in the log.
    pub(crate) fn record_event(&self, event: ReplayEvent) -> RuntimeResult<LogSequence> {
        // compute the encoded size ahead of time
        let encoded_len = serialized_size(&event).map_err(|_| {
            RuntimeError::ReplayEncodeFailed {
                name: "event".to_string(),
            }
            .boxed()
        })? as u64;

        // lock state for mutation
        let mut state = self.state.lock();

        // assign the next sequence
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.next();

        // append the event payload
        let is_rotation_required = state.chunks.last().is_none_or(|chunk| {
            chunk.should_rotate_for_event(
                encoded_len,
                state.max_events_per_chunk,
                state.max_chunk_bytes,
            )
        });
        if is_rotation_required {
            // finalize the current chunk before creating a new one
            finalize_chunk(&mut state);

            // create the next chunk and trailer entry
            let next_index = state.chunks.len() as u32;
            let offset = state.next_offset;
            let chunk = ReplayChunk::new(next_index, sequence);
            state.chunks.push(chunk);
            state.trailer.chunks.push(ReplayChunkIndex {
                index: next_index,
                offset,
                length: 0,
                checksum: 0,
            });
        }

        // append the event to the active chunk
        let chunk = state
            .chunks
            .last_mut()
            .expect("replay log must have an active chunk");
        let start = chunk.data.len();
        let end = start + encoded_len as usize;
        chunk.data.resize(end, 0);
        let encoded_len = postcard::to_slice(&event, &mut chunk.data[start..end])
            .map_err(|_| {
                RuntimeError::ReplayEncodeFailed {
                    name: "event".to_string(),
                }
                .boxed()
            })?
            .len();
        let encoded_end = start + encoded_len;

        // checksum only the encoded event bytes
        chunk.update_checksum_for_range(start, encoded_end);

        // trim trailing capacity when serialized_size overestimates
        chunk.data.truncate(encoded_end);
        chunk.header.byte_length = chunk.data.len() as u64;
        chunk.event_lengths.push(encoded_len as u32);
        chunk.header.event_count = chunk.event_lengths.len() as u32;
        chunk.header.sequence_end = sequence;

        // keep the trailer entry in sync
        update_trailer_entry(&mut state);

        Ok(sequence)
    }

    /// Record a checkpoint index entry in trailer metadata.
    pub fn record_checkpoint(&self, mut checkpoint: ReplayCheckpointIndex) -> RuntimeResult<()> {
        // align the checkpoint with the next log sequence
        let sequence = self.next_sequence();
        checkpoint.sequence = sequence;

        // update trailer index
        let mut state = self.state.lock();
        state.trailer.checkpoints.push(checkpoint);

        Ok(())
    }
}

/// Finalize the active chunk trailer metadata before rotating.
fn finalize_chunk(state: &mut ReplayLogState) {
    // capture metadata for the trailing chunk entry
    let Some(chunk) = state.chunks.last() else {
        return;
    };
    if let Some(entry) = state.trailer.chunks.last_mut() {
        entry.length = chunk.header.byte_length;
        entry.checksum = chunk.header.checksum;
    }

    // advance the next chunk offset
    state.next_offset = state.next_offset.saturating_add(chunk.header.byte_length);
}

/// Refresh the trailing chunk trailer entry after one append.
fn update_trailer_entry(state: &mut ReplayLogState) {
    // update the trailing chunk index entry
    let Some(chunk) = state.chunks.last() else {
        return;
    };
    if let Some(entry) = state.trailer.chunks.last_mut() {
        entry.length = chunk.header.byte_length;
        entry.checksum = chunk.header.checksum;
    }
}

pub(super) fn compute_log_hash(
    chunks: &[ReplayChunkIndex],
    checkpoints: &[ReplayCheckpointIndex],
) -> u128 {
    // hash the chunk index metadata
    let mut hash = FNV_OFFSET_BASIS_128;
    for chunk in chunks {
        hash = fnv1a_128_update(hash, &chunk.index.to_le_bytes());
        hash = fnv1a_128_update(hash, &chunk.offset.to_le_bytes());
        hash = fnv1a_128_update(hash, &chunk.length.to_le_bytes());
        hash = fnv1a_128_update(hash, &chunk.checksum.to_le_bytes());
    }

    // hash the checkpoint metadata
    for checkpoint in checkpoints {
        hash = fnv1a_128_update(hash, &checkpoint.checkpoint_id.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.branch_id.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.sequence.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.size_bytes.to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.hash.to_le_bytes());
        hash = fnv1a_128_update(hash, checkpoint.path.as_bytes());
    }
    hash
}
