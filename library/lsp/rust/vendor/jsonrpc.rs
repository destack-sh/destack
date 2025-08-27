//! JSON-RPC 2.0 result/error types.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
	pub code: i32,
	pub message: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub data: Option<JsonValue>,
}

pub type Result<T> = std::result::Result<T, ResponseError>;


