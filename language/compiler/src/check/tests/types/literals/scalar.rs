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
fn test_let_number_widens_binding_to_integer_type() {
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
fn test_string_literal_annotation_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value: "ready" = "ready";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: "ready" = "ready";

=== checked ===
const value: "ready" = "ready";
/// @type.symbol symbol=value source=value type="ready"
/// @type.node source="\"ready\"" type="ready"

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: true = true;

=== checked ===
const value: true = true;
/// @type.symbol symbol=value source=value type=true
/// @type.node source=true type=true

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_null_literal_keeps_null_type() {
    let session = TestSession::single(
        r#"
const value = null;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: null = null;

=== checked ===
const value = null;
/// @type.symbol symbol=value source=value type=null
/// @type.node source=null type=null

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_undefined_literal_keeps_undefined_type() {
    let session = TestSession::single(
        r#"
const value = undefined;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: undefined = undefined;

=== checked ===
const value = undefined;
/// @type.symbol symbol=value source=value type=undefined
/// @type.node source=undefined type=undefined

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 42 = 42 satisfies int32;

=== checked ===
const value = 42 satisfies int32;
/// @type.symbol symbol=value source=value type=42
/// @type.node source="42 satisfies int32" type=42
/// @type.node source=42 type=42

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}
