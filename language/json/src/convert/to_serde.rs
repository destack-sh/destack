use serde_json::{Map, Number, Value};

use crate::{JsonDocument, JsonValue};

/// Convert a JSON document to a serde_json Value.
pub fn to_serde(document: &JsonDocument) -> Value {
    value_to_serde(&document.value)
}

/// Convert a JSON value to a serde_json Value.
pub fn value_to_serde(value: &JsonValue) -> Value {
    match value {
        JsonValue::Null { .. } => Value::Null,

        JsonValue::Bool { value, .. } => Value::Bool(*value),

        JsonValue::Number { raw, .. } => {
            // parse the raw number string into a serde_json Number
            if let Ok(n) = raw.parse::<Number>() {
                Value::Number(n)
            }
            // fallback for invalid numbers (shouldn't happen with valid AST)
            else {
                Value::Null
            }
        }

        JsonValue::String { value, .. } => Value::String(value.clone()),

        JsonValue::Array { elements, .. } => {
            let items = elements.iter().map(|e| value_to_serde(&e.value)).collect();

            Value::Array(items)
        }

        JsonValue::Object { properties, .. } => {
            let mut map = Map::new();

            for prop in properties {
                let key = prop.key.value.clone();
                let value = value_to_serde(&prop.value);
                map.insert(key, value);
            }

            Value::Object(map)
        }
    }
}
