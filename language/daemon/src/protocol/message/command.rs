use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use destack_source::{FileType, TargetId};

use crate::command::{CommandPayload, CommandRevision, CommonCommandOptions};

use super::{BinaryPayload, DaemonMessageRecord, DiagnosticBatch, FileUpdateImage, RootHandleId};

/// Command request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Root handle.
    pub handle: RootHandleId,
    /// Revision selection for the command.
    #[serde(default)]
    pub revision: CommandRevision,
    /// Common command options.
    pub common: CommonCommandOptions,
    /// Command payload data.
    pub payload: CommandPayload,
}

/// Result of a command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResponse {
    /// Root handle.
    pub handle: RootHandleId,
    /// Whether the command succeeded.
    pub success: bool,
    /// Exit code for the command.
    pub exit_code: i32,
    /// Diagnostics emitted during execution.
    pub diagnostics: Vec<DiagnosticBatch>,
    /// File images for diagnostics rendering.
    pub files: Vec<FileUpdateImage>,
    /// Messages emitted during execution.
    pub messages: Vec<DaemonMessageRecord>,
    /// Output captured from the command.
    pub output: Vec<CommandOutputChunk>,
    /// Generated output files.
    pub outputs: Vec<CommandOutputFile>,
    /// Count of modules involved.
    pub module_count: usize,
    /// Count of profiles involved.
    pub profile_count: usize,
    /// Count of targets involved.
    pub target_count: usize,
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
    /// Root handle.
    pub handle: RootHandleId,
    /// Output stream kind.
    pub stream: OutputStream,
    /// Output bytes.
    pub bytes: Vec<u8>,
    /// Whether this output chunk is final.
    pub done: bool,
}

/// Generated output file produced by a command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandOutputFile {
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
