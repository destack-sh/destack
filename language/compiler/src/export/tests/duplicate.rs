use crate::tests::TestSession;

#[test]
fn test_export_reports_duplicate_key() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export let value = 1;
export { value };
"#,
        )
        .build();
    compiler.assert_dir_exported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=duplicate-export message="duplicate export 'value'"
/// @diagnostic.label line=3 column=10 span="value" line_source="export { value };"
"#,
    );
}

#[test]
fn test_export_reports_missing_local_binding() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export { missing };
"#,
        )
        .build();
    compiler.assert_dir_exported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=missing-export-binding message="missing exported local binding 'missing'"
/// @diagnostic.label line=2 column=10 span="missing" line_source="export { missing };"
"#,
    );
}

#[test]
fn test_export_reports_global_default_key_reexport() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
global {
    export { value as default } from "./dep.tspp";
}
"#,
        )
        .module(
            "dep.tspp",
            r#"
export const value = 1;
"#,
        )
        .build();
    compiler.assert_dir_exported_diagnostics(
        "main.tspp", r#"
/// @diagnostic.error id=default-global-export message="global export cannot use default key"
/// @diagnostic.label line=3 column=14 span="value as default" line_source="export { value as default } from \"./dep.tspp\";"
"#,
    );
}

#[test]
fn test_export_reports_global_namespace_reexport() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
global {
    export * from "./dep.tspp";
}
"#,
        )
        .module(
            "dep.tspp",
            r#"
export const value = 1;
"#,
        )
        .build();
    compiler.assert_dir_exported_diagnostics(
        "main.tspp", r#"
/// @diagnostic.error id=namespace-global-export message="global namespace export requires an alias"
/// @diagnostic.label line=3 column=12 span="* from \"./dep.tspp\"" line_source="export * from \"./dep.tspp\";"
"#,
    );
}
