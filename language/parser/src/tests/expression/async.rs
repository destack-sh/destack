use crate::tests::TestParser;
use crate::{
    ExpressionPosition, ExpressionStop, assert_expression_path, assert_node, assert_string,
};
use tspp_dir::{
    AssignOperator, AssignPattern, Asynchrony, BinaryOperator, Declaration, Expression,
    FunctionDeclaration, FunctionForm, GenericArgument, GenericParameter, LetKind, Literal,
};

/// Parse async generic arrows with constraint and default type parameters in assignments.
#[test]
fn test_parse_async_generic_arrow_assignment_with_constraint_default() {
    // source: pollContext.getCredentials = async <T: object = ICredentialDataDecryptedObject>() => (options.credential ?? {}) as T
    let test = TestParser::new(
        "pollContext.getCredentials = async <T: object = ICredentialDataDecryptedObject>() => (options.credential ?? {}) as T",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // pollContext.getCredentials = async <T: object = ICredentialDataDecryptedObject>() => (options.credential ?? {}) as T
    assert_node!(parser.tree, expr_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);
        assert_expression_path!(parser, parser.tree.get(*left), "pollContext.getCredentials");

        assert_node!(parser.tree, *right, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                assert_eq!(signature.asynchrony, Asynchrony::Async);
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert!(signature.parameters.is_empty());

                let generic_parameters = &signature.generic_parameters;
                assert_eq!(generic_parameters.len(), 1);

                assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
                    assert_string!(parser, *name, "T");
                    assert_expression_path!(parser, parser.tree.get(constraint.expect("expected extends constraint")), "object");
                    assert_expression_path!(parser, parser.tree.get(default.expect("expected default type")), "ICredentialDataDecryptedObject");
                });

                let body = body.expect("expected body");
                assert_node!(parser.tree, body, Expression::As { target_type, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*target_type), "T");
                });
            });
        });
    });
}

/// Parse async comparisons and generic calls without async function false positives.
#[test]
fn test_parse_async_generic_false_positive() {
    let test = TestParser::new("async < 1;\nasync<T>() == 0;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 2);

    assert_node!(parser.tree, expressions[0], Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::LessThan);
        assert_expression_path!(parser, parser.tree.get(*left), "async");
        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(1)));
    });

    assert_node!(parser.tree, expressions[1], Expression::Binary { left, operator, right } => {
        assert_eq!(*operator, BinaryOperator::Equal);
        assert_node!(parser.tree, *left, Expression::Call { left, generic_arguments, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "async");
            assert!(arguments.is_empty());

            let generic_arguments = generic_arguments.as_slice();
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "T");
            });
        });
        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(0)));
    });
}

/// Parse async generic arrow ASI.
#[test]
fn test_parse_async_generic_arrow_asi() {
    let test = TestParser::new("let a = {}\nasync<T,>() => {}\n\n(a as unknown).b = 1;\n");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 3);

    assert_node!(parser.tree, expressions[0], Expression::Let { kind, declarators, .. } => {
        assert_eq!(*kind, LetKind::Let);
        assert_eq!(declarators.len(), 1);
    });

    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
            assert_eq!(signature.asynchrony, Asynchrony::Async);
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert!(signature.parameters.is_empty());
            assert!(!signature.generic_parameters.is_empty());
            assert!(body.is_some());
        });
    });

    assert_node!(parser.tree, expressions[2], Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);
        assert_node!(parser.tree, *left, AssignPattern::Place { expression: value } => {
            assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "b");
                crate::assert_parenthesized!(parser.tree, *left, expression => {
                    assert_node!(parser.tree, *expression, Expression::As { .. } => {});
                });
            });
        });
        assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(1)));
    });
}
