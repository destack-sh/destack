use crate::tests::{DirRows, TestSession};

#[test]
fn test_member_access_decorator_reports_error() {
    let session = TestSession::single(
        r#"
const subject = 1;

@subject.field
const value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const subject: 1 = 1;

@subject.field
const value: 1 = 1;

=== checked ===
const subject = 1;
/// @type.symbol symbol=subject source=subject type=1
/// @type.node source=1 type=1

@subject.field
const value = 1;
/// @type.symbol symbol=value source=value type=1
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error code=EC310 message="decorator must name a declaration"
/// @diagnostic.label line=4 column=10 span="field" line_source="@subject.field"
"#,
    );
}
