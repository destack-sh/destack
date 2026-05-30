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
        DirRows::checked()
            .with_reference_types()
            .with_check_stats(),
        r#"
const value = 42;
/// @type.symbol symbol=value source=value type=42
/// @type.node source=42 type=42
/// @check.stats.solve variables=0 definitions=0 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
/// @check.stats.output total=3 type_nodes=2 type_symbols=1 static_nodes=0 static_symbols=0
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
        DirRows::checked()
            .with_reference_types()
            .with_check_stats(),
        r#"
let value = 42;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=42 type=42
/// @check.stats.solve variables=1 definitions=1 constraints=0 obligations=0 solutions=1 bounds=0 decisions=0
/// @check.stats.output total=3 type_nodes=2 type_symbols=1 static_nodes=0 static_symbols=0
"#,
    );
}

#[test]
fn test_annotation_context_widens_binding_type() {
    let session = TestSession::single(
        r#"
const value: int32 = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            .with_check_stats(),
        r#"
const value: int32 = 42;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=42 type=42
/// @check.stats.solve variables=0 definitions=0 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
/// @check.stats.output total=4 type_nodes=3 type_symbols=1 static_nodes=0 static_symbols=0
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
        DirRows::checked().with_reference_types(),
        r#"
const value: "ready" = "ready";
/// @type.symbol symbol=value source=value type="ready"
/// @type.node source="\"ready\"" type="ready"
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
        DirRows::checked().with_reference_types(),
        r#"
const value: true = true;
/// @type.symbol symbol=value source=value type=true
/// @type.node source=true type=true
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
        DirRows::checked().with_reference_types(),
        r#"
const value = 42 satisfies int32;
/// @type.symbol symbol=value source=value type=42
/// @type.node source="42 satisfies int32" type=42
/// @type.node source=42 type=42
"#,
    );
}
