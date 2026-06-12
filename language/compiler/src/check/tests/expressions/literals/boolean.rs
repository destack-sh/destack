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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: true = true;

=== checked ===
const value = true;
/// @type.symbol symbol=value source=value type=true
/// @type.node source=true type=true

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: boolean = true;

=== checked ===
let value = true;
/// @type.symbol symbol=value source=value type=boolean
/// @type.node source=true type=true

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_annotation_sets_binding_type() {
    let session = TestSession::single(
        r#"
const value: boolean = false;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: boolean = false;

=== checked ===
const value: boolean = false;
/// @type.symbol symbol=value source=value type=boolean
/// @type.node source=false type=false

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: false = false;

=== checked ===
const value = false;
/// @type.symbol symbol=value source=value type=false
/// @type.node source=false type=false

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: string = true;

=== checked ===
const value: string = true;
/// @type.symbol symbol=value source=value type=string
/// @type.node source=true type=true

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'true' is not assignable to type 'string'"
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
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: number = false;

=== checked ===
const value: number = false;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source=false type=false

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'false' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 source="const value: number = false;"
"#,
    );
}
