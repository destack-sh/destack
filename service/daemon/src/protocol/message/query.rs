use serde::{Deserialize, Serialize};

use destack_query::{QueryRequestEnvelope, QueryResponseEnvelope};
use destack_workspace::Revision;

use super::{BinaryPayload, DiagnosticBatch, RootHandleId};

/// Encoded root query request payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequestPayload {
    /// Encoded request payload.
    pub payload: BinaryPayload,
}

impl QueryRequestPayload {
    /// Encode a semantic request envelope as protocol payload bytes.
    pub fn from_envelope(envelope: QueryRequestEnvelope) -> Result<Self, QueryPayloadCodecError> {
        // encode the semantic request envelope as json bytes
        let bytes = serde_json::to_vec(&envelope).map_err(QueryPayloadCodecError::EncodeRequest)?;
        let payload = BinaryPayload {
            format: super::PayloadFormat::Json,
            body: super::PayloadBody::Inline { bytes },
        };

        Ok(Self { payload })
    }

    /// Decode protocol payload bytes into a semantic request envelope.
    pub fn decode_envelope(&self) -> Result<QueryRequestEnvelope, QueryPayloadCodecError> {
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

        // decode the semantic request envelope
        serde_json::from_slice(bytes).map_err(QueryPayloadCodecError::DecodeRequest)
    }
}

/// Encoded root query response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResponsePayload {
    /// Encoded response payload.
    pub payload: BinaryPayload,
}

impl QueryResponsePayload {
    /// Encode a semantic response envelope as protocol payload bytes.
    pub fn from_envelope(envelope: QueryResponseEnvelope) -> Result<Self, QueryPayloadCodecError> {
        // encode the semantic response envelope as json bytes
        let bytes =
            serde_json::to_vec(&envelope).map_err(QueryPayloadCodecError::EncodeResponse)?;
        let payload = BinaryPayload {
            format: super::PayloadFormat::Json,
            body: super::PayloadBody::Inline { bytes },
        };

        Ok(Self { payload })
    }

    /// Decode protocol payload bytes into a semantic response envelope.
    pub fn decode_envelope(&self) -> Result<QueryResponseEnvelope, QueryPayloadCodecError> {
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

        // decode the semantic response envelope
        serde_json::from_slice(bytes).map_err(QueryPayloadCodecError::DecodeResponse)
    }
}

/// Errors emitted by query payload encoding and decoding.
#[derive(Debug)]
pub enum QueryPayloadCodecError {
    /// Request envelope encoding failed.
    EncodeRequest(serde_json::Error),
    /// Request envelope decoding failed.
    DecodeRequest(serde_json::Error),
    /// Request payload format is not json.
    DecodeRequestUnexpectedFormat {
        /// Actual payload format.
        format: super::PayloadFormat,
    },
    /// Request payload body is deferred.
    DecodeRequestPayloadDeferred,
    /// Response envelope encoding failed.
    EncodeResponse(serde_json::Error),
    /// Response envelope decoding failed.
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
    /// Execute a root query.
    RootQuery {
        /// Root handle.
        handle: RootHandleId,
        /// Encoded query request payload.
        request: QueryRequestPayload,
    },
    /// Execute a batch of root queries.
    RootQueryBatch {
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
    /// Encoded root query response payload.
    RootQuery(QueryResponsePayload),
    /// Encoded root query batch response payloads.
    RootQueryBatch(Vec<QueryResponsePayload>),
}

#[cfg(test)]
mod tests {
    use super::*;

    use destack_query::{HoverRequest, HoverResponse, QueryRequest, QueryResponse};
    use destack_source::Uri;

    /// Preserves query request envelopes across payload encoding and decoding.
    #[test]
    fn test_roundtrip_query_request_payload() {
        // build a representative query request envelope
        let envelope = QueryRequestEnvelope {
            expected_revision: Some(Revision::from_test_value(7)),
            request: QueryRequest::Hover(HoverRequest {
                uri: Uri::from_string("/root/main.ds"),
                offset: 42,
            }),
        };

        // encode and decode through the query request payload
        let payload = QueryRequestPayload::from_envelope(envelope.clone()).expect("encode");
        let decoded = payload.decode_envelope().expect("decode");

        // assert full roundtrip preservation
        assert_eq!(decoded, envelope);
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
        let error = payload.decode_envelope().expect_err("decode should fail");
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
        let error = payload.decode_envelope().expect_err("decode should fail");
        assert!(matches!(
            error,
            QueryPayloadCodecError::DecodeRequestPayloadDeferred
        ));
    }

    /// Preserves query response envelopes across payload encoding and decoding.
    #[test]
    fn test_roundtrip_query_response_payload() {
        // build a representative query response envelope
        let envelope = QueryResponseEnvelope {
            revision: Revision::from_test_value(7),
            response: QueryResponse::Hover(HoverResponse { hover: None }),
        };

        // encode and decode through the query response payload
        let payload = QueryResponsePayload::from_envelope(envelope.clone()).expect("encode");
        let decoded = payload.decode_envelope().expect("decode");

        // assert full roundtrip preservation
        assert_eq!(decoded, envelope);
    }
}
