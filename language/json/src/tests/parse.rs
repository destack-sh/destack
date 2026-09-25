use tspp_source::FileId;

use crate::{JsonValue, parse};

/// Parses a simple object with string and number values.
#[test]
fn test_parse_simple_object() {
    let doc = parse(r#"{"name": "test", "value": 42}"#, FileId::new(0)).unwrap();

    let JsonValue::Object { properties, .. } = &doc.value else {
        panic!("expected object");
    };

    assert_eq!(properties.len(), 2);
    assert_eq!(properties[0].key.value, "name");
    assert_eq!(properties[1].key.value, "value");

    let JsonValue::String { value, .. } = &properties[0].value else {
        panic!("expected string");
    };
    assert_eq!(value, "test");

    let JsonValue::Number { raw, .. } = &properties[1].value else {
        panic!("expected number");
    };
    assert_eq!(raw, "42");
}

/// Parses an array with mixed value types.
#[test]
fn test_parse_array() {
    let doc = parse(r#"[1, 2, 3, "hello", true, null]"#, FileId::new(0)).unwrap();

    let JsonValue::Array { elements, .. } = &doc.value else {
        panic!("expected array");
    };

    assert_eq!(elements.len(), 6);

    // verify each element type
    assert!(matches!(&elements[0].value, JsonValue::Number { raw, .. } if raw == "1"));
    assert!(matches!(&elements[1].value, JsonValue::Number { raw, .. } if raw == "2"));
    assert!(matches!(&elements[2].value, JsonValue::Number { raw, .. } if raw == "3"));
    assert!(matches!(&elements[3].value, JsonValue::String { value, .. } if value == "hello"));
    assert!(matches!(
        &elements[4].value,
        JsonValue::Bool { value: true, .. }
    ));
    assert!(matches!(&elements[5].value, JsonValue::Null { .. }));
}

/// Parses JSONC with line comments.
#[test]
fn test_parse_jsonc_line_comments() {
    let doc = parse(
        r#"{
        // this is a comment
        "key": "value"
    }"#,
        FileId::new(0),
    )
    .unwrap();

    let JsonValue::Object { properties, .. } = &doc.value else {
        panic!("expected object");
    };

    assert_eq!(properties.len(), 1);
    assert_eq!(properties[0].key.value, "key");

    // verify trivia was captured
    assert!(!properties[0].trivia_before.is_empty());
}

/// Parses JSONC with block comments.
#[test]
fn test_parse_jsonc_block_comments() {
    let doc = parse(r#"{"key": /* inline comment */ "value"}"#, FileId::new(0)).unwrap();

    let JsonValue::Object { properties, .. } = &doc.value else {
        panic!("expected object");
    };

    assert_eq!(properties.len(), 1);
    assert_eq!(properties[0].key.value, "key");
}

/// Parses string escape sequences correctly.
#[test]
fn test_parse_string_escapes() {
    let doc = parse(
        r#"{"text": "line1\nline2\ttab\\slash\"quote"}"#,
        FileId::new(0),
    )
    .unwrap();

    let JsonValue::Object { properties, .. } = &doc.value else {
        panic!("expected object");
    };

    let JsonValue::String { value, .. } = &properties[0].value else {
        panic!("expected string");
    };

    assert_eq!(value, "line1\nline2\ttab\\slash\"quote");
}

/// Parses unicode escape sequences.
#[test]
fn test_parse_unicode_escapes() {
    let doc = parse(
        r#"{"emoji": "\u0048\u0065\u006c\u006c\u006f"}"#,
        FileId::new(0),
    )
    .unwrap();

    let JsonValue::Object { properties, .. } = &doc.value else {
        panic!("expected object");
    };

    let JsonValue::String { value, .. } = &properties[0].value else {
        panic!("expected string");
    };

    assert_eq!(value, "Hello");
}

/// Parses various number formats.
#[test]
fn test_parse_numbers() {
    let inputs = [
        ("42", "42"),
        ("-42", "-42"),
        ("3.14", "3.14"),
        ("1e10", "1e10"),
        ("1E10", "1E10"),
        ("1.5e-3", "1.5e-3"),
        ("-0.5E+2", "-0.5E+2"),
    ];

    let file_id = FileId::new(0);

    for (input, expected) in inputs {
        let doc = parse(input, file_id).unwrap();

        let JsonValue::Number { raw, .. } = &doc.value else {
            panic!("expected number for input: {input}");
        };

        assert_eq!(raw, expected, "mismatch for input: {input}");
    }
}

/// Parses nested structures.
#[test]
fn test_parse_nested() {
    let doc = parse(
        r#"{"outer": {"inner": [1, 2, {"deep": true}]}}"#,
        FileId::new(0),
    )
    .unwrap();

    let JsonValue::Object { properties, .. } = &doc.value else {
        panic!("expected object");
    };

    assert_eq!(properties[0].key.value, "outer");

    let JsonValue::Object {
        properties: inner_props,
        ..
    } = &properties[0].value
    else {
        panic!("expected nested object");
    };

    assert_eq!(inner_props[0].key.value, "inner");

    let JsonValue::Array { elements, .. } = &inner_props[0].value else {
        panic!("expected array");
    };

    assert_eq!(elements.len(), 3);
}

/// Returns an error for invalid JSON.
#[test]
fn test_parse_error_invalid() {
    let file_id = FileId::new(0);

    // unterminated string
    let result = parse(r#"{"key": "unterminated"#, file_id);
    assert!(result.is_err());

    // invalid token
    let result = parse(r#"{"key": undefined}"#, file_id);
    assert!(result.is_err());

    // missing colon
    let result = parse(r#"{"key" "value"}"#, file_id);
    assert!(result.is_err());
}
