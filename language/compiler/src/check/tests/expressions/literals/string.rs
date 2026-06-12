use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_string_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = "hello";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_check_stats(),
        r#"
=== annotated ===
const value: "hello" = "hello";

=== checked ===
const value = "hello";
/// @type.symbol symbol=value source=value type="hello"
/// @type.node source="\"hello\"" type="hello"

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_let_string_widens_binding_type() {
    let session = TestSession::single(
        r#"
let value = "hello";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: string = "hello";

=== checked ===
let value = "hello";
/// @type.symbol symbol=value source=value type=string
/// @type.node source="\"hello\"" type="hello"

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_annotation_sets_binding_type() {
    let session = TestSession::single(
        r#"
const value: string = "hello";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: string = "hello";

=== checked ===
const value: string = "hello";
/// @type.symbol symbol=value source=value type=string
/// @type.node source="\"hello\"" type="hello"

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_empty_string_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value = "";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: "" = "";

=== checked ===
const value = "";
/// @type.symbol symbol=value source=value type=""
/// @type.node source="\"\"" type=""

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_string_literal_rejects_number_context() {
    let session = TestSession::single(
        r#"
const value: number = "hello";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: number = "hello";

=== checked ===
const value: number = "hello";
/// @type.symbol symbol=value source=value type=float64
/// @type.node source="\"hello\"" type="hello"

/// @check.stats.solve variables=0 types=3 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"hello\"' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=23 source="const value: number = \"hello\";"
"#,
    );
}
