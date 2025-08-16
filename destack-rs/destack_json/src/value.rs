use crate::format::{FormatOptions, format_json};
use std::fmt;

/// A JSON value representation.
#[derive(Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(std::collections::HashMap<String, JsonValue>),
}

impl JsonValue {
    /// Create a new null value.
    pub fn null() -> Self {
        JsonValue::Null
    }

    /// Create a new boolean value.
    pub fn bool(value: bool) -> Self {
        JsonValue::Bool(value)
    }

    /// Create a new number value.
    pub fn number(value: f64) -> Self {
        JsonValue::Number(value)
    }

    /// Create a new string value.
    pub fn string(value: String) -> Self {
        JsonValue::String(value)
    }

    /// Create a new array value.
    pub fn array(value: Vec<JsonValue>) -> Self {
        JsonValue::Array(value)
    }

    /// Create a new object value.
    pub fn object(value: std::collections::HashMap<String, JsonValue>) -> Self {
        JsonValue::Object(value)
    }

    /// Check if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    /// Check if this value is a boolean.
    pub fn is_bool(&self) -> bool {
        matches!(self, JsonValue::Bool(_))
    }

    /// Check if this value is a number.
    pub fn is_number(&self) -> bool {
        matches!(self, JsonValue::Number(_))
    }

    /// Check if this value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, JsonValue::String(_))
    }

    /// Check if this value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, JsonValue::Array(_))
    }

    /// Check if this value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, JsonValue::Object(_))
    }

    /// Get the boolean value if this is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get the number value if this is a number.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Get the string value if this is a string.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get the array value if this is an array.
    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get the object value if this is an object.
    pub fn as_object(&self) -> Option<&std::collections::HashMap<String, JsonValue>> {
        match self {
            JsonValue::Object(obj) => Some(obj),
            _ => None,
        }
    }
}

impl fmt::Debug for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = format_json(self, &FormatOptions::compact());
        f.write_str(&s)
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            // pretty when used with {:#}
            let s = format_json(self, &FormatOptions::pretty(2));
            f.write_str(&s)
        } else {
            let s = format_json(self, &FormatOptions::compact());
            f.write_str(&s)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::FormatOptions;
    use crate::parse::parse_json;

    #[test]
    fn display_and_debug_compact() {
        let v = JsonValue::array(vec![JsonValue::number(1.0), JsonValue::bool(false)]);
        assert_eq!(format!("{v}"), "[1,false]");
        assert_eq!(format!("{v:?}"), "[1,false]");
    }

    #[test]
    fn roundtrip() {
        let s = "{\"a\":[1,2,3],\"b\":true,\"c\":null,\"d\":\"x\"}";
        let v = parse_json(s).unwrap();
        let s2 = format_json(&v, &FormatOptions::compact());
        assert_eq!(parse_json(&s2).unwrap(), v);
    }
}
