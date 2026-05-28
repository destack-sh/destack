use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_bigint_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = 42n;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = 42n;
/// @type.symbol symbol=value type=42n
/// @type.node source=42n type=42n
"#,
    );
}

#[test]
fn test_let_bigint_widens_literal_type() {
    let session = TestSession::single(
        r#"
let value = 42n;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value = 42n;
/// @type.symbol symbol=value type=bigint
/// @type.node source=42n type=bigint
"#,
    );
}

#[test]
fn test_annotation_context_widens_bigint_literal() {
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
/// @type.symbol symbol=value type=float64
/// @type.node source=42n type=42n

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=23 source="const value: number = 42n;"
"#,
    );
}

#[test]
fn test_bigint_literal_flows_into_bigint_union() {
    let session = TestSession::single(
        r#"
const value: bigint | string = 42n;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: bigint | string = 42n;
/// @type.symbol symbol=value type=bigint | string
/// @type.node source=42n type=bigint | string
"#,
    );
}
