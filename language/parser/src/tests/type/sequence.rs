use crate::tests::TestParser;
use crate::{assert_expression_path, assert_name, assert_node, assert_path, assert_string};
use destack_dir::{
    BinaryOperator, Declaration, Expression, InferForm, NodeType, ScalarLiteral, TupleElement,
    TypeDeclaration, TypeExpression, TypeLiteral,
};
use destack_source::LanguageType;

#[test]
fn test_parse_slice_type() {
    let mut test = TestParser::new("type T = [EventTarget]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [EventTarget]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Slice { element } => {
                assert_node!(parser.tree, *element, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "EventTarget");
                });
            });
        });
    });
}

#[test]
fn test_parse_readonly_slice_type() {
    let mut test = TestParser::new("type T = [readonly EventTarget]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [readonly EventTarget]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Slice { element } => {
                assert_node!(parser.tree, *element, TypeExpression::Readonly { target_type } => {
                    assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "EventTarget");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_fixed_array_type() {
    let mut test = TestParser::new("type T = [EventTarget; 32]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [EventTarget; 32]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FixedArray { element, length } => {
                assert_node!(parser.tree, *element, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "EventTarget");
                });
                assert_node!(parser.tree, *length, Expression::ScalarLiteral(ScalarLiteral::Integer(32)));
            });
        });
    });
}

#[test]
fn test_parse_fixed_array_type_length_infer_hole() {
    let mut test = TestParser::new("type T = [EventTarget; _]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [EventTarget; _]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FixedArray { length, .. } => {
                assert_node!(parser.tree, *length, Expression::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Infer { form, name, constraint } => {
                        assert_eq!(*form, InferForm::Hole);
                        assert!(name.is_none());
                        assert!(constraint.is_none());
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_fixed_array_type_value_length_expression() {
    let mut test = TestParser::new("type T<comptime N: uint> = [EventTarget; N * 2]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T<comptime N: uint> = [EventTarget; N * 2]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FixedArray { length, .. } => {
                assert_node!(parser.tree, *length, Expression::Binary { left, operator, right } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "N");
                    assert_eq!(*operator, BinaryOperator::Multiply);
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });
        });
    });
}

#[test]
fn test_parse_fixed_array_type_recovers_missing_length_expression() {
    let mut test = TestParser::new("type T = [EventTarget; ]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [EventTarget; ]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FixedArray { length, .. } => {
                assert_node!(parser.tree, *length, Expression::Missing);
            });
        });
    });
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "]")]);
}

#[test]
fn test_parse_single_element_tuple_type() {
    let mut test = TestParser::new("type T = [EventTarget,]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [EventTarget,]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert!(label.is_none());
                    assert!(!*is_optional);
                    assert!(!*is_readonly);
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
fn test_parse_typescript_single_element_tuple_type() {
    let mut test =
        TestParser::new_with_language("type T = [EventTarget]", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [EventTarget]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert!(label.is_none());
                    assert!(!*is_optional);
                    assert!(!*is_readonly);
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
fn test_parse_tuple_type_with_spread() {
    let mut test = TestParser::new("type T = [...Parts, string]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [...Parts, string]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
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
fn test_parse_labeled_tuple_type_with_spread_payload() {
    let mut test =
        TestParser::new(r#"type T = [keys: ...RedisClient.KeyLike[], withscores: "WITHSCORES"]"#);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_no_errors(&parser);

    // type T = [keys: ...RedisClient.KeyLike[], withscores: "WITHSCORES"]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, export, is_ambient, is_nominal, mutability, generic_parameters, where_clauses, value }) => {
            assert_name!(parser, *name, "T");
            assert!(export.is_none());
            assert_eq!(*is_ambient, false);
            assert!(!*is_nominal);
            assert!(mutability.is_none());
            assert!(generic_parameters.is_empty());
            assert!(where_clauses.is_empty());
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Spread { label, value } => {
                    assert_string!(parser, label.expect("expected spread label"), "keys");
                    assert_node!(parser.tree, *value, TypeExpression::Array { element } => {
                        assert_node!(parser.tree, *element, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "RedisClient.KeyLike");
                        });
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert_string!(parser, label.expect("expected tuple label"), "withscores");
                    assert!(!*is_optional);
                    assert!(!*is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::ScalarLiteral { value } => {
                        assert_eq!(*value, ScalarLiteral::String(parser.strings.intern("WITHSCORES")));
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
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [EventTarget?]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
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
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [EventTarget?,]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
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
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [start?: number]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
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
fn test_parse_optional_tuple_element_with_readonly_type() {
    let mut test = TestParser::new("type T = [readonly EventTarget?]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [readonly EventTarget?]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, is_optional, is_readonly, .. } => {
                    assert!(*is_optional);
                    assert!(!*is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Readonly { target_type } => {
                        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "EventTarget");
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_typescript_readonly_tuple_element() {
    let mut test =
        TestParser::new_with_language("type T = [readonly EventTarget]", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [readonly EventTarget]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, is_optional, is_readonly, .. } => {
                    assert!(!*is_optional);
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
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [string, number]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
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
fn test_parse_tuple_type_with_readonly_type_element() {
    let mut test = TestParser::new("type T = [string, readonly EventTarget]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [string, readonly EventTarget]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[1], TupleElement::Element { value, is_readonly, .. } => {
                    assert!(!*is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Readonly { target_type } => {
                        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                            assert!(generic_arguments.is_empty());
                            assert_path!(parser, *path, "EventTarget");
                        });
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
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [start: number, end: number]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
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
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // type T = [importCode: string, nameMap: Record<string, string>]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
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
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::TypeExpression), None, "")]);

    // type T = [string
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Slice { element } => {
                assert_node!(parser.tree, *element, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
        });
    });
}

#[test]
fn test_parse_tuple_type_missing_first_element() {
    let mut test = TestParser::new("type T = [, string]");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::TypeExpression), None, ",")]);

    // type T = [, string]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::ArrayTuple { elements } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Missing);
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
