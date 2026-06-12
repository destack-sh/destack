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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 42 = 42;

=== checked ===
const value = 42;
/// @type.symbol symbol=value source=value type=42
/// @type.node source=42 type=42

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_let_number_widens_binding_type() {
    let session = TestSession::single(
        r#"
let value = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: float64 = 42 as float64;

=== checked ===
let value = 42;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source=42 type=42

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_annotation_sets_binding_type() {
    let session = TestSession::single(
        r#"
const value: int32 = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: int32 = 42;

=== checked ===
const value: int32 = 42;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=42 type=42

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_const_float_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = 3.14;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 3.14 = 3.14;

=== checked ===
const value = 3.14;
/// @type.symbol symbol=value source=value type=3.14
/// @type.node source=3.14 type=3.14

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_let_float_widens_binding_type() {
    let session = TestSession::single(
        r#"
let value = 3.14;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: float64 = 3.14;

=== checked ===
let value = 3.14;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source=3.14 type=3.14

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_number_literal_rejects_string_context() {
    let session = TestSession::single(
        r#"
const value: string = 123;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: string = 123;

=== checked ===
const value: string = 123;
/// @type.symbol symbol=value source=value type=string
/// @type.node source=123 type=123

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type '123' is not assignable to type 'string'"
/// @diagnostic.label line=2 column=23 source="const value: string = 123;"
"#,
    );
}
