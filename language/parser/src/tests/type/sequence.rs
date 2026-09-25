use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::TestParser;
use crate::{
    ParserErrorKind, TypePosition, TypeStop, assert_expression_path, assert_name, assert_node,
    assert_path, assert_string,
};
use tspp_dir::{
    BinaryOperator, Declaration, Expression, InferForm, Literal, NodeType, TokenType, TupleElement,
    TupleForm, TypeDeclaration, TypeExpression, TypeLiteral,
};

/// Parse a repeated Pattern placeholder as one complete tuple element.
#[test]
fn test_parse_pattern_tuple_element_placeholder() {
    let test = TestParser::new("type Values = [$$$ELEMENTS]");
    let mut parser = test.prepare_pattern();
    let roots = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, roots[0], Expression::Declaration(value) => {
        assert_node!(parser.tree, *value, Declaration::Type(declaration) => {
            assert_node!(parser.tree, declaration.value, TypeExpression::Tuple { elements, .. } => {
                assert_eq!(elements.len(), 1);
            });
        });
    });
}

/// Assert that one bracketed tuple type reports over its whole bracket group.
fn assert_rejects_bracket_tuple_type(input: &str) {
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();

    let errors: Vec<_> = parser
        .errors
        .iter()
        .map(|error| (error.kind(), parser.range_str(error.range())))
        .collect();

    assert_eq!(errors, vec![(ParserErrorKind::BracketTupleType, input)]);
}

/// Reject tuple types written with brackets so only the parenthesized form parses.
#[test]
fn test_reject_bracket_tuple_type() {
    assert_rejects_bracket_tuple_type("[string, int32]");
    assert_rejects_bracket_tuple_type("[string,]");
    assert_rejects_bracket_tuple_type("[start: number, end: number]");
    assert_rejects_bracket_tuple_type("[]");
}

#[test]
fn test_parse_slice_type() {
    let test = TestParser::new("type T = [EventTarget]");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("type T = [readonly EventTarget]");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

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
    let test = TestParser::new("type T = [EventTarget; 32]");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = [EventTarget; 32]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FixedArray { element, length } => {
                assert_node!(parser.tree, *element, TypeExpression::Reference { path, generic_arguments } => {
                    assert!(generic_arguments.is_empty());
                    assert_path!(parser, *path, "EventTarget");
                });
                assert_node!(parser.tree, *length, Expression::Literal(Literal::Integer(32)));
            });
        });
    });
}

#[test]
fn test_parse_fixed_array_type_length_infer_hole() {
    let test = TestParser::new("type T = [EventTarget; _]");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = [EventTarget; _]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FixedArray { length, .. } => {
                assert_node!(parser.tree, *length, Expression::Infer { form, name } => {
                    assert_eq!(*form, InferForm::Hole);
                    assert!(name.is_none());
                });
            });
        });
    });
}

#[test]
fn test_parse_fixed_array_type_value_length_expression() {
    let test = TestParser::new("type T<const N: uint> = [EventTarget; N * 2]");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T<const N: uint> = [EventTarget; N * 2]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FixedArray { length, .. } => {
                assert_node!(parser.tree, *length, Expression::Binary { left, operator, right } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "N");
                    assert_eq!(*operator, BinaryOperator::Multiply);
                    assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(2)));
                });
            });
        });
    });
}

#[test]
fn test_parse_fixed_array_type_recovers_missing_length_expression() {
    let test = TestParser::new("type T = [EventTarget; ]");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = [EventTarget; ]
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::FixedArray { length, .. } => {
                assert_node!(parser.tree, *length, Expression::Missing);
            });
        });
    });
    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "]")]);
}

