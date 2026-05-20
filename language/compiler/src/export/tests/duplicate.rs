use crate::tests::TestSession;
use crate::tests::snapshot::assert_snapshot;

#[test]
fn test_export_reports_duplicate_key() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
export let value = 1;
export { value };
"#,
        )
        .build();

    compiler
        .provide_dir_exported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_exported_key("main.ds")),
        r#"
/// @diagnostic.error code=ET101 message="duplicate export 'value'"
/// @diagnostic.label line=3 column=10 source="export { value };"
"#,
    );
}

#[test]
fn test_export_merges_type_spelling_into_same_key() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
type Foo = string;
export { Foo };
export type { Foo };
"#,
        )
        .build();

    compiler
        .provide_dir_exported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_exported_key("main.ds")),
        r#"
/// @diagnostic.error code=ET101 message="duplicate export 'Foo'"
/// @diagnostic.label line=4 column=15 source="export type { Foo };"
"#,
    );
}

#[test]
fn test_export_reports_missing_local_binding() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
export { missing };
"#,
        )
        .build();

    compiler
        .provide_dir_exported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_exported_key("main.ds")),
        r#"
/// @diagnostic.error code=ET100 message="missing exported local binding 'missing'"
/// @diagnostic.label line=2 column=10 source="export { missing };"
"#,
    );
}
