use crate::JsonValue;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FormatOptions {
    pub indent: usize,
}

impl FormatOptions {
    #[inline]
    pub const fn compact() -> Self {
        Self { indent: 0 }
    }

    #[inline]
    pub const fn pretty(indent: usize) -> Self {
        Self { indent }
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self::compact()
    }
}

/// Format a `JsonValue` into a JSON string.
#[inline]
pub fn format_json(value: &JsonValue, options: &FormatOptions) -> String {
    let mut out = String::new();
    write_value(value, &mut out, 0, options).unwrap_or_else(|_| panic!("write to String failed"));
    out
}

#[inline]
fn write_value(
    value: &JsonValue,
    out: &mut String,
    depth: usize,
    options: &FormatOptions,
) -> std::fmt::Result {
    match value {
        JsonValue::Null => out.write_str("null"),
        JsonValue::Bool(b) => {
            if *b {
                out.write_str("true")
            } else {
                out.write_str("false")
            }
        }
        JsonValue::Number(n) => {
            if n.is_finite() {
                // format numbers without trailing .0 if possible
                if n.fract() == 0.0 {
                    // use integer-like formatting when possible
                    let int_val = *n as i64;
                    if (int_val as f64) == *n {
                        return write!(out, "{int_val}");
                    }
                }
                write!(out, "{n}")
            } else {
                // JSON does not support NaN/Infinity; encode as null
                out.write_str("null")
            }
        }
        JsonValue::String(s) => write_string(s, out),
        JsonValue::Array(arr) => write_array(arr, out, depth, options),
        JsonValue::Object(obj) => write_object(obj, out, depth, options),
    }
}

#[inline]
fn write_indent(out: &mut String, count: usize) -> std::fmt::Result {
    for _ in 0..count {
        out.push(' ');
    }
    Ok(())
}

#[inline]
fn write_array(
    arr: &[JsonValue],
    out: &mut String,
    depth: usize,
    options: &FormatOptions,
) -> std::fmt::Result {
    out.push('[');
    if arr.is_empty() {
        out.push(']');
        return Ok(());
    }

    if options.indent == 0 {
        for (i, item) in arr.iter().enumerate() {
            if i != 0 {
                out.push(',');
            }
            write_value(item, out, depth, options)?;
        }
    } else {
        let new_depth = depth + options.indent;
        for (i, item) in arr.iter().enumerate() {
            out.push('\n');
            write_indent(out, new_depth)?;
            write_value(item, out, new_depth, options)?;
            if i + 1 != arr.len() {
                out.push(',');
            }
        }
        out.push('\n');
        write_indent(out, depth)?;
    }

    out.push(']');
    Ok(())
}

#[inline]
fn write_object(
    obj: &std::collections::HashMap<String, JsonValue>,
    out: &mut String,
    depth: usize,
    options: &FormatOptions,
) -> std::fmt::Result {
    out.push('{');
    if obj.is_empty() {
        out.push('}');
        return Ok(());
    }

    // For stable output, iterate keys in sorted order
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();

    if options.indent == 0 {
        for (i, key) in keys.iter().enumerate() {
            if i != 0 {
                out.push(',');
            }
            write_string(key, out)?;
            out.push(':');
            write_value(&obj[*key], out, depth, options)?;
        }
    } else {
        let new_depth = depth + options.indent;
        for (i, key) in keys.iter().enumerate() {
            out.push('\n');
            write_indent(out, new_depth)?;
            write_string(key, out)?;
            out.push_str(": ");
            write_value(&obj[*key], out, new_depth, options)?;
            if i + 1 != keys.len() {
                out.push(',');
            }
        }
        out.push('\n');
        write_indent(out, depth)?;
    }

    out.push('}');
    Ok(())
}

#[inline]
fn write_string(s: &str, out: &mut String) -> std::fmt::Result {
    out.push('"');
    let bytes = s.as_bytes();
    let mut start = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        let esc = match b {
            b'"' => Some("\""),
            b'\\' => Some("\\\\"),
            b'\n' => Some("\\n"),
            b'\r' => Some("\\r"),
            b'\t' => Some("\\t"),
            0x00..=0x1F => Some("u"), // control characters -> \uXXXX
            _ => None,
        };
        if let Some(esc_seq) = esc {
            // write slice before escape
            if start < i {
                out.push_str(&s[start..i]);
            }
            if esc_seq == "u" {
                write!(out, "\\u{:04X}", b as u16)?;
            } else {
                out.push_str(esc_seq);
            }
            start = i + 1;
        }
    }
    if start < s.len() {
        out.push_str(&s[start..]);
    }
    out.push('"');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsonValue as J;
    use std::collections::HashMap;

    #[test]
    fn format_scalars_compact() {
        assert_eq!(format_json(&J::Null, &FormatOptions::compact()), "null");
        assert_eq!(
            format_json(&J::bool(true), &FormatOptions::compact()),
            "true"
        );
        assert_eq!(
            format_json(&J::bool(false), &FormatOptions::compact()),
            "false"
        );
        assert_eq!(format_json(&J::number(1.0), &FormatOptions::compact()), "1");
        assert_eq!(
            format_json(&J::string("a".into()), &FormatOptions::compact()),
            "\"a\""
        );
    }

    #[test]
    fn format_arrays_and_objects_compact() {
        let arr = J::array(vec![J::number(1.0), J::number(2.0), J::number(3.5)]);
        assert_eq!(format_json(&arr, &FormatOptions::compact()), "[1,2,3.5]");

        let mut m = HashMap::new();
        m.insert("b".to_string(), J::number(2.0));
        m.insert("a".to_string(), J::string("x".into()));
        let obj = J::object(m);
        let s = format_json(&obj, &FormatOptions::compact());
        // keys sorted -> a then b
        assert_eq!(s, "{\"a\":\"x\",\"b\":2}");
    }

    #[test]
    fn format_pretty() {
        let mut m = HashMap::new();
        m.insert(
            "a".to_string(),
            J::array(vec![J::number(1.0), J::number(2.0)]),
        );
        m.insert("b".to_string(), J::Null);
        let v = J::object(m);
        let pretty = format_json(&v, &FormatOptions::pretty(2));
        // simple snapshot
        assert!(pretty.starts_with("{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": null\n}"));
    }

    #[test]
    fn escape_controls() {
        let s = J::string("\t\n\r\u{0007}\\\"".into());
        let out = format_json(&s, &FormatOptions::compact());
        assert_eq!(out, "\"\\t\\n\\r\\u0007\\\\\"\"");
    }
}
