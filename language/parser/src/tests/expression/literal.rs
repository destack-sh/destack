use crate::tests::*;
use crate::{assert_expression_path, assert_node, assert_path, assert_string};
use destack_ast::*;
use destack_source::LanguageType;

/// Parse a tuple literal with two elements.
#[test]
fn test_parse_tuple_literal() {
    let mut test = TestParser::new("(1, 2)");
    let mut parser = test.prepare();
    let type_expression_id = parser.eat_type_expression().unwrap();
    assert_node!(parser.tree, type_expression_id, TypeExpression::Tuple { elements } => {
            assert_eq!(elements.len(), 2);

            // 1
            assert_node!(
                parser.tree,
                elements[0],
                TupleElement::Element { value, .. } => {
                    assert_node!(
                        parser.tree,
                        *value,
                        TypeExpression::ScalarLiteral { value: ScalarLiteral::Integer(1) }
                    );
                }
            );
            // 2
            assert_node!(
                parser.tree,
                elements[1],
                TupleElement::Element { value, .. } => {
                    assert_node!(
                        parser.tree,
                        *value,
                        TypeExpression::ScalarLiteral { value: ScalarLiteral::Integer(2) }
                    );
                }
            );
    });
}

/// Parse a tuple literal over multiple lines.
#[test]
fn test_parse_tuple_literal_multiline() {
    let mut test = TestParser::new(
        r"
const shapes = (
    TetrisPieceShape.I,
    TetrisPieceShape.J,
    TetrisPieceShape.L,
    TetrisPieceShape.O,
    TetrisPieceShape.S,
)",
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            // const shapes = ...
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "shapes");
            });
            // (TetrisPieceShape.I, ...)
            assert_node!(parser.tree, value.unwrap(), Expression::TupleExpression { elements, .. } => {
                assert_eq!(elements.len(), 5);
                // TetrisPieceShape.I
                assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.I");
                });
            });
        });
    });
}

/// Parse an anonymous block.
#[test]
fn test_parse_anonymous_struct_literal() {
    let mut test = TestParser::new("{ }");
    let mut parser = test.prepare();
    parser.flags.set_in_statement_position(true);
    let expr_id = parser.eat_expression(parser.flags.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block { .. });
}

/// Parse a statement-position object literal with a comment.
#[test]
fn test_parse_statement_position_object_literal_with_comment() {
    let mut test = TestParser::new("{ /* key */ a: 1 }");
    let mut parser = test.prepare();
    parser.flags.set_in_statement_position(true);
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { ty: None, properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

/// Parse a statement-position object literal with a computed key.
#[test]
fn test_parse_statement_position_object_literal_computed_key() {
    let mut test = TestParser::new("{ [key]: value }");
    let mut parser = test.prepare();
    parser.flags.set_in_statement_position(true);
    let expr_id = parser.eat_expression(parser.flags.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { ty: None, properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { key: Key::Expression(key_id), value, .. } => {
            assert_expression_path!(parser, parser.tree.get(*key_id), "key");
            assert_expression_path!(parser, parser.tree.get(*value), "value");
        });
    });
}

/// Parse a parenthesized object literal shorthand field.
#[test]
fn test_parse_parenthesized_object_literal_shorthand_field() {
    let mut test = TestParser::new("({ value })");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
                assert_string!(parser, *name, "value");
                assert!(*is_shorthand);
                assert_expression_path!(parser, parser.tree.get(*value), "value");
            });
        });
    });
}

/// Parse a parenthesized object literal explicit field.
#[test]
fn test_parse_parenthesized_object_literal_explicit_field_is_not_shorthand() {
    let mut test = TestParser::new("({ value: value })");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
                assert_string!(parser, *name, "value");
                assert!(!*is_shorthand);
                assert_expression_path!(parser, parser.tree.get(*value), "value");
            });
        });
    });
}

/// Recover a parenthesized object literal computed field without a value.
#[test]
fn test_parse_parenthesized_object_literal_computed_field_without_value() {
    let mut test = TestParser::new("({ [value] })");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, expr_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Error);
        });
    });
}

