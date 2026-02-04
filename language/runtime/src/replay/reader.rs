use std::sync::Arc;

use parking_lot::Mutex;

use super::{LogSequence, ReplayEvent, ReplayTrailer};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::replay::codec::decode_event;
use crate::replay::log::{ReplayChunk, ReplayLogState, compute_chunk_checksum, compute_log_hash};

/// Replay reader interface for deterministic replay.
pub trait ReplayReader {
    /// Read the next event in the replay log.
    fn next_event(&mut self) -> RuntimeResult<Option<ReplayEvent>>;
}

/// Replay reader cursor state.
#[derive(Debug, Clone, Copy)]
struct ReplayCursor {
    /// Current chunk index.
    read_chunk: usize,
    /// Current event index inside the chunk.
    read_index: usize,
    /// Next expected sequence number.
    next_sequence: LogSequence,
    /// Last chunk index that was validated.
    validated_chunk: Option<usize>,
}

/// Replay reader for a shared replay log.
#[derive(Debug)]
pub struct ReplayLogReader {
    /// Shared replay log state.
    state: Arc<Mutex<ReplayLogState>>,
    /// Cursor state for this reader.
    cursor: Mutex<ReplayCursor>,
    /// Optional trailer hash validation.
    trailer_hash: Option<u128>,
    /// Cached computed log hash.
    log_hash: u128,
}

impl ReplayLogReader {
    /// Create a replay reader for the given log state.
    pub(super) fn new(state: Arc<Mutex<ReplayLogState>>) -> Self {
        let (next_sequence, trailer_hash, log_hash) = {
            let state = state.lock();
            let next_sequence = state
                .chunks
                .first()
                .map(|chunk| chunk.header.sequence_start)
                .unwrap_or_else(|| LogSequence::new(0));
            let trailer = state.trailer();
            let log_hash = compute_log_hash(&trailer.chunks, &trailer.checkpoints);
            let trailer_hash = if trailer.log_hash == 0 {
                None
            } else {
                Some(trailer.log_hash)
            };
            (next_sequence, trailer_hash, log_hash)
        };

        Self {
            state,
            cursor: Mutex::new(ReplayCursor {
                read_chunk: 0,
                read_index: 0,
                next_sequence,
                validated_chunk: None,
            }),
            trailer_hash,
            log_hash,
        }
    }

    /// Read the next recorded event if available.
    pub fn next_event(&self) -> RuntimeResult<Option<ReplayEvent>> {
        if let Some(expected) = self.trailer_hash
            && expected != self.log_hash
        {
            return Err(RuntimeError::ReplayMismatch {
                name: "log_hash".to_string(),
            }
            .boxed());
        }

        // lock the cursor for this reader
        let mut cursor = self.cursor.lock();
        let state = self.state.lock();

        // advance to the next chunk if needed
        while cursor.read_chunk < state.chunks.len() {
            // read the next event in the current chunk
            let chunk_index = cursor.read_chunk;
            let chunk = &state.chunks[chunk_index];
            if cursor.validated_chunk != Some(chunk_index) {
                validate_chunk(chunk, state.trailer())?;
                cursor.validated_chunk = Some(chunk_index);
            }
            if cursor.read_index < chunk.events.len() {
                let sequence = cursor.next_sequence;
                if cursor.read_index == 0 && chunk.header.sequence_start != sequence {
                    return Err(RuntimeError::ReplayMismatch {
                        name: "sequence".to_string(),
                    }
                    .boxed());
                }

                let entry = chunk.events[cursor.read_index];
                let start = entry.offset as usize;
                let end = start + entry.length as usize;
                let encoded = &chunk.data[start..end];
                let event = decode_event(encoded).map_err(|_| {
                    RuntimeError::ReplayDecodeFailed {
                        name: "event".to_string(),
                    }
                    .boxed()
                })?;
                if cursor.read_index + 1 == chunk.events.len()
                    && chunk.header.sequence_end != sequence
                {
                    return Err(RuntimeError::ReplayMismatch {
                        name: "sequence".to_string(),
                    }
                    .boxed());
                }
                cursor.read_index += 1;
                cursor.next_sequence = sequence.next();
                return Ok(Some(event));
            }

            // move to the next chunk
            cursor.read_chunk += 1;
            cursor.read_index = 0;
        }

        Ok(None)
    }
}

impl ReplayReader for ReplayLogReader {
    fn next_event(&mut self) -> RuntimeResult<Option<ReplayEvent>> {
        ReplayLogReader::next_event(self)
    }
}

/// Validate chunk integrity against stored metadata.
fn validate_chunk(chunk: &ReplayChunk, trailer: &ReplayTrailer) -> RuntimeResult<()> {
    if chunk.header.event_count as usize != chunk.events.len() {
        return Err(RuntimeError::ReplayMismatch {
            name: "chunk_events".to_string(),
        }
        .boxed());
    }
    if chunk.header.byte_length as usize != chunk.data.len() {
        return Err(RuntimeError::ReplayMismatch {
            name: "chunk_length".to_string(),
        }
        .boxed());
    }
    let checksum = compute_chunk_checksum(&chunk.data);
    if checksum != chunk.header.checksum {
        return Err(RuntimeError::ReplayMismatch {
            name: "chunk_checksum".to_string(),
        }
        .boxed());
    }
    if let Some(entry) = trailer
        .chunks
        .iter()
        .find(|entry| entry.index == chunk.header.index)
        && entry.checksum != chunk.header.checksum
    {
        return Err(RuntimeError::ReplayMismatch {
            name: "chunk_index".to_string(),
        }
        .boxed());
    }
    Ok(())
}
