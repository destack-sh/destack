use destack_artifact::ArtifactVersion;
use destack_repository::Revision;
use destack_serde::Reflect;
use destack_source::{Content, ContentId};
use serde::{Deserialize, Serialize};

use super::super::handshake::HandshakeResponse;
use super::{
    FileDiagnosticsPayload, FileEditPayload, FileOperationResponse, ProtocolError,
    QueryFilePayload, QueryResponsePayload, RequestId, RootClosedResponse, RootOpenedResponse,
    RootReloadResponse, SourceUpdateResponse, WatchBatchResponse, WatchStartedResponse,
    WatchStoppedResponse,
};
use crate::{
    BenchOutput, BuildOutput, CacheOutput, CheckOutput, CleanOutput, DocOutput, DoctorOutput,
    ExportResult, FileImage, FormatOutput, InfoOutput, RunOutput, SettingsOutput, TargetsOutput,
    TaskOutput, TestOutput,
};

/// Responses emitted by the workspace protocol.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub enum WorkspaceResponse {
    /// Successful handshake response.
    Handshake(HandshakeResponse),
    /// Response to ping.
    Pong,
    /// Response to request cancellation.
    Canceled { id: RequestId },
    /// Response to shutdown request.
    ShutdownAck,
    /// Root open response.
    RootOpened(RootOpenedResponse),
    /// Root close response.
    RootClosed(RootClosedResponse),
    /// Root reload response.
    RootReloaded(RootReloadResponse),
    /// The current semantic revision.
    ReadRevision(Revision),
    /// File operation response.
    FileOperationApplied(FileOperationResponse),
    /// Source update response.
    SourceUpdated(SourceUpdateResponse),
    /// Whether one source file is open.
    IsFileOpen(bool),
    /// Formatting edit.
    FormatFile(Option<FileEditPayload>),
    /// Source files read from one exact revision.
    ReadFiles(Vec<FileImage>),
    /// Watch start response.
    WatchStarted(WatchStartedResponse),
    /// Watch batch response.
    WatchBatchReady(WatchBatchResponse),
    /// Watch stop response.
    WatchStopped(WatchStoppedResponse),
    /// Check response.
    Check(CheckOutput),
    /// Format response.
    Format(FormatOutput),
    /// Build response.
    Build(BuildOutput),
    /// Run response.
    Run(RunOutput),
    /// Test response.
    Test(TestOutput),
    /// Documentation response.
    Doc(DocOutput),
    /// Benchmark response.
    Bench(BenchOutput),
    /// Information response.
    Info(InfoOutput),
    /// Targets response.
    Targets(TargetsOutput),
    /// Cache response.
    Cache(CacheOutput),
    /// Settings response.
    Settings(SettingsOutput),
    /// Doctor response.
    Doctor(DoctorOutput),
    /// Task response.
    Task(TaskOutput),
    /// Clean response.
    Clean(CleanOutput),
    /// Artifact blob response.
    ArtifactResult(ArtifactBlob),
    /// Store response.
    StoreResult(ContentId),
    /// Content payload response.
    LoadResult(Content),
    /// Export response.
    ExportResult(ExportResult),
    /// Diagnostics for every source file in one root.
    Diagnose(Vec<FileDiagnosticsPayload>),
    /// Diagnostics for one source file.
    DiagnoseFile(Option<FileDiagnosticsPayload>),
    /// Source file resolved for semantic queries.
    ResolveQueryFile(Option<QueryFilePayload>),
    /// Encoded semantic query response.
    RunQuery(QueryResponsePayload),
    /// Error response.
    Error(ProtocolError),
}

/// Serialized artifact payload returned by the workspace protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ArtifactBlob {
    /// Exact artifact version.
    pub version: ArtifactVersion,
    /// Serialized artifact payload bytes.
    pub bytes: Vec<u8>,
}
