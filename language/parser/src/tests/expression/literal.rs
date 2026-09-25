use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::tests::{TestParser, block_expression_ids};
use crate::{
    TypePosition, TypeStop, assert_expression_path, assert_node, assert_path, assert_string,
};
use tspp_dir::{
    Argument, Block, Declaration, Declarator, Expression, FunctionDeclaration, FunctionForm,
    InferForm, Literal, Name, NodeType, Pattern, PostfixPosition, Property, TokenType,
    TupleElement, TypeExpression,
};
use tspp_source::{NodeSpanList, NodeSpanType};

/// Parse a tuple literal with two elements.
#[test]
fn test_parse_tuple_literal() {
    let test = TestParser::new("(1, 2)");
    let mut parser = test.prepare();
    let type_expression_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();
    assert_node!(parser.tree, type_expression_id, TypeExpression::Tuple { elements, .. } => {
            assert_eq!(elements.len(), 2);

            // 1
            assert_node!(
                parser.tree,
                elements[0],
                TupleElement::Element { value, .. } => {
                    assert_node!(
                        parser.tree,
                        *value,
                        TypeExpression::Literal { value: Literal::Integer(1) }
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
                        TypeExpression::Literal { value: Literal::Integer(2) }
                    );
                }
            );
    });
}

/// Parse a singleton tuple literal with a required trailing comma.
#[test]
fn test_parse_singleton_tuple_expression_literal() {
    let test = TestParser::new("const value = (1,)");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
            assert_node!(
                parser.tree,
                value.expect("expected initializer"),
                Expression::TupleExpression { elements }
            => {
                assert_eq!(elements.len(), 1);
                assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
                });
            });
        });
    });
}

/// Parse a tuple literal over multiple lines.
#[test]
fn test_parse_tuple_literal_multiline() {
    let test = TestParser::new(
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
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("{ }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Block, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::Block { .. });
}

/// Parse a statement-position object literal with a comment.
#[test]
fn test_parse_statement_position_object_literal_with_comment() {
    let test = TestParser::new("{ /* key */ a: 1 }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
    });
}

/// Parse a parenthesized object literal shorthand field.
#[test]
fn test_parse_parenthesized_object_literal_shorthand_field() {
    let test = TestParser::new("({ value })");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    crate::assert_parenthesized!(parser.tree, expr_id, expression => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, is_shorthand } => {
                assert_string!(parser, *name, "value");
                assert!(*is_shorthand);
                assert_expression_path!(parser, parser.tree.get(*value), "value");
                let main_span = parser
                    .tree
                    .get_main_span(*value)
                    .expect("expected shorthand value main span");
                assert_eq!(parser.span_str(main_span), "value");
            });
        });
    });
}

/// Parse a parenthesized object literal explicit field.
#[test]
fn test_parse_parenthesized_object_literal_explicit_field_is_not_shorthand() {
    let test = TestParser::new("({ value: value })");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    crate::assert_parenthesized!(parser.tree, expr_id, expression => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, is_shorthand } => {
                assert_string!(parser, *name, "value");
                assert!(!*is_shorthand);
                assert_expression_path!(parser, parser.tree.get(*value), "value");
            });
        });
    });
}

/// Parse a statement-position block with assignments.
#[test]
fn test_parse_statement_position_block_with_assignment() {
    let test = TestParser::new("{ step = step + 1; return base + step; }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Block, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("{ [] }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Block, ExpressionStop::default())
        .unwrap();
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

/// Parse a fixed array repeat literal.
#[test]
fn test_parse_fixed_array_literal() {
    let test = TestParser::new("[0; 32]");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // [0; 32]
    assert_node!(parser.tree, expression_id, Expression::FixedArrayExpression { value, length } => {
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(0)));
        assert_node!(parser.tree, *length, Expression::Literal(Literal::Integer(32)));
    });
}

