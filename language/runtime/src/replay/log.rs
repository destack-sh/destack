use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::replay::codec::decode_event;
use crate::replay::{
    BranchId, CheckpointEvent, LogSequence, ReplayCheckpointIndex, ReplayChunkHeader,
    ReplayChunkIndex, ReplayEvent, ReplayHeader, ReplayTrailer,
};
use destack_base::{FNV_OFFSET_BASIS_128, fnv1a_128_update};
use postcard::experimental::serialized_size;

/// Default maximum number of events in a chunk.
const DEFAULT_MAX_EVENTS_PER_CHUNK: usize = 1024;
/// Default maximum chunk size in bytes.
const DEFAULT_MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;
/// FNV-1a 64-bit offset basis.
const FNV_OFFSET_BASIS_64: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime.
const FNV_PRIME_64: u64 = 0x100000001b3;

/// In-memory replay log state.
#[derive(Debug, Clone)]
struct ReplayLogState {
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
    chunks: Vec<ReplayChunk>,
    /// Next event index for replay reads.
    read_chunk: usize,
    /// Next event index inside the current chunk.
    read_index: usize,
}

/// Replay log chunk payload.
#[derive(Debug, Clone)]
struct ReplayChunk {
    /// Chunk header metadata.
    header: ReplayChunkHeader,
    /// Chunk payload bytes.
    data: Vec<u8>,
    /// Recorded event offsets.
    events: Vec<ReplayChunkEvent>,
}

/// Offset metadata for a chunked replay event.
#[derive(Debug, Clone, Copy)]
struct ReplayChunkEvent {
    /// Offset into the chunk buffer.
    offset: u64,
    /// Byte length of the encoded event.
    length: u32,
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
        let chunk = new_chunk(0, LogSequence::new(0));
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
                read_chunk: 0,
                read_index: 0,
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

    /// Return the next sequence number.
    pub fn next_sequence(&self) -> LogSequence {
        // lock state for reading
        let state = self.state.lock();
        state.next_sequence
    }

    /// Record an event in the log.
    pub fn record_event(&self, event: ReplayEvent) -> RuntimeResult<LogSequence> {
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
        if should_rotate_chunk(
            state.chunks.last(),
            encoded_len,
            state.max_events_per_chunk,
            state.max_chunk_bytes,
        ) {
            // finalize the current chunk before creating a new one
            finalize_chunk(&mut state);

            // create the next chunk and trailer entry
            let next_index = state.chunks.len() as u32;
            let offset = state.next_offset;
            let chunk = new_chunk(next_index, sequence);
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
        postcard::to_slice(&event, &mut chunk.data[start..end]).map_err(|_| {
            RuntimeError::ReplayEncodeFailed {
                name: "event".to_string(),
            }
            .boxed()
        })?;
        let encoded = &chunk.data[start..end];
        chunk.header.checksum = update_checksum(chunk.header.checksum, encoded);
        chunk.header.byte_length = chunk.data.len() as u64;
        chunk.events.push(ReplayChunkEvent {
            offset: start as u64,
            length: encoded_len as u32,
        });
        chunk.header.event_count = chunk.events.len() as u32;
        chunk.header.sequence_end = sequence;

        // keep the trailer entry in sync
        update_trailer_entry(&mut state);

        Ok(sequence)
    }

    /// Read the next recorded event if available.
    pub fn next_event(&self) -> RuntimeResult<Option<ReplayEvent>> {
        // lock state for replay
        let mut state = self.state.lock();

        // advance to the next chunk if needed
        while state.read_chunk < state.chunks.len() {
            // read the next event in the current chunk
            let chunk = &state.chunks[state.read_chunk];
            if state.read_index < chunk.events.len() {
                let entry = chunk.events[state.read_index];
                let start = entry.offset as usize;
                let end = start + entry.length as usize;
                let encoded = &chunk.data[start..end];
                let event = decode_event(encoded).map_err(|_| {
                    RuntimeError::ReplayDecodeFailed {
                        name: "event".to_string(),
                    }
                    .boxed()
                })?;
                state.read_index += 1;
                return Ok(Some(event));
            }

            // move to the next chunk
            state.read_chunk += 1;
            state.read_index = 0;
        }

        Ok(None)
    }

    /// Record a checkpoint index entry and emit a checkpoint event.
    pub fn record_checkpoint(&self, mut checkpoint: ReplayCheckpointIndex) -> RuntimeResult<()> {
        // align the checkpoint with the next log sequence
        let sequence = self.next_sequence();
        checkpoint.sequence = sequence;

        // record the checkpoint event
        self.record_event(ReplayEvent::Checkpoint(CheckpointEvent {
            checkpoint_id: checkpoint.checkpoint_id,
            branch_id: checkpoint.branch_id,
        }))?;

        // update trailer index
        let mut state = self.state.lock();
        state.trailer.checkpoints.push(checkpoint);

        Ok(())
    }
}

fn new_chunk(index: u32, sequence_start: LogSequence) -> ReplayChunk {
    // seed a new chunk with default metadata
    ReplayChunk {
        header: ReplayChunkHeader {
            index,
            sequence_start,
            sequence_end: sequence_start,
            event_count: 0,
            byte_length: 0,
            checksum: FNV_OFFSET_BASIS_64,
        },
        data: Vec::new(),
        events: Vec::new(),
    }
}

fn should_rotate_chunk(
    chunk: Option<&ReplayChunk>,
    encoded_len: u64,
    max_events_per_chunk: usize,
    max_chunk_bytes: u64,
) -> bool {
    // rotate if there is no active chunk
    let Some(chunk) = chunk else {
        return true;
    };

    // rotate if we hit the event limit
    if chunk.events.len() >= max_events_per_chunk {
        return true;
    }

    // rotate if the chunk would exceed its byte limit
    chunk.header.byte_length.saturating_add(encoded_len) > max_chunk_bytes
}

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

fn update_checksum(current: u64, bytes: &[u8]) -> u64 {
    // compute the next checksum state
    let mut hash = current;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME_64);
    }
    hash
}

fn compute_log_hash(chunks: &[ReplayChunkIndex], checkpoints: &[ReplayCheckpointIndex]) -> u128 {
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

impl crate::replay::ReplayWriter for ReplayLog {
    fn record_event(&mut self, event: ReplayEvent) -> RuntimeResult<()> {
        ReplayLog::record_event(self, event)?;
        Ok(())
    }
}

impl crate::replay::ReplayReader for ReplayLog {
    fn next_event(&mut self) -> RuntimeResult<Option<ReplayEvent>> {
        ReplayLog::next_event(self)
    }
}
