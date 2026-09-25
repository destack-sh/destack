use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tspp_core::{FNV_OFFSET_BASIS_64, fnv1a_64_update};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::trace::{Trace, TraceTag};

use super::constants::{TRACE_ENTRY_LENGTH_BYTES, TRACE_ENTRY_TAG_BYTES};
use super::{TraceChunkHeader, TraceSequence};

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
    pub(super) fn sequence_end(&self) -> RuntimeResult<TraceSequence> {
        let entry_offset = self
            .header
            .entry_count
            .checked_sub(1)
            .ok_or_else(|| RuntimeError::trace_mismatch("empty_chunk".to_string()).boxed())?
            as u64;
        let sequence = self
            .header
            .sequence_start
            .get()
            .checked_add(entry_offset)
            .ok_or_else(|| RuntimeError::trace_mismatch("sequence".to_string()).boxed())?;

        Ok(TraceSequence::new(sequence))
    }

    /// Return whether this chunk contains one sequence.
    pub(super) fn contains_sequence(&self, sequence: TraceSequence) -> bool {
        if self.is_empty() {
            return false;
        }

        let sequence_start = self.header.sequence_start.get();
        let Some(relative_index) = sequence.get().checked_sub(sequence_start) else {
            return false;
        };

        relative_index < u64::from(self.header.entry_count)
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

    /// Decode one generic trace at one byte offset and return the next byte offset.
    pub(super) fn trace_at(&self, offset: usize) -> RuntimeResult<(Trace, usize)> {
        let (tag, encoded, end) = self.entry_at(offset)?;
        let trace = match tag {
            TraceTag::Mutation => Trace::Mutation(self.decode_entry(encoded, tag.name())?),
            TraceTag::Entrypoint => Trace::Entrypoint(self.decode_entry(encoded, tag.name())?),
            TraceTag::Binding => Trace::Binding(self.decode_entry(encoded, tag.name())?),
            TraceTag::Clock => Trace::Clock(self.decode_entry(encoded, tag.name())?),
            TraceTag::Random => Trace::Random(self.decode_entry(encoded, tag.name())?),
        };

        Ok((trace, end))
    }

    /// Decode one typed trace payload at one byte offset.
    pub(super) fn payload_at<T>(
        &self,
        offset: usize,
        expected_tag: TraceTag,
    ) -> RuntimeResult<(T, usize)>
    where
        T: DeserializeOwned,
    {
        let (tag, encoded, end) = self.entry_at(offset)?;
        if tag != expected_tag {
            return Err(RuntimeError::trace_mismatch(expected_tag.name().to_string()).boxed());
        }

        let payload = self.decode_entry(encoded, expected_tag.name())?;

        Ok((payload, end))
    }

    /// Return one encoded trace entry at one byte offset.
    fn entry_at(&self, offset: usize) -> RuntimeResult<(TraceTag, &[u8], usize)> {
        let length_end = offset + TRACE_ENTRY_LENGTH_BYTES;
        if length_end > self.bytes.len() {
            return Err(RuntimeError::trace_mismatch("chunk_length".to_string()).boxed());
        }

        let entry_length = self.entry_length(offset)?;
        let end = length_end + entry_length;
        if end > self.bytes.len() {
            return Err(RuntimeError::trace_mismatch("chunk_length".to_string()).boxed());
        }
        if entry_length < TRACE_ENTRY_TAG_BYTES {
            return Err(RuntimeError::trace_mismatch("trace_tag".to_string()).boxed());
        }

        let tag = TraceTag::from_byte(self.bytes[length_end])?;
        let payload_start = length_end + TRACE_ENTRY_TAG_BYTES;
        let encoded = &self.bytes[payload_start..end];

        Ok((tag, encoded, end))
    }

    /// Decode one trace payload from canonical bytes.
    fn decode_entry<T>(&self, encoded: &[u8], name: &str) -> RuntimeResult<T>
    where
        T: DeserializeOwned,
    {
        let payload = tspp_serde::from_slice(encoded)
            .map_err(|_| RuntimeError::trace_decode_failed(name.to_string()).boxed())?;

        Ok(payload)
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