/// Recover a missing fixed array repeat value.
#[test]
fn test_parse_fixed_array_literal_recovers_missing_value() {
    let test = TestParser::new("[; 32]");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, ";")]);
    assert_node!(parser.tree, expression_id, Expression::FixedArrayExpression { value, length } => {
        assert_node!(parser.tree, *value, Expression::Missing);
        assert_node!(parser.tree, *length, Expression::Literal(Literal::Integer(32)));
    });
}

/// Recover a missing fixed array repeat length.
#[test]
fn test_parse_fixed_array_literal_recovers_missing_length() {
    let test = TestParser::new("[0; ]");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "]")]);
    assert_node!(parser.tree, expression_id, Expression::FixedArrayExpression { value, length } => {
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(0)));
        assert_node!(parser.tree, *length, Expression::Missing);
    });
}

/// Recover a missing fixed array close bracket.
#[test]
fn test_parse_fixed_array_literal_recovers_missing_close_bracket() {
    let test = TestParser::new("[0; 32");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::Expression),
            Some(TokenType::End),
            Some(TokenType::CloseBracket),
            "",
        )],
    );
    assert_node!(parser.tree, expression_id, Expression::FixedArrayExpression { value, length } => {
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(0)));
        assert_node!(parser.tree, *length, Expression::Literal(Literal::Integer(32)));
    });
}

/// Parse an anonymous block with a do disambiguation.
#[test]
fn test_parse_anonymous_block_with_do_disambiguation() {
    let test = TestParser::new("let x = do { }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
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
    let test = TestParser::new("({ x: 1, y })");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    crate::assert_parenthesized!(parser.tree, expr_id, expression => {
        assert_node!(parser.tree, *expression, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 2);
            assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
            });
            assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "y");
                assert_expression_path!(parser, parser.tree.get(*value), "y");
            });
        });
    });
}

