use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use destack_query::{QueryModule, QueryRequest, QueryResponse};
use destack_repository::Revision;
use destack_source::{Diagnostic, FileId, ProfileId, Uri};

use super::{
    BinaryPayload, BinaryPayloadDecodeError, DiagnosticBatch, FileUpdateImage, RootHandleId,
};

/// Query context for one root handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootSnapshot {
    /// Current semantic revision for the root.
    pub revision: Revision,
    /// Profiles selected for the requested target.
    pub profile_ids: Vec<ProfileId>,
}

/// Request for one source file snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSnapshotRequest {
    /// Path to the source file.
    pub path: PathBuf,
    /// Target name used to select the query profile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// Source file snapshot for editor adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSnapshot {
    /// Current semantic revision for the file root.
    pub revision: Revision,
    /// The source file id.
    pub file_id: FileId,
    /// Query module for the requested target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<QueryModule>,
    /// File image used for range conversion.
    pub file: FileUpdateImage,
}

/// Request for source file images in one revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileImagesRequest {
    /// Revision containing the requested files.
    pub revision: Revision,
    /// File ids to resolve.
    pub file_ids: Vec<FileId>,
}

/// Diagnostics and file image for one source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    /// Revision containing the diagnostics.
    pub revision: Revision,
    /// File image used for range conversion.
    pub file: FileUpdateImage,
    /// Diagnostic uri.
    pub diagnostic_uri: Uri,
    /// Protocol file version when the file is open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic_version: Option<i32>,
    /// Diagnostics for the file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Request payload for one query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequestBody {
    /// Expected workspace semantic revision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<Revision>,
    /// Query request.
    #[serde(flatten)]
    pub request: QueryRequest,
}

/// Response payload for one query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResponseBody {
    /// Workspace semantic revision after request execution.
    pub revision: Revision,
    /// Query response.
    #[serde(flatten)]
    pub response: QueryResponse,
}

/// Encoded query request payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequestPayload {
    /// Encoded request payload.
    pub payload: BinaryPayload,
}

impl QueryRequestPayload {
    /// Encode a query request as protocol payload bytes.
    pub fn from_request(
        expected_revision: Option<Revision>,
        request: QueryRequest,
    ) -> Result<Self, QueryPayloadCodecError> {
        // build the request body
        let request = QueryRequestBody {
            expected_revision,
            request,
        };

        // encode the request
        let payload =
            BinaryPayload::from_json(&request).map_err(QueryPayloadCodecError::EncodeRequest)?;

        Ok(Self { payload })
    }

    /// Encode a prepared query request as protocol payload bytes.
    pub fn from_body(request: QueryRequestBody) -> Result<Self, QueryPayloadCodecError> {
        Self::from_request(request.expected_revision, request.request)
    }

    /// Decode protocol payload bytes into a query request.
    pub fn decode_request(&self) -> Result<QueryRequestBody, QueryPayloadCodecError> {
        // decode the query request
        self.payload
            .to_json()
            .map_err(QueryPayloadCodecError::DecodeRequest)
    }
}

/// Encoded query response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResponsePayload {
    /// Encoded response payload.
    pub payload: BinaryPayload,
}

impl QueryResponsePayload {
    /// Encode a query response as protocol payload bytes.
    pub fn from_response(
        revision: Revision,
        response: QueryResponse,
    ) -> Result<Self, QueryPayloadCodecError> {
        // build the response body
        let response = QueryResponseBody { revision, response };

        // encode the response
        let payload =
            BinaryPayload::from_json(&response).map_err(QueryPayloadCodecError::EncodeResponse)?;

        Ok(Self { payload })
    }

    /// Decode protocol payload bytes into a query response.
    pub fn decode_response(&self) -> Result<QueryResponseBody, QueryPayloadCodecError> {
        // decode the query response
        self.payload
            .to_json()
            .map_err(QueryPayloadCodecError::DecodeResponse)
    }
}

/// Errors emitted by query payload encoding and decoding.
#[derive(Debug)]
pub enum QueryPayloadCodecError {
    /// Request encoding failed.
    EncodeRequest(serde_json::Error),
    /// Request decoding failed.
    DecodeRequest(BinaryPayloadDecodeError),
    /// Response encoding failed.
    EncodeResponse(serde_json::Error),
    /// Response decoding failed.
    DecodeResponse(BinaryPayloadDecodeError),
}

impl std::fmt::Display for QueryPayloadCodecError {
    /// Format a query payload codec error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EncodeRequest(error) => write!(formatter, "encode request failed: {error}"),
            Self::DecodeRequest(error) => write!(formatter, "decode request failed: {error}"),
            Self::EncodeResponse(error) => write!(formatter, "encode response failed: {error}"),
            Self::DecodeResponse(error) => write!(formatter, "decode response failed: {error}"),
        }
    }
}

impl std::error::Error for QueryPayloadCodecError {
    /// Return the source error.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EncodeRequest(error) => Some(error),
            Self::DecodeRequest(error) => Some(error),
            Self::EncodeResponse(error) => Some(error),
            Self::DecodeResponse(error) => Some(error),
        }
    }
}

