use tspp_fir::format::FormatResult;
use tspp_fir::prelude::*;

use crate::JsonValue;

use super::{JsonFormatContext, JsonFormatter, format_array, format_object};

/// Format a JSON value.
pub fn format_value(value: &JsonValue, f: &mut JsonFormatter<'_, '_>) -> FormatResult<()> {
    match value {
        JsonValue::Null { .. } => {
            token("null").format(f)?;
        }

        JsonValue::Bool { value, .. } => {
            let keyword = if *value { "true" } else { "false" };
            token(keyword).format(f)?;
        }

        JsonValue::Number { raw, .. } => {
            copied_text(raw).format(f)?;
        }

        JsonValue::String { value, .. } => {
            format_string(value, f)?;
        }

        JsonValue::Array { elements, .. } => {
            format_array(elements, f)?;
        }

        JsonValue::Object { properties, .. } => {
            format_object(properties, f)?;
        }
    }

    Ok(())
}

/// Format a JSON string with proper escaping.
pub fn format_string<'a>(value: &str, f: &mut JsonFormatter<'_, 'a>) -> FormatResult<()> {
    token("\"").format(f)?;

    // escape the string content
    let escaped = escape_json_string(value);
    copied_text(&escaped).format(f)?;

    token("\"").format(f)?;

    Ok(())
}

/// Escape a string for JSON output.
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0c' => result.push_str("\\f"),
            // control characters
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }

    result
}

impl<'a> Format<'a, JsonFormatContext> for JsonValue {
    fn format(&self, f: &mut JsonFormatter<'_, 'a>) -> FormatResult<()> {
        format_value(self, f)
    }
}
