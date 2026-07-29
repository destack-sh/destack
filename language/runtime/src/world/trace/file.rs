use destack_core::{FNV_OFFSET_BASIS_128, fnv1a_128_update};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};

use super::chunk::TraceChunk;
use super::{TraceCheckpointIndex, TraceChunkIndex, TraceHeader, TraceSequence, TraceTrailer};

/// Flat trace file captured for image storage and durable replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceFile {
    /// Trace header metadata.
    header: TraceHeader,
    /// Trace chunks in stream order.
    pub(super) chunks: Vec<TraceChunk>,
    /// Trace trailer metadata.
    trailer: TraceTrailer,
}

impl TraceFile {
    /// Create one flat trace file from ordered chunks and checkpoints.
    pub(super) fn new(
        header: TraceHeader,
        chunks: Vec<TraceChunk>,
        checkpoints: Vec<TraceCheckpointIndex>,
    ) -> Self {
        let trailer = TraceTrailer::build(&chunks, checkpoints);

        Self {
            header,
            chunks,
            trailer,
        }
    }

    /// Return the trace header.
    pub(super) fn header(&self) -> TraceHeader {
        self.header.clone()
    }

    /// Return the next sequence after the last stored entry.
    pub(super) fn next_sequence(&self) -> RuntimeResult<TraceSequence> {
        let sequence = self
            .chunks
            .last()
            .filter(|chunk| !chunk.is_empty())
            .map(|chunk| chunk.sequence_end().and_then(TraceSequence::next))
            .transpose()?
            .unwrap_or(TraceSequence::new(0));

        Ok(sequence)
    }

    /// Validate this file against its stored trailer.
    pub(super) fn validate(&self) -> RuntimeResult<()> {
        let expected = TraceTrailer::build(&self.chunks, self.trailer.checkpoints.clone());

        // require ordered checkpoints for incremental inserts
        if self
            .trailer
            .checkpoints
            .windows(2)
            .any(|pair| pair[0].sequence > pair[1].sequence)
        {
            return Err(RuntimeError::trace_mismatch("checkpoints".to_string()).boxed());
        }

        // verify chunk index entries
        if self.trailer.chunks != expected.chunks {
            return Err(RuntimeError::trace_mismatch("chunks".to_string()).boxed());
        }

        // verify trailer hash
        if self.trailer.log_hash != expected.log_hash {
            return Err(RuntimeError::trace_mismatch("log_hash".to_string()).boxed());
        }

        Ok(())
    }

    /// Split this file into its constituent parts.
    pub(super) fn into_parts(self) -> (TraceHeader, Vec<TraceChunk>, TraceTrailer) {
        (self.header, self.chunks, self.trailer)
    }
}

impl TraceTrailer {
    /// Build one trailer from ordered chunks and checkpoint metadata.
    fn build(chunks: &[TraceChunk], checkpoints: Vec<TraceCheckpointIndex>) -> Self {
        // build chunk index entries in file order
        let mut offset = 0u64;
        let chunks = chunks
            .iter()
            .map(|chunk| {
                let length = chunk.byte_length();
                let entry = TraceChunkIndex {
                    offset,
                    length,
                    checksum: chunk.payload_checksum(),
                };
                offset += length;

                entry
            })
            .collect::<Vec<_>>();

        // bind chunk and checkpoint indexes
        let log_hash = Self::hash(&chunks, &checkpoints);

        Self {
            chunks,
            checkpoints,
            log_hash,
        }
    }

    /// Compute the stable hash for one trailer index.
    fn hash(chunks: &[TraceChunkIndex], checkpoints: &[TraceCheckpointIndex]) -> u128 {
        let mut hash = FNV_OFFSET_BASIS_128;
        for chunk in chunks {
            hash = fnv1a_128_update(hash, &chunk.offset.to_le_bytes());
            hash = fnv1a_128_update(hash, &chunk.length.to_le_bytes());
            hash = fnv1a_128_update(hash, &chunk.checksum.to_le_bytes());
        }

        for checkpoint in checkpoints {
            hash = fnv1a_128_update(hash, &checkpoint.checkpoint_id.get().to_le_bytes());
            hash = fnv1a_128_update(hash, &checkpoint.revision_id.get().to_le_bytes());
            hash = fnv1a_128_update(hash, &checkpoint.sequence.get().to_le_bytes());
            hash = fnv1a_128_update(hash, &checkpoint.size_bytes.to_le_bytes());
            hash = fnv1a_128_update(hash, &checkpoint.hash.to_le_bytes());
            hash = fnv1a_128_update(hash, checkpoint.path.as_bytes());
        }

        hash
    }
}