/// Query request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonQuery {
    /// Request diagnostics snapshot.
    Diagnostics { handle: RootHandleId },
    /// Request rich diagnostics with file images.
    DiagnosticSnapshots { handle: RootHandleId },
    /// Request diagnostics for one file.
    FileDiagnostics {
        /// Root handle.
        handle: RootHandleId,
        /// Source path.
        path: PathBuf,
    },
    /// Request the current semantic revision.
    CurrentRevision { handle: RootHandleId },
    /// Request query context for one root.
    RootSnapshot {
        /// Root handle.
        handle: RootHandleId,
        /// Target name used to select query profiles.
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<String>,
    },
    /// Request a source file snapshot.
    FileSnapshot {
        /// Root handle.
        handle: RootHandleId,
        /// Snapshot request.
        request: FileSnapshotRequest,
    },
    /// Request source file images for a revision.
    FileImages {
        /// Root handle.
        handle: RootHandleId,
        /// File image request.
        request: FileImagesRequest,
    },
    /// Execute a query.
    Execute {
        /// Root handle.
        handle: RootHandleId,
        /// Encoded query request payload.
        request: QueryRequestPayload,
    },
    /// Execute a batch of queries.
    ExecuteBatch {
        /// Root handle.
        handle: RootHandleId,
        /// Encoded query request payloads.
        requests: Vec<QueryRequestPayload>,
    },
}

/// Query response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonQueryResponse {
    /// Diagnostics snapshot.
    Diagnostics(Vec<DiagnosticBatch>),
    /// Rich diagnostics with file images.
    DiagnosticSnapshots(Vec<DiagnosticSnapshot>),
    /// Diagnostics for one file.
    FileDiagnostics(Option<DiagnosticSnapshot>),
    /// The current semantic revision.
    CurrentRevision(Revision),
    /// Query context for one root.
    RootSnapshot(RootSnapshot),
    /// Source file snapshot.
    FileSnapshot(Option<FileSnapshot>),
    /// Source file images.
    FileImages(Vec<FileUpdateImage>),
    /// Encoded query response payload.
    Query(QueryResponsePayload),
    /// Encoded query batch response payloads.
    QueryBatch(Vec<QueryResponsePayload>),
}

#[cfg(test)]
mod tests {
    use super::*;

    use destack_query::{HoverResponse, QueryRequest, QueryResponse, WorkspaceSymbolsRequest};

    /// Preserves query requests across payload encoding and decoding.
    #[test]
    fn test_roundtrip_query_request_payload() {
        // build a representative query request
        let request = QueryRequestBody {
            expected_revision: Some(Revision::from_test_value(7)),
            request: QueryRequest::WorkspaceSymbols(WorkspaceSymbolsRequest {
                profile_ids: Vec::new(),
                query: "main".to_string(),
                max_results: 16,
            }),
        };

        // encode and decode through the query request payload
        let payload = QueryRequestPayload::from_body(request.clone()).expect("encode");
        let decoded = payload.decode_request().expect("decode");

        // assert full roundtrip preservation
        assert_eq!(decoded, request);
    }

    /// Rejects request payloads that are not json encoded.
    #[test]
    fn test_decode_query_request_payload_rejects_non_json() {
        // build a non-json request payload
        let payload = QueryRequestPayload {
            payload: BinaryPayload {
                format: super::super::PayloadFormat::Postcard,
                body: super::super::PayloadBody::Inline {
                    bytes: vec![1, 2, 3],
                },
            },
        };

        // assert decoding fails with an unexpected format error
        let error = payload.decode_request().expect_err("decode should fail");
        assert!(matches!(
            error,
            QueryPayloadCodecError::DecodeRequest(
                BinaryPayloadDecodeError::UnexpectedFormat { .. }
            )
        ));
    }

    /// Rejects request payloads that are still deferred.
    #[test]
    fn test_decode_query_request_payload_rejects_deferred() {
        // build a deferred json request payload
        let payload = QueryRequestPayload {
            payload: BinaryPayload {
                format: super::super::PayloadFormat::Json,
                body: super::super::PayloadBody::Deferred {
                    id: super::super::PayloadId::new(7),
                    total_bytes: 1024,
                },
            },
        };

        // assert decoding fails until payload streaming resolves inline bytes
        let error = payload.decode_request().expect_err("decode should fail");
        assert!(matches!(
            error,
            QueryPayloadCodecError::DecodeRequest(BinaryPayloadDecodeError::PayloadDeferred)
        ));
    }

    /// Preserves query responses across payload encoding and decoding.
    #[test]
    fn test_roundtrip_query_response_payload() {
        // build a representative query response
        let response = QueryResponseBody {
            revision: Revision::from_test_value(7),
            response: QueryResponse::Hover(HoverResponse { hover: None }),
        };

        // encode and decode through the query response payload
        let payload =
            QueryResponsePayload::from_response(response.revision, response.response.clone())
                .expect("encode");
        let decoded = payload.decode_response().expect("decode");

        // assert full roundtrip preservation
        assert_eq!(decoded, response);
    }
}
