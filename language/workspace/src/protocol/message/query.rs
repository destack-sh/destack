use destack_serde::Schema;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use destack_query::{QueryModule, QueryRequest, QueryResponse};
use destack_repository::{FormatterOptions, Revision};
use destack_source::{Diagnostic, FileId, ProfileId, Uri};

use super::{BinaryPayload, BinaryPayloadDecodeError, DiagnosticBatch, RootId};
use crate::{DiagnosticView, FileImage};

/// Query context for one root handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct RootSnapshot {
    /// Current semantic revision for the root.
    pub revision: Revision,
    /// Profiles selected for the requested target.
    pub profile_ids: Vec<ProfileId>,
}

/// Request for one source file snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct FileSnapshotRequest {
    /// Path to the source file.
    pub path: PathBuf,
    /// Target name used to select the query profile.
    pub target: Option<String>,
}

/// Source file snapshot for editor adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct FileSnapshot {
    /// Current semantic revision for the file root.
    pub revision: Revision,
    /// The source file id.
    pub file_id: FileId,
    /// Query module for the requested target.
    pub module: Option<QueryModule>,
    /// Formatter options selected for the file.
    pub formatter: FormatterOptions,
    /// File image used for range conversion.
    pub file: FileImage,
}

/// Request for source file images in one revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct FileImagesRequest {
    /// Revision containing the requested files.
    pub revision: Revision,
    /// File ids to resolve.
    pub file_ids: Vec<FileId>,
}

/// Diagnostics and file image for one source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct DiagnosticSnapshot {
    /// Revision containing the diagnostics.
    pub revision: Revision,
    /// File image used for range conversion.
    pub file: FileImage,
    /// Diagnostic uri.
    pub diagnostic_uri: Uri,
    /// Protocol file version when the file is open.
    pub diagnostic_version: Option<i32>,
    /// Diagnostics for the file.
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSnapshot {
    /// Create a protocol diagnostic snapshot from one local diagnostic view.
    pub fn new(revision: Revision, view: &DiagnosticView) -> Self {
        Self {
            revision,
            file: FileImage::from(view.file.as_ref()),
            diagnostic_uri: view.diagnostic_uri.clone(),
            diagnostic_version: view.diagnostic_version,
            diagnostics: view.diagnostics.clone(),
        }
    }
}

/// Request payload for one query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct QueryRequestBody {
    /// Expected workspace semantic revision.
    pub expected_revision: Option<Revision>,
    /// Query request.
    pub request: QueryRequest,
}

/// Response payload for one query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct QueryResponseBody {
    /// Workspace semantic revision after request execution.
    pub revision: Revision,
    /// Query response.
    pub response: QueryResponse,
}

/// Encoded query request payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
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
            BinaryPayload::from_value(&request).map_err(QueryPayloadCodecError::EncodeRequest)?;

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
            .to_value()
            .map_err(QueryPayloadCodecError::DecodeRequest)
    }
}

/// Encoded query response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
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
            BinaryPayload::from_value(&response).map_err(QueryPayloadCodecError::EncodeResponse)?;

        Ok(Self { payload })
    }

    /// Decode protocol payload bytes into a query response.
    pub fn decode_response(&self) -> Result<QueryResponseBody, QueryPayloadCodecError> {
        // decode the query response
        self.payload
            .to_value()
            .map_err(QueryPayloadCodecError::DecodeResponse)
    }
}

/// Errors emitted by query payload encoding and decoding.
#[derive(Debug)]
pub enum QueryPayloadCodecError {
    /// Request encoding failed.
    EncodeRequest(destack_serde::Error),
    /// Request decoding failed.
    DecodeRequest(BinaryPayloadDecodeError),
    /// Response encoding failed.
    EncodeResponse(destack_serde::Error),
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub enum WorkspaceQuery {
    /// Request diagnostics snapshot.
    Diagnostics { handle: RootId },
    /// Request rich diagnostics with file images.
    DiagnosticSnapshots { handle: RootId },
    /// Request diagnostics for one file.
    FileDiagnostics {
        /// Root handle.
        handle: RootId,
        /// Source path.
        path: PathBuf,
    },
    /// Request the current semantic revision.
    CurrentRevision { handle: RootId },
    /// Request query context for one root.
    RootSnapshot {
        /// Root handle.
        handle: RootId,
        /// Target name used to select query profiles.
        target: Option<String>,
    },
    /// Request a source file snapshot.
    FileSnapshot {
        /// Root handle.
        handle: RootId,
        /// Snapshot request.
        request: FileSnapshotRequest,
    },
    /// Request source file images for a revision.
    FileImages {
        /// Root handle.
        handle: RootId,
        /// File image request.
        request: FileImagesRequest,
    },
    /// Execute a query.
    Execute {
        /// Root handle.
        handle: RootId,
        /// Encoded query request payload.
        request: QueryRequestPayload,
    },
    /// Execute a batch of queries.
    ExecuteBatch {
        /// Root handle.
        handle: RootId,
        /// Encoded query request payloads.
        requests: Vec<QueryRequestPayload>,
    },
}

/// Query response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub enum WorkspaceQueryResponse {
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
    FileImages(Vec<FileImage>),
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

    /// Rejects request payloads that are malformed.
    #[test]
    fn test_decode_query_request_payload_rejects_malformed_bytes() {
        // build a malformed request payload
        let payload = QueryRequestPayload {
            payload: BinaryPayload {
                body: super::super::PayloadBody::Inline {
                    bytes: vec![1, 2, 3],
                },
            },
        };

        // assert decoding fails with a payload error
        let error = payload.decode_request().expect_err("decode should fail");
        assert!(matches!(
            error,
            QueryPayloadCodecError::DecodeRequest(BinaryPayloadDecodeError::InvalidPayload(_))
        ));
    }

    /// Rejects request payloads that are still deferred.
    #[test]
    fn test_decode_query_request_payload_rejects_deferred() {
        // build a deferred request payload
        let payload = QueryRequestPayload {
            payload: BinaryPayload {
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
