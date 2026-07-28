use crate::tests::{DirRows, TestSession};

#[test]
fn test_literal_condition_reports_warning() {
    let session = TestSession::single(
        r#"
if (true) {
    const value = 1;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
if (true) {
    const value: 1 = 1;
}

=== checked ===
if (true) {
    const value = 1;
    /// @type.symbol symbol=value source=value type=1
    /// @resolution.pattern source=value kind=binding target=value

}
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=2 column=5 span="true" line_source="if (true) {"
"#,
    );
}
