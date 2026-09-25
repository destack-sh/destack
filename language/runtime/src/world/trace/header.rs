use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_repository::{Environment, ExecutionMode};

use crate::binding::ReplayPayload;
use crate::world::random::RandomSource;
use crate::world::time::ClockSource;
use crate::world::trace::TraceSequence;
use crate::world::{BranchId, ImageId, RevisionId};

use super::{
    TRACE_DEFAULT_MAX_CHUNK_SIZE_BYTES, TRACE_DEFAULT_MAX_ENTRIES_PER_CHUNK, TRACE_FORMAT_VERSION,
};

/// Trace header describing the execution environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceHeader {
    /// Trace format version.
    pub format_version: u32,
    /// Build hash for runtime compatibility.
    pub build_hash: u128,
    /// Target triple or platform descriptor.
    pub target: String,
    /// Execution mode used while running.
    pub execution_mode: ExecutionMode,
    /// Clock source used while running.
    pub clock_source: ClockSource,
    /// Random source used while running.
    pub random_source: RandomSource,
    /// Branch identifier for this replay stream.
    pub branch_id: BranchId,
    /// Trace payload selection for the log.
    pub replay_payload: ReplayPayload,
    /// Hash of the binding table.
    pub binding_table_hash: u128,
    /// Maximum number of entries per chunk.
    pub max_entries_per_chunk: u32,
    /// Maximum chunk size in bytes.
    pub max_chunk_size_bytes: u64,
    /// Runtime environment.
    pub environment: Arc<Environment>,
}

impl TraceHeader {
    /// Create a trace header with explicit configuration.
    pub fn new(environment: impl Into<Arc<Environment>>) -> Self {
        Self {
            format_version: TRACE_FORMAT_VERSION,
            build_hash: 0,
            target: String::new(),
            execution_mode: ExecutionMode::Strict,
            clock_source: ClockSource::Runtime,
            random_source: RandomSource::Deterministic,
            branch_id: BranchId::new(0),
            replay_payload: ReplayPayload::Results,
            binding_table_hash: 0,
            max_entries_per_chunk: TRACE_DEFAULT_MAX_ENTRIES_PER_CHUNK,
            max_chunk_size_bytes: TRACE_DEFAULT_MAX_CHUNK_SIZE_BYTES,
            environment: environment.into(),
        }
    }
}

/// Header metadata for one trace chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceChunkHeader {
    /// First sequence number in the chunk.
    pub sequence_start: TraceSequence,
    /// Number of entries stored in the chunk.
    pub entry_count: u32,
}

/// Chunk metadata for random access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceChunkIndex {
    /// Byte offset of the chunk in the log.
    pub offset: u64,
    /// Byte length of the chunk payload.
    pub length: u64,
    /// Chunk checksum.
    pub checksum: u64,
}

/// Trailer metadata for a trace log.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceTrailer {
    /// Index entries for chunks in the log.
    pub chunks: Vec<TraceChunkIndex>,
    /// Index entries for external Images.
    pub images: Vec<TraceImageIndex>,
    /// Hash of the entire log stream.
    pub log_hash: u128,
}

/// Index entry referencing one external World Image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceImageIndex {
    /// Image identifier.
    pub image_id: ImageId,
    /// Revision identifier anchored by this Image.
    pub revision_id: RevisionId,
    /// Sequence number associated with the Image.
    pub sequence: TraceSequence,
    /// Path to the Image file.
    pub path: String,
    /// Hash of the Image payload.
    pub hash: u128,
    /// Size of the Image payload in bytes.
    pub size_bytes: u64,
}

impl TraceImageIndex {
    /// Return the in-memory path for one Image identifier.
    pub fn memory_path(image_id: ImageId) -> String {
        format!("memory://image/{}", image_id.get())
    }
}
