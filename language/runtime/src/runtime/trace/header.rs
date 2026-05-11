use serde::{Deserialize, Serialize};

use crate::runtime::binding::BindingReplayPayload;
use crate::runtime::trace::TraceSequence;
use crate::runtime::world::{BranchId, CheckpointId, Revision};
use destack_workspace::{ExecutionMode, RandomMode, TimeMode};

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
    /// Time mode used while running.
    pub time_mode: TimeMode,
    /// Random mode used while running.
    pub random_mode: RandomMode,
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
    /// Runtime environment configuration.
    pub environment: EnvironmentConfig,
}

impl TraceHeader {
    /// Create a trace header with explicit configuration.
    pub fn new(environment: EnvironmentConfig) -> Self {
        Self {
            format_version: 1,
            build_hash: 0,
            target: String::new(),
            execution_mode: ExecutionMode::Fast,
            time_mode: TimeMode::Host,
            random_mode: RandomMode::Host,
            branch_id: BranchId::new(0),
            replay_payload: BindingReplayPayload::Results,
            binding_registry_hash: 0,
            max_events_per_chunk: 1024,
            max_chunk_bytes: 4 * 1024 * 1024,
            environment,
        }
    }
}

/// Environment configuration captured for trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Process arguments.
    pub argv: Vec<String>,
    /// Environment variables.
    pub env: Vec<(String, String)>,
    /// Current working directory.
    pub cwd: String,
    /// Timezone name or offset.
    pub timezone: Option<String>,
    /// Locale identifier.
    pub locale: Option<String>,
}

/// Header metadata for one trace chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceChunkHeader {
    /// Chunk index in the stream.
    pub index: u32,
    /// First sequence number in the chunk.
    pub sequence_start: TraceSequence,
    /// Last sequence number in the chunk.
    pub sequence_end: TraceSequence,
    /// Number of events stored in the chunk.
    pub event_count: u32,
    /// Byte length of the chunk payload.
    pub byte_length: u64,
    /// Checksum for the chunk payload.
    pub checksum: u64,
}

/// Chunk metadata for random access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceChunkIndex {
    /// Chunk index in the stream.
    pub index: u32,
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
    pub revision: Revision,
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
