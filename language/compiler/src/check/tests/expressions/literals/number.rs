use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_number_preserves_literal_type() {
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
fn test_let_number_widens_literal_type() {
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
