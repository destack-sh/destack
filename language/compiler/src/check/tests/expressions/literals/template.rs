use crate::tests::{DirRows, TestSession};

#[test]
fn test_template_literal_has_string_type() {
    let session = TestSession::single(
        r#"
const name = "Ada";
const greeting = `hello ${name}`;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const name = "Ada";
/// @type.symbol symbol=name type="Ada"
/// @type.node source="\"Ada\"" type="Ada"

const greeting = `hello ${name}`;
/// @type.symbol symbol=greeting type=string
/// @type.node source="`hello ${name}`" type=string
/// @type.node source=name type="Ada"
/// @resolution.name source=name target=name
"#,
    );
}

#[test]
fn test_template_literal_is_assignable_to_string() {
    let session = TestSession::single(
        r#"
const greeting: string = `hello`;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const greeting: string = `hello`;
/// @type.symbol symbol=greeting type=string
/// @type.node source=`hello` type=string
"#,
    );
}

#[test]
fn test_template_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = `hello`;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: number = `hello`;
/// @type.symbol symbol=value type=float64
/// @type.node source=`hello` type=string

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: number = `hello`;"
"#,
    );
}
