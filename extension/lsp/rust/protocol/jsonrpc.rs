//! JSON-RPC 2.0 result/error types.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// A JSON-RPC 2.0 error response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

/// A Result type for JSON-RPC operations.
pub type Result<T> = std::result::Result<T, ResponseError>;

/// A JSON-RPC message that can be sent over the wire.
#[derive(Debug)]
pub enum JsonRpcMessage {
    Response { id: JsonValue, result: JsonValue },
    Error { id: JsonValue, error: ResponseError },
    Notification { method: String, params: JsonValue },
}

/// A JSON-RPC 2.0 response structure for serialization.
#[derive(Serialize)]
pub struct JsonRpcResponse<'a> {
    pub jsonrpc: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<&'a JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<&'a JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a ResponseError>,
}

/// A JSON-RPC 2.0 notification structure for serialization.
#[derive(Serialize)]
pub struct JsonRpcNotification<'a> {
    pub jsonrpc: &'a str,
    pub method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<&'a JsonValue>,
}

/// A JSON-RPC 2.0 request structure for deserialization.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<JsonValue>,
    pub method: String,
    #[serde(default)]
    pub params: Option<JsonValue>,
}
