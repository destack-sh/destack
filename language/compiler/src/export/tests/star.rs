use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_star_binding() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export * from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::modules().with_export().with_summaries(),
        r#"
export * from "./dep.tspp";
/// @module.edge relation=re_export specifier=./dep.tspp module=dep.tspp
/// @export.star module=dep.tspp

/// @module.summary edges=1
/// @export.summary stars=1
"#,
    );
}

#[test]
fn test_export_records_namespace_binding() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export * as dep from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::modules().with_export().with_summaries(),
        r#"
export * as dep from "./dep.tspp";
/// @module.edge relation=re_export specifier=./dep.tspp module=dep.tspp
/// @export.reexport key=dep imported=<namespace> declaration=dep module=dep.tspp

/// @module.summary edges=1
/// @export.summary exports=1
"#,
    );
}
