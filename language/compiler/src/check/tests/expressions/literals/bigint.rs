use crate::tests::{DirRows, TestSession};

#[test]
fn test_bigint_literal_has_bigint_type() {
    let session = TestSession::single(
        r#"
const value: bigint = 42n;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: bigint = 42n;
/// @type.symbol symbol=value type=bigint
/// @type.node source=42n type=bigint
"#,
    );
}

#[test]
fn test_bigint_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = 42n;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: number = 42n;
/// @type.symbol symbol=value type=number
/// @type.node source=42n type=bigint

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: number = 42n;"
"#,
    );
}
