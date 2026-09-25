use tspp_dir as dir;
use tspp_source::{FileId, FileType, Span};

use crate::FileImage;
use crate::command::{
    QueryCapture, QueryCaptureValue, QueryMatch, QueryPayload, RewriteMode, RewritePayload,
};
use crate::tests::harness::TestPattern;

/// Manifest selecting one checked entry module.
const ENTRY_CONFIG: &str = r#"{
  "name": "test",
  "targets": {
    "default": {
      "entry": ["main.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;

/// Manifest selecting every authored module.
const INCLUDE_CONFIG: &str = r#"{
  "name": "test",
  "targets": {
    "default": {
      "include": ["**/*.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;

/// Return exact structural matches and captures in source order.
#[test]
fn test_query_source_pattern() {
    let source = "fetch(\"/a\");\nfetch(\"/b\", options);\n";
    let test = TestPattern::new("query-source-pattern").input("main.tspp", source);
    let output = test.query("fetch($URL, $$$ARGUMENTS)").run();
    let main = test.path("main.tspp");
    let file = FileId::from_logical_str("main.tspp");
    let uri = test.uri("main.tspp");

    assert_eq!(
        output.data,
        QueryPayload {
            captures: vec!["URL".to_string(), "ARGUMENTS".to_string()],
            matches: vec![
                QueryMatch {
                    uri: uri.clone(),
                    path: Some(main.clone()),
                    span: Span::new(file, 0, 11),
                    line: 1,
                    column: 1,
                    text: "fetch(\"/a\")".to_string(),
                    captures: vec![
                        QueryCapture {
                            name: "URL".to_string(),
                            values: vec![QueryCaptureValue {
                                span: Span::new(file, 6, 10),
                                text: "\"/a\"".to_string(),
                            }],
                        },
                        QueryCapture {
                            name: "ARGUMENTS".to_string(),
                            values: Vec::new(),
                        },
                    ],
                },
                QueryMatch {
                    uri,
                    path: Some(main),
                    span: Span::new(file, 13, 33),
                    line: 2,
                    column: 1,
                    text: "fetch(\"/b\", options)".to_string(),
                    captures: vec![
                        QueryCapture {
                            name: "URL".to_string(),
                            values: vec![QueryCaptureValue {
                                span: Span::new(file, 19, 23),
                                text: "\"/b\"".to_string(),
                            }],
                        },
                        QueryCapture {
                            name: "ARGUMENTS".to_string(),
                            values: vec![QueryCaptureValue {
                                span: Span::new(file, 25, 32),
                                text: "options".to_string(),
                            }],
                        },
                    ],
                },
            ],
        }
    );
    assert!(output.files.is_empty());
}

/// Keep query patterns outside the authored Builtin Package module index.
#[test]
fn test_query_authored_builtin_package() {
    let config_source = r#"{
  "name": "tspp",
  "targets": {
    "default": {
      "include": ["src/**/*.tspp"]
    }
  },
  "defaultTarget": "default"
}
"#;
    let source = "panic(\"failed\");\n";
    let test = TestPattern::new("query-authored-builtin-package")
        .file("destack.json", config_source)
        .input("src/main.tspp", source);
    let output = test.query("panic($MESSAGE)").include_sources().run();
    let matches = output
        .data
        .matches
        .iter()
        .map(|pattern_match| pattern_match.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(matches, ["panic(\"failed\")"]);
    assert_eq!(
        output.files,
        [FileImage {
            id: FileId::from_logical_str("src/main.tspp"),
            name: "main.tspp".to_string(),
            uri: test.uri("src/main.tspp"),
            path: Some(test.path("src/main.tspp")),
            file_type: FileType::Tspp,
            content: Some(source.to_string()),
        }]
    );
}

/// Order nested selections by source position instead of DIR allocation order.
#[test]
fn test_query_orders_nested_matches_by_source() {
    let source = "outer(inner(value));\n";
    let test = TestPattern::new("query-nested-source-order").input("main.tspp", source);
    let output = test.query("$CALLEE($VALUE)").run();
    let matches = output
        .data
        .matches
        .iter()
        .map(|pattern_match| pattern_match.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(matches, ["outer(inner(value))", "inner(value)"]);
}

/// Resolve qualified predicate symbols through the checked program closure.
#[test]
fn test_query_symbol_predicate() {
    let package_source = "export * as net from \"./net.tspp\";\n";
    let net_source = r#"
export function fetch(value: string): string {
    return value;
}
"#;
    let source = r#"
import * as myPackage from "./package.tspp";

function fetch(value: string): string {
    return value;
}

myPackage.net.fetch("first");
    fetch("second");
"#;
    let test = TestPattern::new("query-symbol-predicate")
        .file("destack.json", ENTRY_CONFIG)
        .file("package.tspp", package_source)
        .file("net.tspp", net_source)
        .input("main.tspp", source);
    let output = test
        .query("$CALLEE($VALUE)")
        .where_("$CALLEE == myPackage.net.fetch")
        .run();
    let matches = output
        .data
        .matches
        .iter()
        .map(|pattern_match| pattern_match.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(matches, ["myPackage.net.fetch(\"first\")"]);
}

/// Emit one complete unified diff without mutating source files.
#[test]
fn test_rewrite_source_pattern_diff() {
    let source = "fetch(\"/a\");\nkeep();\n";
    let test = TestPattern::new("rewrite-source-pattern-diff").input("main.tspp", source);
    let output = test
        .rewrite("fetch($URL)", "client.fetch($URL)")
        .mode(RewriteMode::Diff)
        .run();
    let text = output
        .output
        .iter()
        .map(|chunk| String::from_utf8(chunk.bytes.clone()).expect("utf8 rewrite diff"))
        .collect::<String>();

    assert_eq!(output.data.replacements, 1);
    assert_eq!(output.data.changes[0].patch.patches.len(), 1);
    assert_eq!(
        output.data.changes[0].patch.patches[0].new_text,
        "client.fetch(\"/a\")"
    );
    assert_eq!(
        text,
        "--- a/main.tspp\n+++ b/main.tspp\n\n-   1│ fetch(\"/a\");\n+   1│ client.fetch(\"/a\");\n    2│ keep();\n"
    );
    assert!(output.data.commit.is_none());
    assert_eq!(test.source("main.tspp"), source);
}

/// Fail check mode exactly when a rewrite would change source.
#[test]
fn test_rewrite_checks_required_changes() {
    let source = "fetch(\"/a\");\nkeep();\n";
    let test = TestPattern::new("rewrite-check").input("main.tspp", source);
    let required = test
        .rewrite("fetch($URL)", "client.fetch($URL)")
        .mode(RewriteMode::Check)
        .run();

    assert_eq!(required.exit_code, 1);
    assert_eq!(required.data.replacements, 1);
    assert_eq!(required.data.changes.len(), 1);
    assert!(required.data.commit.is_none());
    assert!(required.output.is_empty());
    assert_eq!(test.source("main.tspp"), source);

    let clean = test
        .rewrite("missing($URL)", "client.fetch($URL)")
        .mode(RewriteMode::Check)
        .run();

    assert_eq!(clean.exit_code, 0);
    assert_eq!(clean.data, RewritePayload::default());
    assert!(clean.output.is_empty());
}

/// Write every non-overlapping rewrite through the workspace file system.
#[test]
fn test_rewrite_source_pattern_write() {
    let source = "fetch(\"/a\");\nfetch(\"/b\");\n";
    let test = TestPattern::new("rewrite-source-pattern-write").input("main.tspp", source);
    let before = test.revision();

    let output = test
        .rewrite("fetch($URL)", "client.fetch($URL)")
        .mode(RewriteMode::Write)
        .run();

    assert_eq!(output.data.replacements, 2);
    assert_eq!(
        test.source("main.tspp"),
        "client.fetch(\"/a\");\nclient.fetch(\"/b\");\n"
    );
    let after = test.revision();
    let commit = output.data.commit.as_ref().expect("rewrite commit");
    assert_eq!(commit.before, before);
    assert_eq!(commit.after, after);
    let files = test
        .harness
        .workspace
        .read_files(after, vec![output.data.changes[0].patch.file])
        .expect("read rewritten workspace file");
    assert_ne!(after, before);
    assert_eq!(
        files[0].text(),
        "client.fetch(\"/a\");\nclient.fetch(\"/b\");\n"
    );
}

/// Select and rewrite contextual MatchArm roots through the same command model.
#[test]
fn test_query_and_rewrite_match_arm() {
    let source = "match (result) { Err(error) => recover(error); Ok(value) => value }\n";
    let test = TestPattern::new("query-rewrite-match-arm").input("main.tspp", source);
    let output = test
        .query("match (value) { Err($ERROR) => $BODY }")
        .kind(dir::NodeType::MatchArm)
        .run();
    assert_eq!(output.data.matches.len(), 1);
    assert_eq!(output.data.matches[0].text, "Err(error) => recover(error)");
    assert_eq!(
        output.data.matches[0]
            .captures
            .iter()
            .map(|capture| (capture.name.as_str(), capture.values[0].text.as_str()))
            .collect::<Vec<_>>(),
        [("ERROR", "error"), ("BODY", "recover(error)")]
    );

    let output = test
        .rewrite(
            "match (value) { Err($ERROR) => $BODY }",
            "match (value) { Err($ERROR) => log($BODY) }",
        )
        .kind(dir::NodeType::MatchArm)
        .mode(RewriteMode::Write)
        .run();

    assert_eq!(output.data.replacements, 1);
    assert_eq!(
        test.source("main.tspp"),
        "match (result) { Err(error) => log(recover(error)); Ok(value) => value }\n"
    );
}

/// Return complete Pattern diagnostics with their transient authored source.
#[test]
fn test_query_reports_pattern_diagnostics() {
    let source = "fetch(\"/a\");\n";
    let test = TestPattern::new("query-pattern-diagnostics").input("main.tspp", source);
    let output = test.query("$$$VALUES").run();

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["invalid-repeated-metavariable"]
    );
    assert_eq!(output.files.len(), 1);
    assert_eq!(output.files[0].name, "pattern.tspp-pattern");
    assert_eq!(output.files[0].content.as_deref(), Some("$$$VALUES"));
    assert!(output.data.matches.is_empty());
}

/// Discard valid file patches when any other selected file has overlapping matches.
#[test]
fn test_rewrite_discards_complete_change_set_after_overlap() {
    let first_source = "call(value);\n";
    let second_source = "outer(inner(value));\n";
    let test = TestPattern::new("rewrite-overlapping-files")
        .input("first.tspp", first_source)
        .input("second.tspp", second_source);
    let output = test
        .rewrite("$CALLEE($VALUE)", "wrap($CALLEE($VALUE))")
        .mode(RewriteMode::Write)
        .run();

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["overlapping-rewrites"]
    );
    assert_eq!(output.data.replacements, 0);
    assert!(output.data.changes.is_empty());
    assert!(output.output.is_empty());
    assert_eq!(test.source("first.tspp"), first_source);
    assert_eq!(test.source("second.tspp"), second_source);
}

/// Apply semantic symbol selection to rewrites as well as read-only queries.
#[test]
fn test_rewrite_symbol_predicate() {
    let source = r#"
function fetch(value: string): string {
    return value;
}

function send(value: string): string {
    return value;
}

fetch("first");
send("second");
"#;
    let test = TestPattern::new("rewrite-symbol-predicate")
        .file("destack.json", ENTRY_CONFIG)
        .input("main.tspp", source);
    let output = test
        .rewrite("$CALLEE($VALUE)", "client.fetch($VALUE)")
        .where_("$CALLEE == fetch")
        .mode(RewriteMode::Write)
        .run();

    assert_eq!(output.data.replacements, 1);
    assert_eq!(
        test.source("main.tspp"),
        r#"
function fetch(value: string): string {
    return value;
}

function send(value: string): string {
    return value;
}

client.fetch("first");
send("second");
"#
    );
}

/// Select calls through checked type assignability.
#[test]
fn test_query_type_predicate() {
    let source = r#"
function consume(value: string | int32): void {}

consume("text");
consume(42);
"#;
    let test = TestPattern::new("query-type-predicate")
        .file("destack.json", ENTRY_CONFIG)
        .input("main.tspp", source);
    let output = test
        .query("consume($VALUE)")
        .where_("$VALUE satisfies string")
        .run();
    let matches = output
        .data
        .matches
        .iter()
        .map(|pattern_match| pattern_match.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(matches, ["consume(\"text\")"]);
}

/// Check only modules containing structural matches before evaluating Predicates.
#[test]
fn test_query_and_rewrite_check_selected_modules() {
    let source = r#"
function consume(value: string): void {}

consume("text");
"#;
    let invalid = "const invalid: int32 = \"text\";\n";
    let test = TestPattern::new("pattern-check-selected-modules")
        .file("destack.json", INCLUDE_CONFIG)
        .input("main.tspp", source)
        .input("invalid.tspp", invalid);

    let query = test
        .query("consume($VALUE)")
        .where_("$VALUE satisfies string")
        .run();

    assert_eq!(query.exit_code, 0);
    assert!(query.diagnostics.is_empty());
    assert_eq!(query.data.matches.len(), 1);
    assert_eq!(query.data.matches[0].text, "consume(\"text\")");

    let rewrite = test
        .rewrite("consume($VALUE)", "inspect($VALUE)")
        .where_("$VALUE satisfies string")
        .mode(RewriteMode::Check)
        .run();

    assert_eq!(rewrite.exit_code, 1);
    assert!(rewrite.diagnostics.is_empty());
    assert_eq!(rewrite.data.replacements, 1);
    assert_eq!(test.source("invalid.tspp"), invalid);
}

/// Allow structural results and suppress semantic results over invalid checked source.
#[test]
fn test_distinguish_structural_and_semantic_invalid_programs() {
    let source = r#"
function fetch(value: string): string {
    return value;
}

const invalid: int32 = "text";
fetch("first");
"#;
    let test = TestPattern::new("pattern-invalid-checked-program")
        .file("destack.json", ENTRY_CONFIG)
        .input("main.tspp", source);
    let structural = test.query("fetch($VALUE)").run();

    assert_eq!(
        structural
            .data
            .matches
            .iter()
            .map(|pattern_match| pattern_match.text.as_str())
            .collect::<Vec<_>>(),
        ["fetch(\"first\")"]
    );

    let structural = test.rewrite("fetch($VALUE)", "client.fetch($VALUE)").run();

    assert_eq!(structural.exit_code, 0);
    assert_eq!(structural.data.replacements, 1);
    assert_eq!(
        structural.data.changes[0].patch.patches[0].new_text,
        "client.fetch(\"first\")"
    );

    let query = test
        .query("$CALLEE($VALUE)")
        .where_("$CALLEE == fetch")
        .run();

    assert_eq!(query.exit_code, 1);
    assert_eq!(
        query
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["not-assignable"]
    );
    assert!(query.data.matches.is_empty());

    let rewrite = test
        .rewrite("$CALLEE($VALUE)", "client.fetch($VALUE)")
        .where_("$CALLEE == fetch")
        .mode(RewriteMode::Write)
        .run();

    assert_eq!(rewrite.exit_code, 1);
    assert_eq!(
        rewrite
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["not-assignable"]
    );
    assert_eq!(rewrite.data.replacements, 0);
    assert!(rewrite.data.changes.is_empty());
    assert!(rewrite.output.is_empty());
    assert_eq!(test.source("main.tspp"), source);
}
