use crate::tests::TestSession;

#[test]
fn test_reject_written_hole_in_a_module_binding_annotation() {
    let session = TestSession::single(
        r#"
const value: _ = 1;
"#,
    );

    session.assert_dir_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=2 column=14 span="_" line_source="const value: _ = 1;"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

#[test]
fn test_reject_written_hole_in_a_member_annotation() {
    let session = TestSession::single(
        r#"
class Box {
    value: _ = 1;
}
"#,
    );

    session.assert_dir_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=3 column=12 span="_" line_source="value: _ = 1;"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

#[test]
fn test_accept_written_hole_in_a_body_binding_annotation() {
    let session = TestSession::single(
        r#"
function build(): void {
    const value: _ = 1;
}
"#,
    );

    session.assert_dir_diagnostics("main.ds", "");
}
