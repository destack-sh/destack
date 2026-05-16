use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_export_records_local_value() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
export let value: number = 1;
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_export()),
        r#"
export let value: number = 1;
/// @export.local key=value source=value

/// @export.summary exports=1 stars=0
"#,
    );
}

#[test]
fn test_export_records_local_alias() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
let value = 1;
export { value as renamed };
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_export()),
        r#"
let value = 1;
export { value as renamed };
/// @export.local key=renamed source=value

/// @export.summary exports=1 stars=0
"#,
    );
}

#[test]
fn test_export_uses_latest_local_binding() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
let value = 1;
let value = 2;
export { value };
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_export()),
        r#"
let value = 1;
let value = 2;
export { value };
/// @export.local key=value source=value#2

/// @export.summary exports=1 stars=0
"#,
    );
}
