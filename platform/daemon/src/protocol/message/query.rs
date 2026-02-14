use serde::{Deserialize, Serialize};

use destack_service::query::{QueryRequestEnvelope, QueryResponseEnvelope};
use destack_source::{ModuleId, ProfileId};

use super::{BinaryPayload, DiagnosticBatch, WorkspaceHandleId};

/// Query request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonQuery {
    /// Request workspace index snapshot.
    WorkspaceIndex { handle: WorkspaceHandleId },
    /// Request module graph payload.
    ModuleGraph {
        /// Workspace handle.
        handle: WorkspaceHandleId,
        /// Profile id for the graph.
        profile: ProfileId,
    },
    /// Request module signature payload.
    ModuleSignature {
        /// Workspace handle.
        handle: WorkspaceHandleId,
        /// Module id.
        module_id: ModuleId,
        /// Profile id.
        profile: ProfileId,
    },
    /// Request diagnostics snapshot.
    Diagnostics { handle: WorkspaceHandleId },
    /// Request cache statistics.
    CacheStats { handle: WorkspaceHandleId },
    /// Execute a workspace query.
    WorkspaceQuery {
        /// Workspace handle.
        handle: WorkspaceHandleId,
        /// Query request envelope.
        request: QueryRequestEnvelope,
    },
    /// Execute a batch of workspace queries.
    WorkspaceQueryBatch {
        /// Workspace handle.
        handle: WorkspaceHandleId,
        /// Query request envelopes.
        requests: Vec<QueryRequestEnvelope>,
    },
}

/// Query response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonQueryResponse {
    /// Workspace index payload.
    WorkspaceIndex(BinaryPayload),
    /// Module graph payload.
    ModuleGraph(BinaryPayload),
    /// Module signature payload.
    ModuleSignature(BinaryPayload),
    /// Diagnostics snapshot.
    Diagnostics(Vec<DiagnosticBatch>),
    /// Cache stats payload.
    CacheStats(CacheStatsPayload),
    /// Workspace query response.
    WorkspaceQuery(QueryResponseEnvelope),
    /// Workspace query batch response.
    WorkspaceQueryBatch(Vec<QueryResponseEnvelope>),
}

/// Cache stats payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheStatsPayload {
    /// Cache hits observed.
    pub hits: u64,
    /// Cache misses observed.
    pub misses: u64,
    /// Cache entries loaded from disk.
    pub disk_reads: u64,
    /// Cache entries written to disk.
    pub disk_writes: u64,
}
