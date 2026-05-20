use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_star_binding() {
    let compiler = TestSession::new()
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
        DirRows::dependencies().with_export().with_summaries(),
        r#"
export * from "./dep.ds";
/// @dependency.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.star module=dep.ds

/// @dependency.summary edges=1
/// @export.summary stars=1
"#,
    );
}

#[test]
fn test_export_records_namespace_binding() {
    let compiler = TestSession::new()
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
        DirRows::dependencies().with_export().with_summaries(),
        r#"
export * as dep from "./dep.ds";
/// @dependency.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.indirect key=dep imported=<namespace> module=dep.ds

/// @dependency.summary edges=1
/// @export.summary exports=1
"#,
    );
}
