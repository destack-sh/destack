use serde_json::Value;
use tspp_source::{FileId, Span};

use crate::{JsonDocument, JsonElement, JsonProperty, JsonString, JsonValue};

/// Convert a serde_json Value to a JSON document with synthetic spans.
pub fn from_serde(value: &Value, file_id: FileId) -> JsonDocument {
    let json_value = value_from_serde(value, file_id);

    JsonDocument {
        value: json_value,
        trivia_before: Vec::new(),
        trivia_after: Vec::new(),
        span: synthetic_span(file_id),
    }
}

/// Convert a serde_json Value to a JSON value with synthetic spans.
pub fn value_from_serde(value: &Value, file_id: FileId) -> JsonValue {
    let span = synthetic_span(file_id);

    match value {
        Value::Null => JsonValue::Null { span },

        Value::Bool(b) => JsonValue::Bool { value: *b, span },

        Value::Number(n) => JsonValue::Number {
            raw: n.to_string(),
            span,
        },

        Value::String(s) => JsonValue::String {
            value: s.clone(),
            span,
        },

        Value::Array(items) => {
            let elements = items
                .iter()
                .map(|item| JsonElement {
                    value: value_from_serde(item, file_id),
                    comma: None,
                    trivia_before: Vec::new(),
                    trivia_after: Vec::new(),
                })
                .collect();

            JsonValue::Array { elements, span }
        }

        Value::Object(map) => {
            let properties = map
                .iter()
                .map(|(key, val)| JsonProperty {
                    key: JsonString {
                        value: key.clone(),
                        span: synthetic_span(file_id),
                    },
                    colon: synthetic_span(file_id),
                    value: value_from_serde(val, file_id),
                    comma: None,
                    trivia_before: Vec::new(),
                    trivia_after: Vec::new(),
                })
                .collect();

            JsonValue::Object { properties, span }
        }
    }
}

/// Create a synthetic span for generated AST nodes.
fn synthetic_span(file_id: FileId) -> Span {
    Span::new(file_id, 0, 0)
}
