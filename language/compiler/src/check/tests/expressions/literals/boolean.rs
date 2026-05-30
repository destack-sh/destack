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
fn test_let_boolean_widens_binding_type() {
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
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_annotation_context_widens_binding_type() {
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
/// @type.node source=false type=false
"#,
    );
}

#[test]
fn test_false_const_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = false;
/// @type.symbol symbol=value type=false
/// @type.node source=false type=false
"#,
    );
}

#[test]
fn test_boolean_literal_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = true;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: string = true;
/// @type.symbol symbol=value type=string
/// @type.node source=true type=true

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: string = true;"
"#,
    );
}

#[test]
fn test_boolean_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = false;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: number = false;
/// @type.symbol symbol=value type=float64
/// @type.node source=false type=false

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: number = false;"
"#,
    );
}
