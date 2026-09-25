use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::TestParser;
use crate::{assert_node, assert_path, assert_string};
use tspp_dir::{
    Access, Declaration, Expression, GenericArgument, GenericParameter, TypeDeclaration,
    TypeExpression,
};

/// Parse one named borrow lifetime ahead of the access modifier.
#[test]
fn test_parse_borrow_with_named_lifetime() {
    let test = TestParser::new("type View = &'a immutable Buffer");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { lifetime, access, target_type, .. } => {
                assert_eq!(*access, Some(Access::Immutable));
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
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::BorrowedOf { lifetime, access, .. } => {
                assert_eq!(*access, Some(Access::Mutable));
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
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
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

/// Parse tick names as intersection operands inside generic arguments.
#[test]
fn test_parse_lifetime_intersection_generic_argument() {
    let test = TestParser::new("type Confined = Dynamic<Printable & 'a>");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Dynamic");
                assert_eq!(generic_arguments.len(), 1);
                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Intersection { elements } => {
                        assert_eq!(elements.len(), 2);
                        assert_node!(parser.tree, elements[0], TypeExpression::Reference { path, .. } => {
                            assert_path!(parser, *path, "Printable");
                        });
                        assert_node!(parser.tree, elements[1], TypeExpression::Lifetime { name } => {
                            assert_string!(parser, *name, "'a");
                        });
                    });
                });
            });
        });
    });
}

/// Parse a lifetime and space meet as one generic argument intersection.
#[test]
fn test_parse_lifetime_space_intersection_generic_argument() {
    let test = TestParser::new("type Leaked = Borrowed<Node, 'static & S>");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "Borrowed");
                assert_eq!(generic_arguments.len(), 2);
                assert_node!(parser.tree, generic_arguments[1], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Intersection { elements } => {
                        assert_eq!(elements.len(), 2);
                        assert_node!(parser.tree, elements[0], TypeExpression::Lifetime { name } => {
                            assert_string!(parser, *name, "'static");
                        });
                        assert_node!(parser.tree, elements[1], TypeExpression::Reference { path, .. } => {
                            assert_path!(parser, *path, "S");
                        });
                    });
                });
            });
        });
    });
}

/// Parse one bare tick generic parameter as an implicit const value.
#[test]
fn test_parse_bare_lifetime_generic_parameter() {
    let test = TestParser::new("function first<'a>(a: &'a Node): &'a Node { return a; }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
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

/// Parse one explicit const tick parameter with its bound.
#[test]
fn test_parse_const_lifetime_generic_parameter() {
    let test = TestParser::new(
        "function only<const 'a: Lifetime>(value: &'a Node): &'a Node { return value; }",
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Function(function) => {
            let parameters = &function.signature.generic_parameters;
            assert_eq!(parameters.len(), 1);
            assert_node!(parser.tree, parameters[0], GenericParameter::Type { name, constraint, is_const, .. } => {
                assert_string!(parser, *name, "'a");
                assert!(constraint.is_some());
                assert!(is_const);
            });
        });
    });
}
