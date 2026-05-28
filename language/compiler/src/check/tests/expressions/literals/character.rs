use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_character_preserves_literal_type() {
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
/// @type.symbol symbol=value type='a'
/// @type.node source='a' type='a'
"#,
    );
}

#[test]
fn test_let_character_widens_literal_type() {
    let session = TestSession::single(
        r#"
let value = 'a';
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value = 'a';
/// @type.symbol symbol=value type=char
/// @type.node source='a' type=char
"#,
    );
}

#[test]
fn test_annotation_context_widens_character_literal() {
    let session = TestSession::single(
        r#"
const value: char = 'a';
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: char = 'a';
/// @type.symbol symbol=value type=char
/// @type.node source='a' type=char
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
/// @type.node source='a' type='a'

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: string = 'a';"
"#,
    );
}

#[test]
fn test_character_literal_rejects_integer_context() {
    let session = TestSession::single(
        r#"
const value: int32 = 'a';
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: int32 = 'a';
/// @type.symbol symbol=value type=int32
/// @type.node source='a' type='a'

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=22 source="const value: int32 = 'a';"
"#,
    );
}

#[test]
fn test_character_literal_flows_into_character_union() {
    let session = TestSession::single(
        r#"
const value: char | string = 'a';
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: char | string = 'a';
/// @type.symbol symbol=value type=char | string
/// @type.node source='a' type=char | string
"#,
    );
}
