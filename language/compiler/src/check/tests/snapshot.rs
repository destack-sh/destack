use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

/// Build one checked DIR snapshot selection.
pub(super) fn check_snapshot_selection() -> DirSnapshotSet {
    DirSnapshotSet::none()
        .with_types()
        .with_generic()
        .with_relation()
        .with_extension()
        .with_resolution()
        .with_instance()
        .with_capture()
        .with_layout()
}

/// Assert the checked DIR snapshot for one source module.
pub(super) fn assert_check_snapshot(source: &str, expected: &str) {
    let compiler = TestCompiler::new().module("main.ds", source).build();
    let key = compiler.dir_checked_key("main.ds");
    let actual = compiler.checked_dir_snapshot("main.ds", check_snapshot_selection());
    let diagnostics = compiler.diagnostic_snapshot(key);

    assert_snapshot(diagnostics, "");
    assert_snapshot(actual, expected);
}

/// Assert checked DIR snapshots for selected source modules.
pub(super) fn assert_check_snapshots(compiler: &TestCompiler, paths: &[&str], expected: &str) {
    for path in paths {
        let key = compiler.dir_checked_key(path);
        compiler
            .provide_dir_checked(path)
            .expect("checked snapshot test should run check");
        assert_snapshot(compiler.diagnostic_snapshot(key), "");
    }

    assert_snapshot(
        compiler.checked_dir_snapshots(paths, check_snapshot_selection()),
        expected,
    );
}

/// Assert one checked DIR snapshot from an existing compiler.
pub(super) fn assert_check_module_snapshot(compiler: &TestCompiler, path: &str, expected: &str) {
    let key = compiler.dir_checked_key(path);
    compiler
        .provide_dir_checked(path)
        .expect("checked snapshot test should run check");

    assert_snapshot(compiler.diagnostic_snapshot(key), "");
    assert_snapshot(
        compiler.checked_dir_snapshot(path, check_snapshot_selection()),
        expected,
    );
}

/// Assert the checked diagnostics snapshot for one source module.
pub(super) fn assert_check_snapshot_with_diagnostics(source: &str, expected: &str) {
    let compiler = TestCompiler::new().module("main.ds", source).build();
    let key = compiler.dir_checked_key("main.ds");
    compiler
        .provide_dir_checked("main.ds")
        .expect("checked diagnostics test should run check");

    let actual = format!(
        "=== dir ===\n{}\n\n=== diagnostics ===\n{}",
        compiler.checked_dir_snapshot("main.ds", check_snapshot_selection()),
        compiler.diagnostic_snapshot(key),
    );

    assert_snapshot(actual, expected);
}
