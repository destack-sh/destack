use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_export_records_indirect_binding() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
export { Foo as Bar } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let Foo = 1;
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot(
            "main.ds",
            DirSnapshotSet::none().with_dependency().with_export(),
        ),
        r#"
export { Foo as Bar } from "./dep.ds";
/// @dependency.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.indirect key=Bar import=Foo module=dep.ds

/// @dependency.summary edges=1
/// @export.summary exports=1 stars=0
"#,
    );
}

#[test]
fn test_export_records_default_indirect_aliases() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
export { default as value, named as default } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export default 1;
export let named = 2;
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot(
            "main.ds",
            DirSnapshotSet::none().with_dependency().with_export(),
        ),
        r#"
export { default as value, named as default } from "./dep.ds";
/// @dependency.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.indirect key=value import=<default> module=dep.ds
/// @export.indirect key=<default> import=named module=dep.ds

/// @dependency.summary edges=1
/// @export.summary exports=2 stars=0
"#,
    );
}