#[test]
fn test_parse_single_element_tuple_type() {
    let test = TestParser::new("type T = (EventTarget,)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (EventTarget,)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
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
    let test = TestParser::new("type T = (...Parts, string)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (...Parts, string)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Spread { label, value } => {
                    assert!(label.is_none());
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "Parts");
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_labeled_tuple_type_with_spread_payload() {
    let test =
        TestParser::new(r#"type T = (keys: ...RedisClient.KeyLike[], withscores: "WITHSCORES")"#);
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    // type T = (keys: ...RedisClient.KeyLike[], withscores: "WITHSCORES")
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { name, export, is_shared: _, is_ambient, is_nominal, mutability, generic_parameters, where_clauses, value, backing_visibility: _ }) => {
            assert_name!(parser, *name, "T");
            assert!(export.is_none());
            assert!(!*is_ambient);
            assert!(!*is_nominal);
            assert!(mutability.is_none());
            assert!(generic_parameters.is_empty());
            assert!(where_clauses.is_empty());
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
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
                    assert_node!(parser.tree, *value, TypeExpression::Literal { value } => {
                        assert_eq!(*value, Literal::String(parser.strings.intern("WITHSCORES")));
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_optional_tuple_element() {
    let test = TestParser::new("type T = (EventTarget?)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (EventTarget?)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
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
    let test = TestParser::new("type T = (EventTarget?,)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (EventTarget?,)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
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
    let test = TestParser::new("type T = (start?: number)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (start?: number)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, is_optional, is_readonly } => {
                    assert_string!(parser, *label, "start");
                    assert!(*is_optional);
                    assert!(!is_readonly);
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_optional_tuple_element_with_readonly_type() {
    let test = TestParser::new("type T = (readonly EventTarget?)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (readonly EventTarget?)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
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
fn test_parse_tuple_type() {
    let test = TestParser::new("type T = (string, number)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (string, number)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { form, elements } => {
                assert_eq!(*form, TupleForm::Tuple);
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_tuple_type_with_readonly_type_element() {
    let test = TestParser::new("type T = (string, readonly EventTarget)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (string, readonly EventTarget)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
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
    let test = TestParser::new("type T = (start: number, end: number)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (start: number, end: number)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { label, value, .. } => {
                    assert_string!(parser, *label, "start");
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { label, value, .. } => {
                    assert_string!(parser, *label, "end");
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
            });
        });
    });
}

/// Record labeled tuple names as their element main spans.
#[test]
fn test_record_labeled_tuple_main_spans() {
    let test = TestParser::new("type T = (start: number, rest: ...string[])");
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    test.assert_no_errors(&parser);

    // read both regular and spread tuple labels
    assert_node!(parser.tree, expression, Expression::Declaration(declaration) => {
        assert_node!(parser.tree, *declaration, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
                let start = parser.tree.get_main_span(elements[0]).expect("start label span");
                let rest = parser.tree.get_main_span(elements[1]).expect("rest label span");

                assert_eq!(parser.span_str(start), "start");
                assert_eq!(parser.span_str(rest), "rest");
            });
        });
    });
}

#[test]
fn test_parse_labeled_tuple_type_complex() {
    let test = TestParser::new("type T = (importCode: string, nameMap: Record<string, string>)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    // type T = (importCode: string, nameMap: Record<string, string>)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
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
fn test_recover_slice_type_missing_close_bracket() {
    let test = TestParser::new("type T = [string");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::TypeExpression),
            Some(TokenType::End),
            Some(TokenType::CloseBracket),
            "",
        )],
    );

    // type T = [string
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Slice { element } => {
                assert_node!(parser.tree, *element, TypeExpression::Keyword { value } => {
                    assert_eq!(*value, TypeLiteral::String);
                });
            });
        });
    });
}

#[test]
fn test_parse_tuple_type_missing_first_element() {
    let test = TestParser::new("type T = (, string)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Statement, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(Some(NodeType::TypeExpression), None, None, ",")],
    );

    // type T = (, string)
    assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
        assert_node!(parser.tree, *decl_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Tuple { elements, .. } => {
                assert_eq!(elements.len(), 2);
                assert_node!(parser.tree, elements[0], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Missing);
                });
                assert_node!(parser.tree, elements[1], TupleElement::Element { value, .. } => {
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value } => {
                        assert_eq!(*value, TypeLiteral::String);
                    });
                });
            });
        });
    });
}
