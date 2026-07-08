use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::trace::Trace;

use super::{TraceChunkHeader, TraceSequence};
use destack_core::{FNV_OFFSET_BASIS_64, fnv1a_64_update};
use serde::{Deserialize, Serialize};

/// Byte length of one trace entry length prefix.
pub(super) const TRACE_ENTRY_LENGTH_BYTES: usize = std::mem::size_of::<u32>();

/// Trace chunk payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceChunk {
    /// Chunk header metadata.
    pub(super) header: TraceChunkHeader,
    /// Chunk payload bytes.
    pub(super) bytes: Vec<u8>,
}

impl TraceChunk {
    /// Create one empty chunk with initialized metadata.
    pub(super) fn new(sequence_start: TraceSequence) -> Self {
        Self {
            header: TraceChunkHeader {
                sequence_start,
                entry_count: 0,
            },
            bytes: Vec::new(),
        }
    }

    /// Return whether this chunk should rotate before appending one entry.
    pub(super) fn should_rotate_for_entry(
        &self,
        encoded_len: u64,
        max_entries_per_chunk: usize,
        max_chunk_size_bytes: u64,
    ) -> bool {
        // rotate when entry count would exceed the chunk limit
        if self.header.entry_count as usize >= max_entries_per_chunk {
            return true;
        }

        // rotate when byte length would exceed the chunk limit
        self.byte_length().saturating_add(encoded_len) > max_chunk_size_bytes
    }

    /// Return whether this chunk stores no entries.
    pub(super) fn is_empty(&self) -> bool {
        self.header.entry_count == 0
    }

    /// Compute this chunk payload checksum from scratch.
    pub(super) fn payload_checksum(&self) -> u64 {
        fnv1a_64_update(FNV_OFFSET_BASIS_64, &self.bytes)
    }

    /// Return this chunk payload length in bytes.
    pub(super) fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Return the last sequence number stored in this chunk.
    pub(super) fn sequence_end(&self) -> TraceSequence {
        debug_assert!(!self.is_empty());
        let entry_offset = (self.header.entry_count - 1) as u64;

        TraceSequence::new(self.header.sequence_start.get() + entry_offset)
    }

    /// Return whether this chunk contains one sequence.
    pub(super) fn contains_sequence(&self, sequence: TraceSequence) -> bool {
        if self.is_empty() {
            return false;
        }

        let sequence_value = sequence.get();
        let sequence_start = self.header.sequence_start.get();
        let sequence_end = self.sequence_end().get();

        sequence_value >= sequence_start && sequence_value <= sequence_end
    }

    /// Return the byte offset for one entry index.
    pub(super) fn entry_offset(&self, entry_index: usize) -> RuntimeResult<usize> {
        let mut offset = 0usize;

        // scan length-prefixed entries up to the requested index
        for _ in 0..entry_index {
            let length_end = offset + TRACE_ENTRY_LENGTH_BYTES;
            if length_end > self.bytes.len() {
                return Err(RuntimeError::trace_mismatch("offset".to_string()).boxed());
            }

            let entry_length = self.entry_length(offset)?;
            offset = length_end
                .checked_add(entry_length)
                .ok_or_else(|| RuntimeError::trace_mismatch("offset".to_string()).boxed())?;
        }

        Ok(offset)
    }

    /// Return the entry index and byte offset for one sequence.
    pub(super) fn sequence_offset(&self, sequence: TraceSequence) -> RuntimeResult<(usize, usize)> {
        let relative_index = sequence
            .get()
            .checked_sub(self.header.sequence_start.get())
            .ok_or_else(|| RuntimeError::trace_mismatch("sequence".to_string()).boxed())?
            as usize;
        let read_offset = self.entry_offset(relative_index)?;

        Ok((relative_index, read_offset))
    }

    /// Validate chunk integrity against stored metadata.
    pub(super) fn validate_entries(&self) -> RuntimeResult<()> {
        if self.entry_offset(self.header.entry_count as usize)? != self.bytes.len() {
            return Err(RuntimeError::trace_mismatch("chunk_offsets".to_string()).boxed());
        }

        Ok(())
    }

    /// Decode one trace at one byte offset and return the next byte offset.
    pub(super) fn trace_at(&self, offset: usize) -> RuntimeResult<(Trace, usize)> {
        let length_end = offset + TRACE_ENTRY_LENGTH_BYTES;
        if length_end > self.bytes.len() {
            return Err(RuntimeError::trace_mismatch("chunk_length".to_string()).boxed());
        }

        let entry_length = self.entry_length(offset)?;
        let end = length_end + entry_length;
        if end > self.bytes.len() {
            return Err(RuntimeError::trace_mismatch("chunk_length".to_string()).boxed());
        }

        let encoded = &self.bytes[length_end..end];
        let trace = destack_serde::from_slice(encoded)
            .map_err(|_| RuntimeError::trace_decode_failed("entry".to_string()).boxed())?;

        Ok((trace, end))
    }

    /// Return one length-prefixed entry length.
    fn entry_length(&self, offset: usize) -> RuntimeResult<usize> {
        let length_end = offset + TRACE_ENTRY_LENGTH_BYTES;
        if length_end > self.bytes.len() {
            return Err(RuntimeError::trace_mismatch("offset".to_string()).boxed());
        }

        let mut entry_length = [0u8; TRACE_ENTRY_LENGTH_BYTES];
        entry_length.copy_from_slice(&self.bytes[offset..length_end]);

        Ok(u32::from_le_bytes(entry_length) as usize)
    }
}
