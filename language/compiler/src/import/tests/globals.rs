use crate::tests::TestCompiler;
use crate::tests::snapshot::assert_snapshot;

#[test]
fn test_import_records_configured_globals() {
    let compiler = TestCompiler::new()
        .global(
            "core/global.ds",
            r#"
global {
    let process: Process;
}
"#,
        )
        .module(
            "main.ds",
            r#"
let value = 1;
"#,
        )
        .build();

    assert_snapshot(
        compiler.global_environment_snapshot(),
        r#"
/// @global.module path=core/global.ds
/// @global.summary modules=1
"#,
    );
}
