use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_export_records_default_function() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
export default function main(): number {
    return 1;
}
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_export()),
        r#"
export default function main(): number {
/// @export.local key=<default> source=main

    return 1;
}

/// @export.summary exports=1 stars=0
"#,
    );
}

#[test]
fn test_export_records_default_expression() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
export default 1;
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_export()),
        r#"
export default 1;
/// @export.local key=<default> source=symbol1

/// @export.summary exports=1 stars=0
"#,
    );
}
