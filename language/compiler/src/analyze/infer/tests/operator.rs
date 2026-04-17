use super::*;

/// Analyze binary number operation.
#[test]
fn test_analyze_binary_number_operation() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "1 + 2");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // constant folding: 1 + 2 evaluates to literal type 3
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(3))
        }
    );
}

/// Narrow nullish types in an if guard.
#[test]
fn test_narrowing_nullish_if_guard() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const value: string | null = null;
if (value != null) {
    const narrowed: string = value;
} else {
    0;
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the if expression
    let (_, if_expression) = view
        .tree()
        .iter_nodes_of_type::<Expression>()
        .find(|(_, expression)| matches!(expression, Expression::If { .. }))
        .expect("expected if expression");
    let Expression::If {
        then_expression, ..
    } = if_expression
    else {
        panic!("expected if expression");
    };

    // locate the narrowed declaration in the then block
    let then_expression_id = match view.tree().get(*then_expression) {
        Expression::Block(block_id) => view
            .tree()
            .get(*block_id)
            .first_expression()
            .expect("expected then block expression"),
        _ => *then_expression,
    };

    let Expression::Let { declarators, .. } = view.tree().get(then_expression_id) else {
        panic!("expected let expression");
    };

    let declarator_id = declarators.first().expect("expected declarator");
    let declarator = view.tree().get(*declarator_id);
    let value_expression_id = declarator
        .value
        .expect("expected value expression in declarator");

    // assert nullish types are stripped in the then branch
    let left_type_id = view.expect_inferred_type_id(value_expression_id);
    let left_type = view.types().get_type(left_type_id);

    let is_nullish = match left_type {
        Type::Union { elements } => elements.iter().any(|element_id| {
            matches!(
                view.types().get_type(*element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Null | TypeLiteral::Undefined
                }
            )
        }),
        Type::TypeLiteral {
            value: TypeLiteral::Null | TypeLiteral::Undefined,
        } => true,
        _ => false,
    };

    assert!(!is_nullish, "expected nullish to be stripped");
}

/// Analyze binary number comparison.
#[test]
fn test_analyze_binary_number_comparison() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "1 < 2");

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // select the root expression
    let expression_id = view.root_expression_id(0);

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    // constant folding: 1 < 2 evaluates to literal true
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
        }
    );
}

/// Analyze union discriminant comparisons as boolean expressions.
#[test]
fn test_analyze_union_discriminant_comparison() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function select(value: { kind: 0, value: int32 } | { kind: 1, value: int32 }): int32 {
    return value.kind == 0 ? 1 : 2;
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the ternary condition
    let condition_id = view
        .tree()
        .iter_node_ids_of_type::<Expression>()
        .iter()
        .find_map(|expression_id| match view.tree().get(*expression_id) {
            Expression::If {
                kind: IfKind::Ternary,
                condition: IfCondition::Expression { condition },
                ..
            } => Some(*condition),
            _ => None,
        })
        .expect("expected ternary condition expression");

    // read inferred type
    let ty = view.expect_inferred_type(condition_id);

    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean)
        }
    );
}

/// Analyze union literal comparisons that include non-literal members.
#[test]
fn test_analyze_union_literal_comparison_mixed() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function isReady(value: true | { value: int32 }): boolean {
    return value == true;
}
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // locate the equality expression
    let expression_id = view
        .tree()
        .iter_node_ids_of_type::<Expression>()
        .iter()
        .find_map(|expression_id| match view.tree().get(*expression_id) {
            Expression::Binary { operator, .. } if *operator == BinaryOperator::Equal => {
                Some(*expression_id)
            }
            _ => None,
        })
        .expect("expected equality expression");

    // read inferred type
    let ty = view.expect_inferred_type(expression_id);

    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Boolean)
        }
    );
}
