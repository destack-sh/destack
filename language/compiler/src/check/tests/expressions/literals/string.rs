use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_string_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = "hello";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = "hello";
/// @type.symbol symbol=value type="hello"
/// @type.node source="\"hello\"" type="hello"
"#,
    );
}

#[test]
fn test_let_string_widens_literal_type() {
    let session = TestSession::single(
        r#"
let value = "hello";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value = "hello";
/// @type.symbol symbol=value type=string
/// @type.node source="\"hello\"" type=string
"#,
    );
}

#[test]
fn test_annotation_context_widens_string_literal() {
    let session = TestSession::single(
        r#"
const value: string = "hello";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: string = "hello";
/// @type.symbol symbol=value type=string
/// @type.node source="\"hello\"" type=string
"#,
    );
}

#[test]
fn test_empty_string_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = "";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = "";
/// @type.symbol symbol=value type=""
/// @type.node source="\"\"" type=""
"#,
    );
}

#[test]
fn test_string_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = "hello";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: number = "hello";
/// @type.symbol symbol=value type=float64
/// @type.node source="\"hello\"" type="hello"

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: number = \"hello\";"
"#,
    );
}
