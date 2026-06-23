use serde::Serialize;
use serde_json::{Value, json};

use crate::common::report::REPORT_SCHEMA_VERSION;
use crate::common::{CommandError, CommandReport, grouped_list_payload, list_payload};

/// Minimal list entry for payload tests.
#[derive(Serialize)]
struct TestEntry {
    /// Entry identifier.
    id: u32,
}

/// Minimal grouped payload for payload tests.
#[derive(Serialize)]
struct TestGroup {
    /// Group label.
    category: String,
    /// Group entries.
    entries: Vec<TestEntry>,
}

/// Serializes a success report with the expected envelope.
#[test]
fn test_command_report_serializes_success() {
    // build a report with data
    let mut report = CommandReport::success("report", 0);
    report.summary = Some("ok".to_string());
    report.data = Some(json!({ "answer": 42 }));

    // serialize to json
    let value = serde_json::to_value(&report).expect("report should serialize");
    let report = value.as_object().expect("report should be a json object");

    // validate core fields
    assert_eq!(
        report.get("schema_version").and_then(Value::as_u64),
        Some(REPORT_SCHEMA_VERSION as u64),
    );
    assert_eq!(
        report.get("command").and_then(Value::as_str),
        Some("report")
    );
    assert_eq!(
        report.get("status").and_then(Value::as_str),
        Some("success"),
    );
    assert_eq!(report.get("exit_code").and_then(Value::as_i64), Some(0));
    assert!(report.get("data").is_some());
}

/// Serializes a failure report with an error payload.
#[test]
fn test_command_report_serializes_failure() {
    // build a failure report with a structured error
    let mut report = CommandReport::failure("report", 1);
    report.summary = Some("failed".to_string());
    report.error = Some(CommandError::new("bad", "cli", "boom"));

    // serialize to json
    let value = serde_json::to_value(&report).expect("report should serialize");
    let report = value.as_object().expect("report should be a json object");

    // validate core fields
    assert_eq!(
        report.get("command").and_then(Value::as_str),
        Some("report")
    );
    assert_eq!(
        report.get("status").and_then(Value::as_str),
        Some("failure"),
    );
    let error = report.get("error").expect("report should include error");
    assert_object_keys(error, &["code", "kind", "message"]);
}

/// Builds list payloads with a total count.
#[test]
fn test_list_payload_includes_total() {
    // build a list payload
    let payload = list_payload(vec![TestEntry { id: 1 }, TestEntry { id: 2 }]);

    // validate payload shape
    let payload = payload
        .as_object()
        .expect("payload should be a json object");
    assert_eq!(payload.get("total").and_then(Value::as_u64), Some(2));
    let items = payload
        .get("items")
        .and_then(Value::as_array)
        .expect("payload should include items");
    assert_eq!(items.len(), 2);
}

/// Builds grouped list payloads with totals.
#[test]
fn test_grouped_list_payload_includes_totals() {
    // build grouped payload data
    let groups = vec![
        TestGroup {
            category: "alpha".to_string(),
            entries: vec![TestEntry { id: 1 }],
        },
        TestGroup {
            category: "beta".to_string(),
            entries: vec![TestEntry { id: 2 }, TestEntry { id: 3 }],
        },
    ];

    // build grouped payload
    let payload = grouped_list_payload(groups, 3);

    // validate payload shape
    let payload = payload
        .as_object()
        .expect("payload should be a json object");
    assert_eq!(payload.get("total_groups").and_then(Value::as_u64), Some(2));
    assert_eq!(payload.get("total_items").and_then(Value::as_u64), Some(3));
    let items = payload
        .get("groups")
        .and_then(Value::as_array)
        .expect("payload should include groups");
    assert_eq!(items.len(), 2);
}

/// Assert a json object contains exactly the expected keys.
fn assert_object_keys<'a>(
    value: &'a Value,
    expected: &[&str],
) -> &'a serde_json::Map<String, Value> {
    let object = value.as_object().expect("value should be a json object");
    let mut actual: Vec<&str> = object.keys().map(|key| key.as_str()).collect();
    actual.sort_unstable();
    let mut expected = expected.to_vec();
    expected.sort_unstable();
    assert_eq!(actual, expected);
    object
}
