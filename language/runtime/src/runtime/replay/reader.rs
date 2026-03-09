use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::{TraceEvent, TraceSequence, TraceTrailer};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::codec::decode_event;
use crate::runtime::replay::log::{TraceLogImage, compute_log_hash};
use crate::runtime::world::BranchId;

use super::chunk::TraceSegment;

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
    /// Shared trace-log image.
    state: Arc<Mutex<TraceLogImage>>,
    /// Cursor state for this reader.
    cursor: Mutex<TraceReadCursor>,
}

impl TraceCursor {
    /// Create a trace cursor for the given trace-log image.
    pub(super) fn new(state: Arc<Mutex<TraceLogImage>>) -> Self {
        let next_sequence = {
            let state = state.lock();
            state
                .segment(0)
                .map(|segment| segment.header.sequence_start)
                .unwrap_or_else(|| TraceSequence::new(0))
        };

        Self {
            state,
            cursor: Mutex::new(TraceReadCursor {
                read_chunk: 0,
                read_index: 0,
                read_offset: 0,
                next_sequence,
                validated_chunk: None,
            }),
        }
    }

    /// Read the next recorded event if available.
    pub(crate) fn next_event(&self) -> RuntimeResult<Option<TraceEvent>> {
        let state = self.state.lock();
        let trailer = state.trailer();
        let log_hash = compute_log_hash(&trailer.segments, &trailer.checkpoints);
        if trailer.log_hash != 0 && trailer.log_hash != log_hash {
            return Err(RuntimeError::TraceMismatch {
                name: "log_hash".to_string(),
            }
            .boxed());
        }

        // lock the cursor for this reader
        let mut cursor = self.cursor.lock();

        // advance to the next segment if needed
        while cursor.read_chunk < state.segment_count() {
            // read the next event in the current segment
            let chunk_index = cursor.read_chunk;
            let chunk = state
                .segment(chunk_index)
                .expect("trace log image must contain the requested segment")
                .as_ref();
            if cursor.validated_chunk != Some(chunk_index) {
                validate_segment(chunk, trailer)?;
                cursor.validated_chunk = Some(chunk_index);
            }
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
                let event = decode_event(encoded).map_err(|_| {
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

            // move to the next segment
            cursor.read_chunk += 1;
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
        Ok(())
    }

    /// Return the next sequence visible through this cursor.
    pub(crate) fn tell(&self) -> TraceSequence {
        self.cursor.lock().next_sequence
    }

    /// Seek this cursor to one sequence boundary.
    pub(crate) fn seek_sequence(&self, sequence: TraceSequence) -> RuntimeResult<()> {
        let state = self.state.lock();
        if sequence.get() > state.next_sequence().get() {
            return Err(RuntimeError::TraceMismatch {
                name: "sequence".to_string(),
            }
            .boxed());
        }

        let mut next_sequence = state
            .segment(0)
            .map(|segment| segment.header.sequence_start)
            .unwrap_or_else(|| TraceSequence::new(0));
        let mut read_chunk = 0usize;
        let mut read_index = 0usize;
        let mut read_offset = 0usize;
        let mut validated_chunk = None;

        while read_chunk < state.segment_count() {
            let segment = state
                .segment(read_chunk)
                .expect("trace log image must contain the requested segment");
            let segment = segment.as_ref();
            if segment.event_lengths.is_empty() {
                break;
            }

            for (event_index, event_length) in segment.event_lengths.iter().enumerate() {
                if next_sequence == sequence {
                    let mut cursor = self.cursor.lock();
                    *cursor = TraceReadCursor {
                        read_chunk,
                        read_index,
                        read_offset,
                        next_sequence,
                        validated_chunk,
                    };
                    return Ok(());
                }

                read_index = event_index + 1;
                read_offset += *event_length as usize;
                next_sequence = next_sequence.next();
            }

            read_chunk += 1;
            read_index = 0;
            read_offset = 0;
            validated_chunk = Some(read_chunk.saturating_sub(1));
        }

        let mut cursor = self.cursor.lock();
        *cursor = TraceReadCursor {
            read_chunk,
            read_index,
            read_offset,
            next_sequence,
            validated_chunk,
        };

        Ok(())
    }
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

    let checksum = segment.payload_checksum();
    if checksum != segment.header.checksum {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_checksum".to_string(),
        }
        .boxed());
    }
    if let Some(entry) = trailer
        .segments
        .iter()
        .find(|entry| entry.index == segment.header.index)
        && entry.checksum != segment.header.checksum
    {
        return Err(RuntimeError::TraceMismatch {
            name: "chunk_index".to_string(),
        }
        .boxed());
    }
    Ok(())
}
