use crate::tests::{DirRows, TestSession};

#[test]
fn test_character_literal_has_char_type() {
    let session = TestSession::single(
        r#"
const value = 'a';
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = 'a';
/// @type.symbol symbol=value type=char
/// @type.node source="'a'" type=char
"#,
    );
}

#[test]
fn test_character_literal_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = 'a';
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: string = 'a';
/// @type.symbol symbol=value type=string
/// @type.node source="'a'" type=char

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: string = 'a';"
"#,
    );
}
