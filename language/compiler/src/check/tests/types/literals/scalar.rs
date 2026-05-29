use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_number_keeps_literal_type() {
    let session = TestSession::single(
        r#"
const value = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = 42;
/// @type.symbol symbol=value type=42
/// @type.node source=42 type=42
"#,
    );
}

#[test]
fn test_let_number_widens_to_integer_type() {
    let session = TestSession::single(
        r#"
let value = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value = 42;
/// @type.symbol symbol=value type=int32
/// @type.node source=42 type=int32
"#,
    );
}

#[test]
fn test_annotation_context_widens_number_literal() {
    let session = TestSession::single(
        r#"
const value: int32 = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: int32 = 42;
/// @type.symbol symbol=value type=int32
/// @type.node source=42 type=int32
"#,
    );
}

#[test]
fn test_string_literal_annotation_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value: "ready" = "ready";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: "ready" = "ready";
/// @type.symbol symbol=value type="ready"
/// @type.node source="\"ready\"" type="ready"
"#,
    );
}

#[test]
fn test_boolean_literal_annotation_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value: true = true;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: true = true;
/// @type.symbol symbol=value type=true
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_satisfies_preserves_scalar_literal_type() {
    let session = TestSession::single(
        r#"
const value = 42 satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = 42 satisfies int32;
/// @type.symbol symbol=value type=42
/// @type.node source="42 satisfies int32" type=42
/// @type.node source=42 type=42
"#,
    );
}
