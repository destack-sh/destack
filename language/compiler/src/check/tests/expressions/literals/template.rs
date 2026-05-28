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
/// @resolution.name source=name target=name
/// @type.node source="`hello ${name}`" type=string
"#,
    );
}
