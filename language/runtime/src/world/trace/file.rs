use serde::{Deserialize, Serialize};
use tspp_core::{FNV_OFFSET_BASIS_128, fnv1a_128_update};

use crate::diagnostic::{RuntimeError, RuntimeResult};

use super::chunk::TraceChunk;
use super::{TraceChunkIndex, TraceHeader, TraceImageIndex, TraceSequence, TraceTrailer};

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
    /// Create one flat trace file from ordered chunks and Images.
    pub(super) fn new(
        header: TraceHeader,
        chunks: Vec<TraceChunk>,
        images: Vec<TraceImageIndex>,
    ) -> Self {
        let trailer = TraceTrailer::build(&chunks, images);

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
        let expected = TraceTrailer::build(&self.chunks, self.trailer.images.clone());

        // require ordered images for incremental inserts
        if self
            .trailer
            .images
            .windows(2)
            .any(|pair| pair[0].sequence > pair[1].sequence)
        {
            return Err(RuntimeError::trace_mismatch("images".to_string()).boxed());
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
    /// Build one trailer from ordered chunks and Image metadata.
    fn build(chunks: &[TraceChunk], images: Vec<TraceImageIndex>) -> Self {
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

        // bind chunk and image indexes
        let log_hash = Self::hash(&chunks, &images);

        Self {
            chunks,
            images,
            log_hash,
        }
    }

    /// Compute the stable hash for one trailer index.
    fn hash(chunks: &[TraceChunkIndex], images: &[TraceImageIndex]) -> u128 {
        let mut hash = FNV_OFFSET_BASIS_128;
        for chunk in chunks {
            hash = fnv1a_128_update(hash, &chunk.offset.to_le_bytes());
            hash = fnv1a_128_update(hash, &chunk.length.to_le_bytes());
            hash = fnv1a_128_update(hash, &chunk.checksum.to_le_bytes());
        }

        for image in images {
            hash = fnv1a_128_update(hash, &image.image_id.get().to_le_bytes());
            hash = fnv1a_128_update(hash, &image.revision_id.get().to_le_bytes());
            hash = fnv1a_128_update(hash, &image.sequence.get().to_le_bytes());
            hash = fnv1a_128_update(hash, &image.size_bytes.to_le_bytes());
            hash = fnv1a_128_update(hash, &image.hash.to_le_bytes());
            hash = fnv1a_128_update(hash, image.path.as_bytes());
        }

        hash
    }
}
