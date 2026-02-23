use serde::{Deserialize, Serialize};

use destack_service::query::{
    QueryCodec, QueryCodecError, QueryRequestEnvelope, QueryResponseEnvelope,
};
use destack_source::{ModuleId, ProfileId};

use super::{BinaryPayload, DiagnosticBatch, WorkspaceHandleId};

/// Default query wire codec used by protocol payloads.
const DEFAULT_QUERY_CODEC: QueryCodec = QueryCodec::JsonV1;

/// Encoded workspace query request payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequestPayload {
    /// Codec used for request bytes.
    pub codec: QueryCodec,
    /// Encoded request bytes.
    pub bytes: Vec<u8>,
}

impl QueryRequestPayload {
    /// Encode a semantic request envelope as protocol payload bytes.
    pub fn from_envelope(envelope: QueryRequestEnvelope) -> Result<Self, QueryCodecError> {
        // encode using the default daemon query codec
        let codec = DEFAULT_QUERY_CODEC;
        let bytes = codec.encode_request(&envelope)?;

        Ok(Self { codec, bytes })
    }

    /// Decode protocol payload bytes into a semantic request envelope.
    pub fn decode_envelope(&self) -> Result<QueryRequestEnvelope, QueryCodecError> {
        // decode using the payload codec
        self.codec.decode_request(&self.bytes)
    }
}

/// Encoded workspace query response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResponsePayload {
    /// Codec used for response bytes.
    pub codec: QueryCodec,
    /// Encoded response bytes.
    pub bytes: Vec<u8>,
}

impl QueryResponsePayload {
    /// Encode a semantic response envelope as protocol payload bytes.
    pub fn from_envelope(envelope: QueryResponseEnvelope) -> Result<Self, QueryCodecError> {
        // encode using the default daemon query codec
        let codec = DEFAULT_QUERY_CODEC;
        let bytes = codec.encode_response(&envelope)?;

        Ok(Self { codec, bytes })
    }

    /// Decode protocol payload bytes into a semantic response envelope.
    pub fn decode_envelope(&self) -> Result<QueryResponseEnvelope, QueryCodecError> {
        // decode using the payload codec
        self.codec.decode_response(&self.bytes)
    }
}

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
        /// Encoded query request payload.
        request: QueryRequestPayload,
    },
    /// Execute a batch of workspace queries.
    WorkspaceQueryBatch {
        /// Workspace handle.
        handle: WorkspaceHandleId,
        /// Encoded query request payloads.
        requests: Vec<QueryRequestPayload>,
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
    /// Encoded workspace query response payload.
    WorkspaceQuery(QueryResponsePayload),
    /// Encoded workspace query batch response payloads.
    WorkspaceQueryBatch(Vec<QueryResponsePayload>),
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
