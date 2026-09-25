use tspp_source::FileId;

use crate::{JsonValue, from_serde, parse, to_serde};

/// Converts an object to serde_json.
#[test]
fn test_to_serde_object() {
    let doc = parse(
        r#"{"name": "test", "count": 123, "active": true}"#,
        FileId::new(0),
    )
    .unwrap();
    let serde_value = to_serde(&doc);

    assert!(serde_value.is_object());

    let obj = serde_value.as_object().unwrap();
    assert_eq!(obj.get("name").unwrap().as_str().unwrap(), "test");
    assert_eq!(obj.get("count").unwrap().as_i64().unwrap(), 123);
    assert!(obj.get("active").unwrap().as_bool().unwrap());
}

/// Converts an array to serde_json.
#[test]
fn test_to_serde_array() {
    let doc = parse(r#"[1, "two", true, null]"#, FileId::new(0)).unwrap();
    let serde_value = to_serde(&doc);

    assert!(serde_value.is_array());

    let arr = serde_value.as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0].as_i64().unwrap(), 1);
    assert_eq!(arr[1].as_str().unwrap(), "two");
    assert!(arr[2].as_bool().unwrap());
    assert!(arr[3].is_null());
}

/// Converts serde_json to AST.
#[test]
fn test_from_serde_object() {
    let serde_value = serde_json::json!({
        "name": "test",
        "items": [1, 2, 3]
    });

    let doc = from_serde(&serde_value, FileId::new(0));

    let JsonValue::Object { properties, .. } = &doc.value else {
        panic!("expected object");
    };

    assert_eq!(properties.len(), 2);
}
