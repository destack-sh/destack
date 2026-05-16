use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_import_records_loader_attribute() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
import data from "./data.json" with { type: "json" };
"#,
        )
        .data("data.json", r#"{ "ok": true }"#)
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_dependency()),
        r#"
import data from "./data.json" with { type: "json" };
/// @dependency.edge relation=import specifier=./data.json loader=json module=data.json

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_applies_loader_extension() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
import data from "./data" with { type: "json" };
"#,
        )
        .data("data.json", r#"{ "ok": true }"#)
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_dependency()),
        r#"
import data from "./data" with { type: "json" };
/// @dependency.edge relation=import specifier=./data loader=json module=data.json

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_reports_invalid_loader_attribute() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
import data from "./data.json" with { type: true };
"#,
        )
        .data("data.json", r#"{ "ok": true }"#)
        .build();

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI203 message="invalid import attribute type '<non-string>'"
/// @diagnostic.label line=2 column=1 source="import data from \"./data.json\" with { type: true };"
"#,
    );
}

#[test]
fn test_import_reports_unknown_loader_attribute() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
import data from "./data.json" with { type: "xml" };
"#,
        )
        .data("data.json", r#"{ "ok": true }"#)
        .build();

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI203 message="invalid import attribute type 'xml'"
/// @diagnostic.label line=2 column=1 source="import data from \"./data.json\" with { type: \"xml\" };"
"#,
    );
}
