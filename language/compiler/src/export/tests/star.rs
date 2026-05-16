use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_export_records_star_binding() {
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshot(
            "main.ds",
            DirSnapshotSet::none().with_dependency().with_export(),
        ),
        r#"
export * from "./dep.ds";
/// @dependency.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.star module=dep.ds

/// @dependency.summary edges=1
/// @export.summary exports=0 stars=1
"#,
    );
}

#[test]
fn test_export_records_namespace_binding() {
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshot(
            "main.ds",
            DirSnapshotSet::none().with_dependency().with_export(),
        ),
        r#"
export * as dep from "./dep.ds";
/// @dependency.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.indirect key=dep import=<namespace> module=dep.ds

/// @dependency.summary edges=1
/// @export.summary exports=1 stars=0
"#,
    );
}
