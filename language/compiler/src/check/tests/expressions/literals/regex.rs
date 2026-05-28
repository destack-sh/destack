use crate::tests::{DirRows, TestSession};

#[test]
fn test_regex_literal_has_regexp_type() {
    let session = TestSession::single(
        r#"
const value: RegExp = /abc/;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: RegExp = /abc/;
/// @resolution.name source=RegExp target=regexp.RegExp
/// @type.symbol symbol=value type=regexp.RegExp
/// @type.node source=/abc/ type=regexp.RegExp
"#,
    );
}

#[test]
fn test_regex_literal_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = /abc/;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: string = /abc/;
/// @type.symbol symbol=value type=string
/// @type.node source=/abc/ type=regexp.RegExp

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: string = /abc/;"
"#,
    );
}
