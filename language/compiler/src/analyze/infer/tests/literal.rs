use super::*;

/// Analyze number literal.
#[test]
fn test_analyze_number_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source("test.ds", "42");

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // integer literals now have literal types, not widened primitive types
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
        }
    );
}

/// Analyze string literal.
#[test]
fn test_analyze_string_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source("test.ds", r#""hello""#);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // string literal has literal type (e.g., "hello" has type "hello")
    assert!(matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        }
    ));
}

/// Analyze boolean literal.
#[test]
fn test_analyze_boolean_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source("test.ds", "true");

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // boolean literal has literal type (e.g., true has type true)
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
        }
    );
}
