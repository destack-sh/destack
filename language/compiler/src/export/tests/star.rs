use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_star_binding() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export * from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::modules()
            .with_export()
            .with_summaries()
            .with_export_stats(),
        r#"
export * from "./dep.ds";
/// @module.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.star module=dep.ds

/// @module.summary edges=1
/// @export.summary stars=1
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:1
"#,
    );
}

#[test]
fn test_export_records_namespace_binding() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export * as dep from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::modules()
            .with_export()
            .with_summaries()
            .with_export_stats(),
        r#"
export * as dep from "./dep.ds";
/// @module.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.indirect key=dep imported=<namespace> module=dep.ds

/// @module.summary edges=1
/// @export.summary exports=1
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:1
"#,
    );
}
