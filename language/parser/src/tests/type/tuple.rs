use crate::tests::*;
use crate::{assert_node, assert_path, assert_string};
use destack_ast::*;

#[test]
fn test_parse_tuple_type_with_spread() {
    let mut test = TestParser::new("type T = [...Parts, string]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = [...Parts, string]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Spread { label, value } => {
                    assert!(label.is_none());
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "Parts");
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_optional_tuple_element() {
    let mut test = TestParser::new("type T = [EventTarget?]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = [EventTarget?]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert!(label.is_none());
                    assert!(*is_optional);
                    assert!(!is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "EventTarget");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_optional_tuple_element_trailing_comma() {
    let mut test = TestParser::new("type T = [EventTarget?,]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = [EventTarget?,]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, is_optional, .. } => {
                    assert!(*is_optional);
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "EventTarget");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_optional_labeled_tuple_element() {
    let mut test = TestParser::new("type T = [start?: number]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = [start?: number]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert_string!(parser, *label, "start");
                    assert!(*is_optional);
                    assert!(!is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_optional_readonly_tuple_element() {
    let mut test = TestParser::new("type T = [readonly EventTarget?]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = [readonly EventTarget?]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, is_optional, is_readonly, .. } => {
                    assert!(*is_optional);
                    assert!(*is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "EventTarget");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_tuple_type() {
    let mut test = TestParser::new("type T = [string, number]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = [string, number]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_labeled_tuple_type() {
    let mut test = TestParser::new("type T = [start: number, end: number]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = [start: number, end: number]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, .. } => {
                    assert_string!(parser, *label, "start");
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { label, value, .. } => {
                    assert_string!(parser, *label, "end");
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_labeled_tuple_type_complex() {
    let mut test =
        TestParser::new("type T = [importCode: string, nameMap: Record<string, string>]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    // type T = [importCode: string, nameMap: Record<string, string>]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, .. } => {
                    assert_string!(parser, *label, "importCode");
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { label, .. } => {
                    assert_string!(parser, *label, "nameMap");
                });
            });
        });
    });
}

#[test]
fn test_parse_tuple_type_missing_close_bracket() {
    let mut test = TestParser::new("type T = [string");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.options).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::TypeExpression), None, "")]);

    // type T = [string
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, is_optional, is_readonly, .. } => {
                    assert!(!is_optional);
                    assert!(!is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
            });
        });
    });
}