#[test]
fn test_parse_mixed_index_call_postfix() {
    let test = TestParser::new("x?.[f]?.y<T>?.().?");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // x?.[f]?.y<T>?.().?
    // the whole postfix spine carries one optional access (`?.`), so it is
    // wrapped once in Chain; the trailing `.?` is the indirect try operator,
    // never part of the chain itself
    assert_node!(parser.tree, expr_id, Expression::Chain { expression } => {
        // .?
        assert_node!(parser.tree, *expression, Expression::Maybe { left, position: PostfixPosition::Indirect } => {
            // ?.()
            assert_node!(parser.tree, *left, Expression::Call { left, arguments, position: PostfixPosition::Indirect, is_optional, .. } => {
                assert!(*is_optional);
                assert_eq!(arguments.len(), 0);
                // .y<T>
                assert_node!(parser.tree, *left, Expression::Instantiation { left, generic_arguments } => {
                    assert_eq!(generic_arguments.len(), 1);
                    // ?.y
                    assert_node!(parser.tree, *left, Expression::Member { left, name, is_optional } => {
                        assert_string!(parser, *name, "y");
                        assert!(*is_optional);
                        // ?.[f]
                        assert_node!(parser.tree, *left, Expression::Index { left, index, position: PostfixPosition::Indirect, is_optional } => {
                            assert!(*is_optional);
                            // f
                            assert_node!(parser.tree, index.unwrap(), Expression::Identifier { name } => {
                                assert_string!(parser, *name, "f");
                            });
                            // x
                            assert_expression_path!(parser, parser.tree.get(*left), "x");
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
    let test = TestParser::new("(string,)");
    let mut parser = test.prepare();
    let type_expression_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();

    assert_node!(parser.tree, type_expression_id, TypeExpression::Tuple { elements, .. } => {
        assert_eq!(elements.len(), 1);
    });
}

/// Parse a parenthesized type tuple with one spread element.
#[test]
fn test_parse_spread_tuple_literal() {
    let test = TestParser::new("(...PlatformCapability[])");
    let mut parser = test.prepare();
    let type_expression_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();

    assert_node!(parser.tree, type_expression_id, TypeExpression::Tuple { elements, .. } => {
        assert_eq!(elements.len(), 1);
        assert_node!(parser.tree, elements[0], TupleElement::Spread { value, .. } => {
            assert_node!(parser.tree, *value, TypeExpression::Array { .. });
        });
    });
}

/// Parse an empty parenthesis as a tuple literal.
#[test]
fn test_parse_empty_parenthesis_tuple() {
    let test = TestParser::new("()");
    let mut parser = test.prepare();
    let type_expression_id = parser
        .parse_type(TypePosition::Type, TypeStop::default())
        .unwrap();
    assert_node!(parser.tree, type_expression_id, TypeExpression::Tuple { elements, .. } => {
        assert_eq!(elements.len(), 0);
    });
}

/// Parse a struct literal with a path type and two fields.
#[test]
fn test_parse_struct_literal_path() {
    let test = TestParser::new("geom.Vector2 { x: 1, y }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(
        parser.tree,
        expr_id,
        Expression::StructExpression { ty, properties, .. } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                assert_path!(parser, *path, "geom.Vector2");

                let root_span = parser
                    .tree
                    .get_side_span(*ty, NodeSpanType::ListItem(NodeSpanList::Segment, 0))
                    .unwrap();
                let member_span = parser
                    .tree
                    .get_side_span(*ty, NodeSpanType::ListItem(NodeSpanList::Segment, 1))
                    .unwrap();
                assert_eq!(parser.span_str(root_span), "geom");
                assert_eq!(parser.span_str(member_span), "Vector2");
            });
            assert_eq!(properties.len(), 2);
            assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(val)) => {
                    assert_eq!(*val, 1);
                });
            });
            assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "y");
                assert_expression_path!(parser, parser.tree.get(*value), "y");
            });
        }
    );

    test.assert_no_errors(&parser);
}

/// Parse an inferred struct literal as a typed value hole.
#[test]
fn test_parse_struct_literal_infer_hole() {
    let test = TestParser::new("_ { x: 1 }");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(
        parser.tree,
        expr_id,
        Expression::StructExpression { ty, properties, .. } => {
            assert_node!(parser.tree, *ty, TypeExpression::Infer { form, name, constraint } => {
                assert_eq!(*form, InferForm::Hole);
                assert!(name.is_none());
                assert!(constraint.is_none());
            });
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
            });
        }
    );

    test.assert_no_errors(&parser);
}

/// Parse a struct literal with generic parameters and two fields.
#[test]
fn test_parse_struct_literal_path_with_generic_parameters() {
    let test = TestParser::new(
        r##"
geom.Mesh<2, 4> {
    vertices: [1, 2],
    y,
}"##,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(
        parser.tree,
        expr_id,
        Expression::StructExpression { ty, properties, .. } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments } => {
                assert_path!(parser, *path, "geom.Mesh");
                assert_eq!(generic_arguments.len(), 2);
            });
            assert_eq!(properties.len(), 2);
            assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "vertices");
                assert_node!(parser.tree, *value, Expression::ArrayExpression { .. });
            });
            assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, .. } => {
                assert_string!(parser, *name, "y");
                assert_expression_path!(parser, parser.tree.get(*value), "y");
            });
        }
    );

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_object_literal_with_typed_arrow_value() {
    let test = TestParser::new(
        r#"{
    reproFunc: (_: unknown): unknown => { },
}"#,
    );
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
        assert_eq!(properties.len(), 1);
        assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
            assert_string!(parser, *name, "reproFunc");
            assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body: Some(_), .. }) => {
                    assert_eq!(signature.form, FunctionForm::Lambda);
                    assert_eq!(signature.parameters.len(), 1);
                });
            });
        });
    });
}

/// Comma in parentheses parses as tuple expression.
#[test]
fn test_parse_tuple_expression() {
    let test = TestParser::new("(a, b, c)");
    let mut parser = test.prepare();
    let expr_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

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
