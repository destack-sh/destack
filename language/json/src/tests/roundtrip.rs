use tspp_source::FileId;

use crate::{JsonFormatOptions, format_json, from_serde, parse, to_serde};

/// Roundtrips parsing and formatting for primitives.
#[test]
fn test_roundtrip_primitives() {
    let options = JsonFormatOptions::default();

    let cases = [
        ("null", "null"),
        ("true", "true"),
        ("false", "false"),
        ("42", "42"),
        ("-3.14", "-3.14"),
        (r#""hello""#, r#""hello""#),
    ];

    for (input, expected) in cases {
        let doc = parse(input, FileId::new(0)).unwrap();
        let output = format_json(&doc, &options).unwrap();
        assert_eq!(output, expected, "roundtrip failed for: {input}");
    }
}

/// Roundtrips a complex nested structure.
#[test]
fn test_roundtrip_complex() {
    let input = r#"{"users": [{"id": 1, "name": "alice"}, {"id": 2, "name": "bob"}], "meta": {"total": 2}}"#;

    let doc = parse(input, FileId::new(0)).unwrap();
    let output = format_json(&doc, &JsonFormatOptions::default()).unwrap();

    // parse again and compare serde values
    let doc2 = parse(&output, FileId::new(0)).unwrap();

    let value1 = to_serde(&doc);
    let value2 = to_serde(&doc2);

    assert_eq!(value1, value2);
}

/// Roundtrips through serde_json and back.
#[test]
fn test_serde_roundtrip() {
    let input = r#"{"name": "test", "values": [1, 2, 3], "nested": {"flag": true}}"#;

    // parse -> to_serde -> from_serde -> format
    let doc1 = parse(input, FileId::new(0)).unwrap();
    let serde_value = to_serde(&doc1);
    let doc2 = from_serde(&serde_value, FileId::new(0));
    let output = format_json(&doc2, &JsonFormatOptions::default()).unwrap();

    // parse again and verify structure matches
    let doc3 = parse(&output, FileId::new(0)).unwrap();
    let serde_value2 = to_serde(&doc3);

    assert_eq!(serde_value, serde_value2);
}

/// Roundtrips empty containers.
#[test]
fn test_roundtrip_empty() {
    let options = JsonFormatOptions::default();

    // empty object
    let doc = parse("{}", FileId::new(0)).unwrap();
    assert_eq!(format_json(&doc, &options).unwrap(), "{}");

    // empty array
    let doc = parse("[]", FileId::new(0)).unwrap();
    assert_eq!(format_json(&doc, &options).unwrap(), "[]");
}

/// Roundtrips nested empty containers.
#[test]
fn test_roundtrip_nested_empty() {
    let input = r#"{"empty_obj": {}, "empty_arr": [], "nested": {"also_empty": []}}"#;

    let doc = parse(input, FileId::new(0)).unwrap();
    let output = format_json(&doc, &JsonFormatOptions::default()).unwrap();

    // verify serde values match
    let doc2 = parse(&output, FileId::new(0)).unwrap();
    assert_eq!(to_serde(&doc), to_serde(&doc2));
}
