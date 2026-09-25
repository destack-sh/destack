use tspp_source::FileId;

use crate::{JsonFormatOptions, format_json, parse};

/// Formats a simple object to a single line.
#[test]
fn test_format_simple_object() {
    let doc = parse(r#"{"name":"test","value":42}"#, FileId::new(0)).unwrap();
    let output = format_json(&doc, &JsonFormatOptions::default()).unwrap();

    assert_eq!(output, r#"{"name": "test", "value": 42}"#);
}

/// Formats an array to a single line.
#[test]
fn test_format_array() {
    let doc = parse(r#"[1,2,3,"hello"]"#, FileId::new(0)).unwrap();
    let output = format_json(&doc, &JsonFormatOptions::default()).unwrap();

    assert_eq!(output, r#"[1, 2, 3, "hello"]"#);
}

/// Formats an empty object.
#[test]
fn test_format_empty_object() {
    let doc = parse(r#"{}"#, FileId::new(0)).unwrap();
    let output = format_json(&doc, &JsonFormatOptions::default()).unwrap();

    assert_eq!(output, "{}");
}

/// Formats an empty array.
#[test]
fn test_format_empty_array() {
    let doc = parse(r#"[]"#, FileId::new(0)).unwrap();
    let output = format_json(&doc, &JsonFormatOptions::default()).unwrap();

    assert_eq!(output, "[]");
}

/// Formats with trailing commas when enabled.
#[test]
fn test_format_trailing_comma() {
    let doc = parse(r#"{"a": 1}"#, FileId::new(0)).unwrap();

    // without trailing comma (default)
    let output = format_json(&doc, &JsonFormatOptions::default()).unwrap();
    assert_eq!(output, r#"{"a": 1}"#);

    // with trailing comma (only appears when expanded)
    let options = JsonFormatOptions::default().with_trailing_comma(true);
    let output = format_json(&doc, &options).unwrap();

    // single property stays on one line, so no trailing comma visible
    assert_eq!(output, r#"{"a": 1}"#);
}

/// Formats string escapes correctly.
#[test]
fn test_format_string_escapes() {
    let doc = parse(r#"{"text": "line1\nline2"}"#, FileId::new(0)).unwrap();
    let output = format_json(&doc, &JsonFormatOptions::default()).unwrap();

    assert_eq!(output, r#"{"text": "line1\nline2"}"#);
}
