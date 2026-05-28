use crate::tests::{DirRows, TestSession};

#[test]
fn test_false_static_if_branch_is_not_checked() {
    let session = TestSession::single(
        r#"
@if(false)
const hidden: MissingType = missingValue;

const visible = 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
@if(false)
const hidden: MissingType = missingValue;

const visible = 1;
/// @type.node source="const visible = 1" type=void
/// @type.symbol symbol=visible type=int32
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_true_static_if_branch_is_checked() {
    let session = TestSession::single(
        r#"
@if(true)
const value: int32 = "text";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
@if(true)
/// @type.node type=void

const value: int32 = "text";
/// @type.symbol symbol=value type=int32
/// @type.node source="\"text\"" type="text"
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=3 column=22 source="const value: int32 = \"text\";"
"#,
    );
}
