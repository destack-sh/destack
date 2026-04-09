use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use destack_source::{FileType, TargetId};

use crate::command::{CommandPayload, CommonCommandOptions};

use super::{BinaryPayload, DaemonMessageRecord, DiagnosticBatch, FileSnapshot, WorkspaceHandleId};

/// Command request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Common command options.
    pub common: CommonCommandOptions,
    /// Command payload data.
    pub payload: CommandPayload,
}

/// Result of a command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Whether the command succeeded.
    pub success: bool,
    /// Exit code for the command.
    pub exit_code: i32,
    /// Diagnostics emitted during execution.
    pub diagnostics: Vec<DiagnosticBatch>,
    /// File snapshots for diagnostics rendering.
    pub files: Vec<FileSnapshot>,
    /// Messages emitted during execution.
    pub messages: Vec<DaemonMessageRecord>,
    /// Output captured from the command.
    pub output: Vec<CommandOutputChunk>,
    /// Output metadata produced.
    pub outputs: Vec<OutputInfo>,
    /// Count of modules involved.
    pub module_count: usize,
    /// Count of profiles involved.
    pub profile_count: usize,
    /// Count of targets involved.
    pub target_count: usize,
    /// Optional stats payload.
    pub stats: Option<CommandStats>,
    /// Command-specific payload for structured output.
    pub data: Option<BinaryPayload>,
}

/// Output chunk from command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandOutputChunk {
    /// Output stream kind.
    pub stream: OutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
}

/// Command output stream kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Notification for command output streaming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandOutputNotification {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Output stream kind.
    pub stream: OutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
    /// Whether this output chunk is final.
    pub done: bool,
}

/// Output metadata produced by commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputInfo {
    /// Output id.
    pub id: u64,
    /// Target id.
    pub target: TargetId,
    /// File type for the output.
    pub file_type: FileType,
    /// Output path for the generated file.
    pub path: PathBuf,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Optional content hash.
    pub content_hash: Option<u64>,
}

/// Command cache statistics payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandCacheStats {
    /// Cache hits from memory.
    pub hits_memory: u64,
    /// Cache misses.
    pub misses: u64,
    /// Cache writes to memory.
    pub writes_memory: u64,
    /// Cache errors.
    pub errors: u64,
    /// Cache hit rate across all cache kinds.
    pub hit_rate: f32,
}

/// Command statistics payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandStats {
    /// Elapsed time in milliseconds.
    pub elapsed_ms: u64,
    /// Number of artifact attempts started.
    pub artifacts_started: u64,
    /// Number of artifact attempts completed.
    pub artifacts_completed: u64,
    /// Number of artifact attempts failed.
    pub artifacts_failed: u64,
    /// Number of artifact attempts that yielded requirements.
    pub artifacts_yielded: u64,
    /// Number of modules processed.
    pub modules_processed: u64,
    /// Number of lines processed.
    pub lines_processed: u64,
    /// Number of slow artifact attempts detected.
    pub artifacts_slow: u64,
    /// Cache statistics when available.
    pub cache: Option<CommandCacheStats>,
    /// Timing tag statistics when available.
    pub timings: Option<Vec<CommandTimingTagStats>>,
}

/// Timing tag statistics for command payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommandTimingTagStats {
    /// Timing tag name.
    pub name: String,
    /// Total time spent in this tag (milliseconds).
    pub duration_ms: u64,
    /// Number of samples recorded.
    pub sample_count: u64,
}
