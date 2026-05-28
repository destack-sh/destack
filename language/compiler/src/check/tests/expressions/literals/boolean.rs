use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_boolean_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = true;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = true;
/// @type.symbol symbol=value type=true
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_let_boolean_widens_literal_type() {
    let session = TestSession::single(
        r#"
let value = true;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value = true;
/// @type.symbol symbol=value type=boolean
/// @type.node source=true type=boolean
"#,
    );
}

#[test]
fn test_annotation_context_widens_boolean_literal() {
    let session = TestSession::single(
        r#"
const value: boolean = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: boolean = false;
/// @type.symbol symbol=value type=boolean
/// @type.node source=false type=boolean
"#,
    );
}
