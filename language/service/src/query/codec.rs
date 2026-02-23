use serde::{Deserialize, Serialize};

use super::{QueryRequestEnvelope, QueryResponseEnvelope};

/// Wire codec identifiers for serialized query envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryCodec {
    /// Json envelope codec v1.
    JsonV1,
}

impl QueryCodec {
    /// Encode a query request envelope into bytes.
    pub fn encode_request(
        self,
        request: &QueryRequestEnvelope,
    ) -> Result<Vec<u8>, QueryCodecError> {
        // dispatch request encoding by codec
        match self {
            QueryCodec::JsonV1 => {
                serde_json::to_vec(request).map_err(QueryCodecError::EncodeRequest)
            }
        }
    }

    /// Decode a query request envelope from bytes.
    pub fn decode_request(self, bytes: &[u8]) -> Result<QueryRequestEnvelope, QueryCodecError> {
        // dispatch request decoding by codec
        match self {
            QueryCodec::JsonV1 => {
                serde_json::from_slice(bytes).map_err(QueryCodecError::DecodeRequest)
            }
        }
    }

    /// Encode a query response envelope into bytes.
    pub fn encode_response(
        self,
        response: &QueryResponseEnvelope,
    ) -> Result<Vec<u8>, QueryCodecError> {
        // dispatch response encoding by codec
        match self {
            QueryCodec::JsonV1 => {
                serde_json::to_vec(response).map_err(QueryCodecError::EncodeResponse)
            }
        }
    }

    /// Decode a query response envelope from bytes.
    pub fn decode_response(self, bytes: &[u8]) -> Result<QueryResponseEnvelope, QueryCodecError> {
        // dispatch response decoding by codec
        match self {
            QueryCodec::JsonV1 => {
                serde_json::from_slice(bytes).map_err(QueryCodecError::DecodeResponse)
            }
        }
    }
}

/// Errors emitted by query wire encoding and decoding.
#[derive(Debug)]
pub enum QueryCodecError {
    /// Request envelope encoding failed.
    EncodeRequest(serde_json::Error),
    /// Request envelope decoding failed.
    DecodeRequest(serde_json::Error),
    /// Response envelope encoding failed.
    EncodeResponse(serde_json::Error),
    /// Response envelope decoding failed.
    DecodeResponse(serde_json::Error),
}

impl std::fmt::Display for QueryCodecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format wire codec errors
        match self {
            QueryCodecError::EncodeRequest(error) => {
                write!(formatter, "encode request failed: {error}")
            }
            QueryCodecError::DecodeRequest(error) => {
                write!(formatter, "decode request failed: {error}")
            }
            QueryCodecError::EncodeResponse(error) => {
                write!(formatter, "encode response failed: {error}")
            }
            QueryCodecError::DecodeResponse(error) => {
                write!(formatter, "decode response failed: {error}")
            }
        }
    }
}

impl std::error::Error for QueryCodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QueryCodecError::EncodeRequest(error) => Some(error),
            QueryCodecError::DecodeRequest(error) => Some(error),
            QueryCodecError::EncodeResponse(error) => Some(error),
            QueryCodecError::DecodeResponse(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::query::{HoverRequest, HoverResponse, QueryRequest, QueryResponse};
    use destack_source::Uri;

    #[test]
    fn test_roundtrip_query_request_with_json_v1() {
        // build a representative request envelope
        let request = QueryRequestEnvelope {
            snapshot_id: Some("snapshot-123".to_string()),
            request: QueryRequest::Hover(HoverRequest {
                uri: Uri::from_string("/workspace/main.ds"),
                offset: 11,
            }),
        };

        // encode and decode through the wire codec
        let codec = QueryCodec::JsonV1;
        let encoded = codec.encode_request(&request).expect("encode request");
        let decoded = codec.decode_request(&encoded).expect("decode request");

        // assert roundtrip preservation
        assert_eq!(decoded, request);
    }

    #[test]
    fn test_roundtrip_query_response_with_json_v1() {
        // build a representative response envelope
        let response = QueryResponseEnvelope {
            snapshot_id: "snapshot-123".to_string(),
            response: QueryResponse::Hover(HoverResponse { hover: None }),
        };

        // encode and decode through the wire codec
        let codec = QueryCodec::JsonV1;
        let encoded = codec.encode_response(&response).expect("encode response");
        let decoded = codec.decode_response(&encoded).expect("decode response");

        // assert roundtrip preservation
        assert_eq!(decoded, response);
    }
}
