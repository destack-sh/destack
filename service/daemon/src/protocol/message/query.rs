use serde::{Deserialize, Serialize};

use destack_query::{QueryRequest, QueryResponse};
use destack_workspace::Revision;

use super::{BinaryPayload, DiagnosticBatch, RootHandleId};

/// Request payload for one query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequestBody {
    /// Expected workspace semantic revision for mutating requests.
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
        // build the json payload body
        let request = QueryRequestBody {
            expected_revision,
            request,
        };

        // encode the query request as json bytes
        let bytes = serde_json::to_vec(&request).map_err(QueryPayloadCodecError::EncodeRequest)?;
        let payload = BinaryPayload {
            format: super::PayloadFormat::Json,
            body: super::PayloadBody::Inline { bytes },
        };

        Ok(Self { payload })
    }

    /// Encode a prepared query request as protocol payload bytes.
    pub fn from_body(request: QueryRequestBody) -> Result<Self, QueryPayloadCodecError> {
        Self::from_request(request.expected_revision, request.request)
    }

    /// Decode protocol payload bytes into a query request.
    pub fn decode_request(&self) -> Result<QueryRequestBody, QueryPayloadCodecError> {
        // reject non-json payload formats
        if self.payload.format != super::PayloadFormat::Json {
            return Err(QueryPayloadCodecError::DecodeRequestUnexpectedFormat {
                format: self.payload.format,
            });
        }

        // reject unresolved deferred payloads
        let super::PayloadBody::Inline { bytes } = &self.payload.body else {
            return Err(QueryPayloadCodecError::DecodeRequestPayloadDeferred);
        };

        // decode the query request
        serde_json::from_slice(bytes).map_err(QueryPayloadCodecError::DecodeRequest)
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
        // build the json payload body
        let response = QueryResponseBody { revision, response };

        // encode the query response as json bytes
        let bytes =
            serde_json::to_vec(&response).map_err(QueryPayloadCodecError::EncodeResponse)?;
        let payload = BinaryPayload {
            format: super::PayloadFormat::Json,
            body: super::PayloadBody::Inline { bytes },
        };

        Ok(Self { payload })
    }

    /// Decode protocol payload bytes into a query response.
    pub fn decode_response(&self) -> Result<QueryResponseBody, QueryPayloadCodecError> {
        // reject non-json payload formats
        if self.payload.format != super::PayloadFormat::Json {
            return Err(QueryPayloadCodecError::DecodeResponseUnexpectedFormat {
                format: self.payload.format,
            });
        }

        // reject unresolved deferred payloads
        let super::PayloadBody::Inline { bytes } = &self.payload.body else {
            return Err(QueryPayloadCodecError::DecodeResponsePayloadDeferred);
        };

        // decode the query response
        serde_json::from_slice(bytes).map_err(QueryPayloadCodecError::DecodeResponse)
    }
}

/// Errors emitted by query payload encoding and decoding.
#[derive(Debug)]
pub enum QueryPayloadCodecError {
    /// Request encoding failed.
    EncodeRequest(serde_json::Error),
    /// Request decoding failed.
    DecodeRequest(serde_json::Error),
    /// Request payload format is not json.
    DecodeRequestUnexpectedFormat {
        /// Actual payload format.
        format: super::PayloadFormat,
    },
    /// Request payload body is deferred.
    DecodeRequestPayloadDeferred,
    /// Response encoding failed.
    EncodeResponse(serde_json::Error),
    /// Response decoding failed.
    DecodeResponse(serde_json::Error),
    /// Response payload format is not json.
    DecodeResponseUnexpectedFormat {
        /// Actual payload format.
        format: super::PayloadFormat,
    },
    /// Response payload body is deferred.
    DecodeResponsePayloadDeferred,
}

impl std::fmt::Display for QueryPayloadCodecError {
    /// Format a query payload codec error.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EncodeRequest(error) => write!(formatter, "encode request failed: {error}"),
            Self::DecodeRequest(error) => write!(formatter, "decode request failed: {error}"),
            Self::DecodeRequestUnexpectedFormat { format } => write!(
                formatter,
                "decode request failed: unexpected payload format {format:?}"
            ),
            Self::DecodeRequestPayloadDeferred => {
                write!(formatter, "decode request failed: payload is deferred")
            }
            Self::EncodeResponse(error) => write!(formatter, "encode response failed: {error}"),
            Self::DecodeResponse(error) => write!(formatter, "decode response failed: {error}"),
            Self::DecodeResponseUnexpectedFormat { format } => write!(
                formatter,
                "decode response failed: unexpected payload format {format:?}"
            ),
            Self::DecodeResponsePayloadDeferred => {
                write!(formatter, "decode response failed: payload is deferred")
            }
        }
    }
}

impl std::error::Error for QueryPayloadCodecError {
    /// Return the source error.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EncodeRequest(error) => Some(error),
            Self::DecodeRequest(error) => Some(error),
            Self::DecodeRequestUnexpectedFormat { .. } => None,
            Self::DecodeRequestPayloadDeferred => None,
            Self::EncodeResponse(error) => Some(error),
            Self::DecodeResponse(error) => Some(error),
            Self::DecodeResponseUnexpectedFormat { .. } => None,
            Self::DecodeResponsePayloadDeferred => None,
        }
    }
}

/// Query request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonQuery {
    /// Request diagnostics snapshot.
    Diagnostics { handle: RootHandleId },
    /// Request the current semantic revision.
    CurrentRevision { handle: RootHandleId },
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
    /// The current semantic revision.
    CurrentRevision(Revision),
    /// Encoded query response payload.
    Query(QueryResponsePayload),
    /// Encoded query batch response payloads.
    QueryBatch(Vec<QueryResponsePayload>),
}

#[cfg(test)]
mod tests {
    use super::*;

    use destack_query::{HoverRequest, HoverResponse, QueryRequest, QueryResponse};
    use destack_source::Uri;

    /// Preserves query requests across payload encoding and decoding.
    #[test]
    fn test_roundtrip_query_request_payload() {
        // build a representative query request
        let request = QueryRequestBody {
            expected_revision: Some(Revision::from_test_value(7)),
            request: QueryRequest::Hover(HoverRequest {
                uri: Uri::from_string("/root/main.ds"),
                offset: 42,
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
            QueryPayloadCodecError::DecodeRequestUnexpectedFormat { .. }
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
            QueryPayloadCodecError::DecodeRequestPayloadDeferred
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
