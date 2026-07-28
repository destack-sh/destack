use std::path::PathBuf;

use destack_artifact::ArtifactReference;
use destack_repository::Revision;
use destack_serde::Reflect;
use destack_source::{Content, ContentId, FileId, TextRange};
use serde::{Deserialize, Serialize};

use super::super::handshake::HandshakeRequest;
use super::{
    CloseRootRequest, FileOperationRequest, OpenRootRequest, QueryRequestPayload,
    ReloadRootRequest, RequestId, RootId, SourceUpdateRequest, WatchNextRequest, WatchStartRequest,
    WatchStopRequest,
};
use crate::{
    BenchInput, BuildInput, CacheInput, CheckInput, CleanInput, DocInput, DoctorInput,
    ExportRequest, FormatInput, InfoInput, QueryInput, RewriteInput, RunInput, SettingsInput,
    TargetsInput, TaskInput, TestInput,
};

/// Requests accepted by the workspace protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum WorkspaceRequest {
    /// Negotiate protocol version and capabilities.
    Handshake(HandshakeRequest),
    /// Check server liveness.
    Ping,
    /// Cancel an in flight request.
    Cancel { id: RequestId },
    /// Request server shutdown.
    Shutdown,
    /// Open or register a root.
    OpenRoot(OpenRootRequest),
    /// Close a root handle.
    CloseRoot(CloseRootRequest),
    /// Reload a root.
    ReloadRoot(ReloadRootRequest),
    /// Read the current semantic revision.
    ReadRevision {
        /// Root handle.
        handle: RootId,
    },
    /// Apply a file operation to a root.
    ApplyFileOperation(FileOperationRequest),
    /// Apply a source update to a root.
    ApplySourceUpdate(SourceUpdateRequest),
    /// Check whether one source file is open.
    IsFileOpen {
        /// Root handle.
        handle: RootId,
        /// Source path.
        path: PathBuf,
    },
    /// Format one source file or selected text range.
    FormatFile {
        /// Root handle.
        handle: RootId,
        /// Source path.
        path: PathBuf,
        /// Optional UTF-16 source range.
        range: Option<TextRange>,
    },
    /// Read source files from one exact revision.
    ReadFiles {
        /// Root handle.
        handle: RootId,
        /// Exact semantic revision.
        revision: Revision,
        /// Source file ids.
        file_ids: Vec<FileId>,
    },
    /// Start watching a root.
    StartWatch(WatchStartRequest),
    /// Receive and apply the next watch batch.
    NextWatchBatch(WatchNextRequest),
    /// Stop watching a root.
    StopWatch(WatchStopRequest),
    /// Check source state.
    Check {
        /// Root handle.
        handle: RootId,
        /// Check input.
        input: CheckInput,
    },
    /// Format source files or content.
    Format {
        /// Root handle.
        handle: RootId,
        /// Format input.
        input: FormatInput,
    },
    /// Query source files with a structural pattern.
    Query {
        /// Root handle.
        handle: RootId,
        /// Query input.
        input: QueryInput,
    },
    /// Rewrite source files with a structural pattern.
    Rewrite {
        /// Root handle.
        handle: RootId,
        /// Rewrite input.
        input: RewriteInput,
    },
    /// Build target artifacts.
    Build {
        /// Root handle.
        handle: RootId,
        /// Build input.
        input: BuildInput,
    },
    /// Run a workspace target.
    Run {
        /// Root handle.
        handle: RootId,
        /// Run input.
        input: RunInput,
    },
    /// Run workspace tests.
    Test {
        /// Root handle.
        handle: RootId,
        /// Test input.
        input: TestInput,
    },
    /// Generate documentation.
    Doc {
        /// Root handle.
        handle: RootId,
        /// Documentation input.
        input: DocInput,
    },
    /// Run benchmarks.
    Bench {
        /// Root handle.
        handle: RootId,
        /// Benchmark input.
        input: BenchInput,
    },
    /// Return workspace information.
    Info {
        /// Root handle.
        handle: RootId,
        /// Information input.
        input: InfoInput,
    },
    /// Return configured targets.
    Targets {
        /// Root handle.
        handle: RootId,
        /// Targets input.
        input: TargetsInput,
    },
    /// Return cache locations.
    Cache {
        /// Root handle.
        handle: RootId,
        /// Cache input.
        input: CacheInput,
    },
    /// Return resolved settings.
    Settings {
        /// Root handle.
        handle: RootId,
        /// Settings input.
        input: SettingsInput,
    },
    /// Return workspace health information.
    Doctor {
        /// Root handle.
        handle: RootId,
        /// Doctor input.
        input: DoctorInput,
    },
    /// Run workspace tasks.
    Task {
        /// Root handle.
        handle: RootId,
        /// Task input.
        input: TaskInput,
    },
    /// Clean generated state.
    Clean {
        /// Root handle.
        handle: RootId,
        /// Clean input.
        input: CleanInput,
    },
    /// Return one artifact payload.
    Artifact {
        /// Root handle.
        handle: RootId,
        /// Artifact reference.
        artifact: ArtifactReference,
    },
    /// Store one content payload.
    Store {
        /// Root handle.
        handle: RootId,
        /// Content payload.
        content: Content,
    },
    /// Load one content payload.
    Load {
        /// Root handle.
        handle: RootId,
        /// Content id.
        content: ContentId,
    },
    /// Materialize derived outputs.
    Export {
        /// Root handle.
        handle: RootId,
        /// Export request.
        request: ExportRequest,
    },
    /// Diagnose every source file in one root.
    Diagnose {
        /// Root handle.
        handle: RootId,
    },
    /// Diagnose one source file.
    DiagnoseFile {
        /// Root handle.
        handle: RootId,
        /// Source path.
        path: PathBuf,
    },
    /// Resolve one source file for semantic queries.
    ResolveQueryFile {
        /// Root handle.
        handle: RootId,
        /// Source path.
        path: PathBuf,
    },
    /// Run one semantic query.
    RunQuery {
        /// Root handle.
        handle: RootId,
        /// Encoded query request payload.
        request: QueryRequestPayload,
    },
}
