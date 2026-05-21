use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::host::binding::BindingReplayPayload;
use crate::world::trace::TraceSequence;
use crate::world::{BranchId, CheckpointId, RevisionId};
use destack_workspace::{ClockSource, Environment, ExecutionMode, RandomSource};

/// Current trace format version.
pub const TRACE_FORMAT_VERSION: u32 = 1;

/// Default maximum number of events in one trace chunk.
pub const TRACE_DEFAULT_MAX_EVENTS_PER_CHUNK: u32 = 1024;

/// Default maximum byte length of one trace chunk.
pub const TRACE_DEFAULT_MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

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
    pub replay_payload: BindingReplayPayload,
    /// Hash of the binding registry.
    pub binding_registry_hash: u128,
    /// Maximum number of events per chunk.
    pub max_events_per_chunk: u32,
    /// Maximum chunk size in bytes.
    pub max_chunk_bytes: u64,
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
            execution_mode: ExecutionMode::Fast,
            clock_source: ClockSource::Host,
            random_source: RandomSource::Host,
            branch_id: BranchId::new(0),
            replay_payload: BindingReplayPayload::Results,
            binding_registry_hash: 0,
            max_events_per_chunk: TRACE_DEFAULT_MAX_EVENTS_PER_CHUNK,
            max_chunk_bytes: TRACE_DEFAULT_MAX_CHUNK_BYTES,
            environment: environment.into(),
        }
    }
}

/// Header metadata for one trace chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceChunkHeader {
    /// First sequence number in the chunk.
    pub sequence_start: TraceSequence,
    /// Number of events stored in the chunk.
    pub event_count: u32,
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
    /// Index entries for external checkpoints.
    pub checkpoints: Vec<TraceCheckpointIndex>,
    /// Hash of the entire log stream.
    pub log_hash: u128,
}

/// Index entry referencing an external checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceCheckpointIndex {
    /// Checkpoint identifier.
    pub checkpoint_id: CheckpointId,
    /// Revision identifier anchored by this checkpoint.
    pub revision_id: RevisionId,
    /// Sequence number associated with the checkpoint.
    pub sequence: TraceSequence,
    /// Path to the checkpoint file.
    pub path: String,
    /// Hash of the checkpoint payload.
    pub hash: u128,
    /// Size of the checkpoint payload in bytes.
    pub size_bytes: u64,
}

impl TraceCheckpointIndex {
    /// Return the in-memory checkpoint path for one checkpoint identifier.
    pub fn memory_path(checkpoint_id: CheckpointId) -> String {
        format!("memory://checkpoint/{}", checkpoint_id.get())
    }
}
