use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use destack_query::Module;
use destack_repository::Revision;
use destack_serde::Reflect;

use super::{BinaryPayload, BinaryPayloadDecodeError};
use crate::{Error, FileImage, QueryFile, RunQueryRequest, RunQueryResponse};

/// Wire representation of one query file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryFilePayload {
    /// The requested source path.
    pub path: PathBuf,
    /// The exact semantic revision.
    pub revision: Revision,
    /// The module containing the file.
    pub module: Module,
    /// The source file image.
    pub file: FileImage,
}

impl TryFrom<QueryFilePayload> for QueryFile {
    type Error = Error;

    /// Convert one wire payload into runtime query state.
    fn try_from(payload: QueryFilePayload) -> Result<Self, Self::Error> {
        let file = payload.file.into_file()?;

        Ok(Self {
            path: payload.path,
            revision: payload.revision,
            module: payload.module,
            file: Arc::new(file),
        })
    }
}

impl From<&QueryFile> for QueryFilePayload {
    /// Build one wire payload from runtime query state.
    fn from(file: &QueryFile) -> Self {
        Self {
            path: file.path.clone(),
            revision: file.revision,
            module: file.module,
            file: FileImage::from(file.file.as_ref()),
        }
    }
}

/// Encoded query request payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryRequestPayload {
    /// Encoded request payload.
    pub payload: BinaryPayload,
}

impl QueryRequestPayload {
    /// Encode a query request as protocol payload bytes.
    pub fn from_request(request: RunQueryRequest) -> Result<Self, QueryPayloadCodecError> {
        // encode the request
        let payload =
            BinaryPayload::from_value(&request).map_err(QueryPayloadCodecError::EncodeRequest)?;

        Ok(Self { payload })
    }

    /// Decode protocol payload bytes into a query request.
    pub fn decode_request(&self) -> Result<RunQueryRequest, QueryPayloadCodecError> {
        // decode the query request
        self.payload
            .to_value()
            .map_err(QueryPayloadCodecError::DecodeRequest)
    }
}

/// Encoded query response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct QueryResponsePayload {
    /// Encoded response payload.
    pub payload: BinaryPayload,
}

impl QueryResponsePayload {
    /// Encode a query response as protocol payload bytes.
    pub fn from_response(response: RunQueryResponse) -> Result<Self, QueryPayloadCodecError> {
        // encode the response
        let payload =
            BinaryPayload::from_value(&response).map_err(QueryPayloadCodecError::EncodeResponse)?;

        Ok(Self { payload })
    }

    /// Decode protocol payload bytes into a query response.
    pub fn decode_response(&self) -> Result<RunQueryResponse, QueryPayloadCodecError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::RevisionPolicy;
    use destack_query::{HoverResponse, QueryRequest, QueryResponse, SearchSymbolsRequest};

    /// Preserves query requests across payload encoding and decoding.
    #[test]
    fn test_roundtrip_query_request_payload() {
        // build a representative query request
        let request = RunQueryRequest {
            revision: RevisionPolicy::Exact(Revision::from_test_value(7)),
            request: QueryRequest::SearchSymbols(SearchSymbolsRequest {
                query: "main".to_string(),
                max_results: 16,
            }),
        };

        // encode and decode through the query request payload
        let payload = QueryRequestPayload::from_request(request.clone()).expect("encode");
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
        let response = RunQueryResponse {
            revision: Revision::from_test_value(7),
            response: QueryResponse::Hover(HoverResponse { hover: None }),
        };

        // encode and decode through the query response payload
        let payload = QueryResponsePayload::from_response(response.clone()).expect("encode");
        let decoded = payload.decode_response().expect("decode");

        // assert full roundtrip preservation
        assert_eq!(decoded, response);
    }
}
