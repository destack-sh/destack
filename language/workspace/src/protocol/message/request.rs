use destack_artifact::ArtifactReference;
use destack_serde::Schema;
use destack_source::{Content, ContentId};
use serde::{Deserialize, Serialize};

use super::super::handshake::HandshakeRequest;
use super::{
    CloseRootRequest, FileOperationRequest, OpenRootRequest, ReloadRootRequest, RequestId, RootId,
    SourceUpdateRequest, WatchNextRequest, WatchStartRequest, WatchStopRequest, WorkspaceQuery,
};
use crate::{
    BenchInput, BuildInput, CacheInput, CheckInput, CleanInput, DocInput, DoctorInput,
    ExportRequest, FormatInput, InfoInput, LintInput, RunInput, SettingsInput, TargetsInput,
    TaskInput, TestInput,
};

/// Requests accepted by the workspace protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
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
    /// Apply a file operation to a root.
    ApplyFileOperation(FileOperationRequest),
    /// Apply a source update to a root.
    ApplySourceUpdate(SourceUpdateRequest),
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
    /// Lint source state.
    Lint {
        /// Root handle.
        handle: RootId,
        /// Lint input.
        input: LintInput,
    },
    /// Format source files or content.
    Format {
        /// Root handle.
        handle: RootId,
        /// Format input.
        input: FormatInput,
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
    /// Execute a query.
    Query(WorkspaceQuery),
}