/// Parse a statement-position block with assignments.
#[test]
fn test_parse_statement_position_block_with_assignment() {
    let mut test = TestParser::new("{ step = step + 1; return base + step; }");
    let mut parser = test.prepare();
    parser.flags.set_in_statement_position(true);
    let expr_id = parser.eat_expression(parser.flags.in_type()).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block(block_id) => {
        assert_node!(parser.tree, *block_id, Block { leading_expressions, tail_expression, .. } => {
            assert_eq!(leading_expressions.len(), 2);
            assert!(tail_expression.is_none());
            assert_node!(parser.tree, leading_expressions[0], Expression::Assign { .. });
            assert_node!(parser.tree, leading_expressions[1], Expression::Return { .. });
        });
    });
}

/// Parse a statement-position block with an array literal.
#[test]
fn test_parse_statement_position_block_with_array_literal() {
    let mut test = TestParser::new("{ [] }");
    let mut parser = test.prepare();
    parser.flags.set_in_statement_position(true);
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block(block_id) => {
        assert_node!(parser.tree, *block_id, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*block_id));
            assert_eq!(expressions.len(), 1);
            let expression_id = expressions[0];
            assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
                assert!(elements.is_empty());
            });
        });
    });
}

/// Prefer a block over a computed method object literal in statement position.
#[test]
fn test_parse_statement_position_computed_method_as_block() {
    let mut test = TestParser::new("{ [key]()\n{} }");
    let mut parser = test.prepare();
    parser.flags.set_in_statement_position(true);
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block(_) => {});
}

/// Parse an anonymous block with a do disambiguation.
#[test]
fn test_parse_anonymous_block_with_do_disambiguation() {
    let mut test = TestParser::new("let x = do { }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "x");
            });
            assert_node!(parser.tree, value.unwrap(), Expression::Block { .. });
        });
    });
}

/// Parse an object literal in parenthesis.
#[test]
fn test_parse_object_literal_in_parenthesis() {
    let mut test = TestParser::new("({ x: 1, y })");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::Parenthesized { expression } => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 2);
            assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "y");
                assert_expression_path!(parser, parser.tree.get(*value), "y");
            });
        });
    });
}

/// Parse a mixed index postfix expression (should disambiguate ternary and index/call).
#[test]
fn test_parse_mixed_index_call_postfix() {
    let mut test = TestParser::new("x?.[f]?.y<T>?.().?");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // x?.[f]?.y<T>?.().?
    // .?
    assert_node!(parser.tree, expr_id, Expression::Maybe { left, position: PostfixPosition::Indirect } => {
        // ()
        assert_node!(parser.tree, *left, Expression::Call { left, arguments, .. } => {
            assert_eq!(arguments.len(), 0);
            // ?
            assert_node!(parser.tree, *left, Expression::Maybe { left, position: PostfixPosition::Direct } => {
                // .y<T>
                assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
                    assert_eq!(generic_arguments.len(), 1);
                    assert_node!(parser.tree, *left, Expression::Member { left, name } => {
                        assert_string!(parser, *name, "y");
                        assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                            // .[f]
                            assert_node!(parser.tree, *left, Expression::Index { left, index, position: PostfixPosition::Indirect } => {
                                // f
                                assert_node!(parser.tree, index.unwrap(), Expression::Identifier { name } => {
                                    assert_string!(parser, *name, "f");
                                });
                                // ?
                                assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                                    // x
                                    assert_expression_path!(parser, parser.tree.get(*left), "x");
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Parse a singleton parenthesized type tuple.
#[test]
fn test_parse_singleton_tuple_literal() {
    let mut test = TestParser::new("(string,)");
    let mut parser = test.prepare();
    let type_expression_id = parser.eat_type_expression().unwrap();

    assert_node!(parser.tree, type_expression_id, TypeExpression::Tuple { elements } => {
        assert_eq!(elements.len(), 1);
    });
}

/// Parse a parenthesized type tuple with one spread element.
#[test]
fn test_parse_spread_tuple_literal() {
    let mut test = TestParser::new("(...PlatformCapability[])");
    let mut parser = test.prepare();
    let type_expression_id = parser.eat_type_expression().unwrap();

    assert_node!(parser.tree, type_expression_id, TypeExpression::Tuple { elements } => {
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], TupleElement::Spread { value, .. } => {
            assert_node!(parser.tree, *value, TypeExpression::Array { .. });
        });
    });
}

/// Parse an empty parenthesis as a tuple literal.
#[test]
fn test_parse_empty_parenthesis_tuple() {
    let mut test = TestParser::new("()");
    let mut parser = test.prepare();
    let type_expression_id = parser.eat_type_expression().unwrap();
    assert_node!(parser.tree, type_expression_id, TypeExpression::Tuple { elements } => {
        assert_eq!(elements.len(), 0);
    });
}

/// Parse a struct literal with a path type and two fields.
#[test]
fn test_parse_struct_literal_path() {
    let mut test = TestParser::new("geom.Vector2 { x: 1, y }");
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(
        parser.tree,
        expr_id,
        Expression::ObjectExpression { ty: Some(ty), properties, .. } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                assert_path!(parser, *path, "geom.Vector2");
            });
            assert_eq!(properties.len(), 2);
            assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
                    assert_eq!(*val, 1);
                });
            });
            assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "y");
                assert_expression_path!(parser, parser.tree.get(*value), "y");
            });
        }
    );

    test.assert_no_errors(&parser);
}

