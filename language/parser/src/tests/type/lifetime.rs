use crate::parse::{ExpressionContext, StatementPosition};
use crate::tests::TestParser;
use crate::{assert_node, assert_path, assert_string};
use destack_dir::{
    Declaration, Expression, GenericParameter, Mutability, TypeDeclaration, TypeExpression,
};

/// Parse one named borrow lifetime ahead of the access modifier.
#[test]
fn test_parse_borrow_with_named_lifetime() {
    let test = TestParser::new("type View = &'a readonly Buffer");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionContext {
            statement: StatementPosition::Direct,
            ..ExpressionContext::default()
        })
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { lifetime, mutability, target_type, .. } => {
                assert_eq!(*mutability, Some(Mutability::Immutable));
                let lifetime = lifetime.expect("expected lifetime");
                assert_node!(parser.tree, lifetime, TypeExpression::Lifetime { name } => {
                    assert_string!(parser, *name, "'a");
                    let span = parser
                        .tree
                        .get_main_span(lifetime)
                        .expect("expected lifetime main span");
                    assert_eq!(parser.span_str(span), "'a");
                });
                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "Buffer");
                });
            });
        });
    });
}

/// Parse the static lifetime literal in a borrow.
#[test]
fn test_parse_borrow_with_static_lifetime() {
    let test = TestParser::new("type View = &'static Buffer");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionContext {
            statement: StatementPosition::Direct,
            ..ExpressionContext::default()
        })
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { lifetime, mutability, .. } => {
                assert_eq!(*mutability, Some(Mutability::Mutable));
                assert_node!(parser.tree, lifetime.unwrap(), TypeExpression::Lifetime { name } => {
                    assert_string!(parser, *name, "'static");
                });
            });
        });
    });
}

/// Parse tick names as generic arguments and union operands.
#[test]
fn test_parse_lifetime_union_generic_argument() {
    let test = TestParser::new("type Joined = Borrowed<Node, 'a | 'b>");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionContext {
            statement: StatementPosition::Direct,
            ..ExpressionContext::default()
        })
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Borrowed");
                assert_eq!(generic_arguments.len(), 2);
            });
        });
    });
}

/// Parse one bare tick generic parameter as an implicit comptime value.
#[test]
fn test_parse_bare_lifetime_generic_parameter() {
    let test = TestParser::new("function first<'a>(a: &'a Node): &'a Node { return a; }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionContext {
            statement: StatementPosition::Direct,
            ..ExpressionContext::default()
        })
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Function(function) => {
            let parameters = &function.signature.generic_parameters;
            assert_eq!(parameters.len(), 1);
            assert_node!(parser.tree, parameters[0], GenericParameter::Lifetime { name } => {
                assert_string!(parser, *name, "'a");
            });
        });
    });
}

/// Parse one explicit comptime tick parameter with its bound.
#[test]
fn test_parse_comptime_lifetime_generic_parameter() {
    let test = TestParser::new(
        "function only<comptime 'a: Lifetime>(value: &'a Node): &'a Node { return value; }",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionContext {
            statement: StatementPosition::Direct,
            ..ExpressionContext::default()
        })
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Function(function) => {
            let parameters = &function.signature.generic_parameters;
            assert_eq!(parameters.len(), 1);
            assert_node!(parser.tree, parameters[0], GenericParameter::Value { name, declared_type, is_comptime, .. } => {
                assert_string!(parser, *name, "'a");
                assert!(declared_type.is_some());
                assert!(is_comptime);
            });
        });
    });
}
