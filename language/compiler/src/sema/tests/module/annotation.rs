use crate::tests::{DirRows, TestSession};

/// Type an incomplete ambient binding after reporting its missing annotation.
#[test]
fn test_type_incomplete_ambient_binding() {
    let session = TestSession::single(
        r#"
declare const value;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value;

=== dir ===
declare const value;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
"#,
        r#"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=2 column=15 span="value" line_source="declare const value;"
"#,
    );
}

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
