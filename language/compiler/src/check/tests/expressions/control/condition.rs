use crate::tests::TestSession;

#[test]
fn test_reports_literal_runtime_condition() {
    let session = TestSession::single(
        r#"
if (true) {
    const value = 1;
}
"#,
    );

    session.assert_dir_checked_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.warning code=WC402 message="condition is always true"
/// @diagnostic.label line=2 column=5 span="true" line_source="if (true) {"
"#,
    );
}
