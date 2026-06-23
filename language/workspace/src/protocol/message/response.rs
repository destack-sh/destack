use destack_artifact::ArtifactVersion;
use destack_serde::Schema;
use destack_source::{Content, ContentId};
use serde::{Deserialize, Serialize};

use super::super::handshake::HandshakeResponse;
use super::{
    FileOperationResponse, ProtocolError, RequestId, RootClosedResponse, RootOpenedResponse,
    RootReloadResponse, SourceUpdateResponse, WatchBatchResponse, WatchStartedResponse,
    WatchStoppedResponse, WorkspaceQueryResponse,
};
use crate::{
    BenchOutput, BuildOutput, CacheOutput, CheckOutput, CleanOutput, DocOutput, DoctorOutput,
    ExportResult, FormatOutput, InfoOutput, LintOutput, RunOutput, SettingsOutput, TargetsOutput,
    TaskOutput, TestOutput,
};

/// Responses emitted by the workspace protocol.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize, Schema)]
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
    /// File operation response.
    FileOperationApplied(FileOperationResponse),
    /// Source update response.
    SourceUpdated(SourceUpdateResponse),
    /// Watch start response.
    WatchStarted(WatchStartedResponse),
    /// Watch batch response.
    WatchBatchReady(WatchBatchResponse),
    /// Watch stop response.
    WatchStopped(WatchStoppedResponse),
    /// Check response.
    Check(CheckOutput),
    /// Lint response.
    Lint(LintOutput),
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
    /// Query response.
    QueryResult(WorkspaceQueryResponse),
    /// Error response.
    Error(ProtocolError),
}

/// Serialized artifact payload returned by the workspace protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct ArtifactBlob {
    /// Exact artifact version.
    pub version: ArtifactVersion,
    /// Serialized artifact payload bytes.
    pub bytes: Vec<u8>,
}
