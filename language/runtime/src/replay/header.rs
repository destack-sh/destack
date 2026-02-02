use serde::{Deserialize, Serialize};

use crate::platform::bindings::ExecutionMode;
use crate::replay::{BranchId, CheckpointId, LogSequence};

/// Replay log header describing the execution environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    /// Replay format version.
    pub format_version: u32,
    /// Build hash for runtime compatibility.
    pub build_hash: String,
    /// Target triple or platform descriptor.
    pub target: String,
    /// Profile key used to resolve configuration.
    pub profile_key: Option<String>,
    /// Hash of the resolved profile configuration.
    pub profile_hash: u128,
    /// Execution mode used while running.
    pub execution_mode: ExecutionMode,
    /// Hash of the binding registry.
    pub binding_registry_hash: u128,
    /// Hash of the effect registry.
    pub effect_registry_hash: u128,
    /// Maximum number of events per chunk.
    pub max_events_per_chunk: u32,
    /// Maximum chunk size in bytes.
    pub max_chunk_bytes: u64,
    /// Runtime environment configuration.
    pub environment: EnvironmentConfig,
}

impl ReplayHeader {
    /// Create a replay header with explicit configuration.
    pub fn new(environment: EnvironmentConfig) -> Self {
        Self {
            format_version: 1,
            build_hash: String::new(),
            target: String::new(),
            profile_key: None,
            profile_hash: 0,
            execution_mode: ExecutionMode::Fast,
            binding_registry_hash: 0,
            effect_registry_hash: 0,
            max_events_per_chunk: 1024,
            max_chunk_bytes: 4 * 1024 * 1024,
            environment,
        }
    }
}

impl Default for ReplayHeader {
    fn default() -> Self {
        Self::new(EnvironmentConfig::default())
    }
}

/// Environment configuration captured for replay.
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

/// Header metadata for a replay log chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayChunkHeader {
    /// Chunk index in the stream.
    pub index: u32,
    /// First sequence number in the chunk.
    pub sequence_start: LogSequence,
    /// Last sequence number in the chunk.
    pub sequence_end: LogSequence,
    /// Number of events stored in the chunk.
    pub event_count: u32,
    /// Byte length of the chunk payload.
    pub byte_length: u64,
    /// Checksum for the chunk payload.
    pub checksum: u64,
}

/// Chunk metadata for random access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayChunkIndex {
    /// Chunk index in the stream.
    pub index: u32,
    /// Byte offset of the chunk in the log.
    pub offset: u64,
    /// Byte length of the chunk payload.
    pub length: u64,
    /// Chunk checksum.
    pub checksum: u64,
}

/// Trailer metadata for a replay log.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplayTrailer {
    /// Index entries for chunks in the log.
    pub chunks: Vec<ReplayChunkIndex>,
    /// Index entries for external checkpoints.
    pub checkpoints: Vec<ReplayCheckpointIndex>,
    /// Hash of the entire log stream.
    pub log_hash: u128,
}

/// Index entry referencing an external checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayCheckpointIndex {
    /// Checkpoint identifier.
    pub checkpoint_id: CheckpointId,
    /// Branch identifier for this checkpoint.
    pub branch_id: BranchId,
    /// Sequence number associated with the checkpoint.
    pub sequence: LogSequence,
    /// Path to the checkpoint file.
    pub path: String,
    /// Hash of the checkpoint payload.
    pub hash: u128,
    /// Size of the checkpoint payload in bytes.
    pub size_bytes: u64,
}
