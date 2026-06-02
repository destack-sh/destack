use crate::tests::TestSession;

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
    compiler.assert_dir_exported_diagnostics(
        "main.ds",
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
    compiler.assert_dir_exported_diagnostics(
        "main.ds",
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
    compiler.assert_dir_exported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=ET100 message="missing exported local binding 'missing'"
/// @diagnostic.label line=2 column=10 source="export { missing };"
"#,
    );
}

#[test]
fn test_export_reports_global_default_key_reexport() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
global {
    export { value as default } from "./dep.ds";
}
"#,
        )
        .module(
            "dep.ds",
            r#"
export const value = 1;
"#,
        )
        .build();
    compiler.assert_dir_exported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=ET102 message="global export cannot use default key"
/// @diagnostic.label line=3 column=14 source="export { value as default } from \"./dep.ds\";"
"#,
    );
}

#[test]
fn test_export_reports_global_namespace_reexport() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
global {
    export * from "./dep.ds";
}
"#,
        )
        .module(
            "dep.ds",
            r#"
export const value = 1;
"#,
        )
        .build();
    compiler.assert_dir_exported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=ET107 message="global namespace export requires an alias"
/// @diagnostic.label line=3 column=5 source="export * from \"./dep.ds\";"
"#,
    );
}
