use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_records_loader_attribute() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import data from "./data.json" with { type: "json" };
"#,
        )
        .data(
            "data.json",
            r#"
{
    "ok": true
}
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import data from "./data.json" with { type: "json" };
/// @module.edge relation=import specifier=./data.json loader=json module=data.json

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_applies_loader_extension() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import data from "./data" with { type: "json" };
"#,
        )
        .data(
            "data.json",
            r#"
{
    "ok": true
}
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import data from "./data" with { type: "json" };
/// @module.edge relation=import specifier=./data loader=json module=data.json

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_reports_invalid_loader_attribute() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import data from "./data.json" with { type: true };
"#,
        )
        .data(
            "data.json",
            r#"
{
    "ok": true
}
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp", r#"
/// @diagnostic.error id=invalid-import-attribute-type message="invalid import attribute type '<non-string>'"
/// @diagnostic.label line=2 column=1 span="import data from \"./data.json\" with { type: true }" line_source="import data from \"./data.json\" with { type: true };"
"#,
    );
}

#[test]
fn test_import_reports_unknown_loader_attribute() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import data from "./data.json" with { type: "xml" };
"#,
        )
        .data(
            "data.json",
            r#"
{
    "ok": true
}
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp", r#"
/// @diagnostic.error id=invalid-import-attribute-type message="invalid import attribute type 'xml'"
/// @diagnostic.label line=2 column=1 span="import data from \"./data.json\" with { type: \"xml\" }" line_source="import data from \"./data.json\" with { type: \"xml\" };"
"#,
    );
}