/// Parse a struct literal with generic parameters and two fields.
#[test]
fn test_parse_struct_literal_path_with_generic_parameters() {
    let mut test = TestParser::new_with_language(
        r##"
geom.Mesh<2, 4> {
    vertices: [1, 2],
    y,
}"##,
        LanguageType::Destack,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(
        parser.tree,
        expr_id,
        Expression::ObjectExpression { ty: Some(ty), properties, .. } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "geom.Mesh");
                assert_eq!(generic_arguments.len(), 2);
            });
            assert_eq!(properties.len(), 2);
            assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "vertices");
                assert_node!(parser.tree, *value, Expression::ArrayExpression { .. });
            });
            assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                assert_string!(parser, *name, "y");
                assert_expression_path!(parser, parser.tree.get(*value), "y");
            });
        }
    );

    test.assert_no_errors(&parser);
}

/// Parse boolean IdentifierName property keys and accessors in JavaScript.
#[test]
fn test_parse_object_boolean_identifier_name_keys() {
    let mut test = TestParser::new_with_language(
        "{ true: 1, false: 2, get true() {}, set false(value) {} }",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 4);

        assert_node!(parser.tree, properties[0], Property::Field { key, value, .. } => {
            assert_node!(key, Key::Name(Name::Identifier(name)) => {
                assert_string!(parser, *name, "true");
            });
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });

        assert_node!(parser.tree, properties[1], Property::Field { key, value, .. } => {
            assert_node!(key, Key::Name(Name::Identifier(name)) => {
                assert_string!(parser, *name, "false");
            });
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });

        assert_node!(parser.tree, properties[2], Property::Method { key, signature, .. } => {
            assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                assert_string!(parser, *name, "true");
            });
            assert_eq!(signature.mode, Some(FunctionMode::Getter));
        });

        assert_node!(parser.tree, properties[3], Property::Method { key, signature, .. } => {
            assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                assert_string!(parser, *name, "false");
            });
            assert_eq!(signature.mode, Some(FunctionMode::Setter));
        });
    });
}

#[test]
fn test_parse_object_literal_with_typed_arrow_value() {
    let language = LanguageType::TypeScript;
    let mut test = TestParser::new_with_language(
        r#"{
    reproFunc: (_: any): any => { },
}"#,
        language,
    );
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
            assert_string!(parser, *name, "reproFunc");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(_), .. }) => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert_eq!(signature.parameters.len(), 1);
                });
            });
        });
    });
}

/// Comma in parentheses parses as tuple expression.
#[test]
fn test_parse_tuple_expression() {
    let language = LanguageType::Destack;
    let mut test = TestParser::new_with_language("(a, b, c)", language);
    let mut parser = test.prepare();
    let expr_id = parser.eat_expression(parser.flags).unwrap();

    // (a, b, c)
    assert_node!(parser.tree, expr_id, Expression::TupleExpression { elements } => {
        assert_eq!(elements.len(), 3);

        // a
        assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "a");
        });

        // b
        assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "b");
        });

        // c
        assert_node!(parser.tree, elements[2], Argument::Positional { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "c");
        });
    });
}
