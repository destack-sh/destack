use crate::tests::{TestParser, block_expression_ids};
use crate::{
    ExpressionPosition, ExpressionStop, assert_comment, assert_expression_path, assert_node,
    assert_string,
};
use tspp_dir::{
    AssignOperator, BinaryOperator, Block, CommentKind, Declaration, Declarator, ExportKind,
    Expression, FunctionDeclaration, FunctionForm, IfForm, Literal, Name, Pattern, Property,
};

/// Parse `true ? 1 : 2`.
#[test]
fn test_parse_if_ternary() {
    let test = TestParser::new("true ? 1 : 2");
    let mut parser = test.prepare();
    let if_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Literal(Literal::Boolean(true)));
        assert_node!(parser.tree, *then_expression, Expression::Literal(Literal::Integer(1)));
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Literal(Literal::Integer(2)));
    });
}

/// Parse assignment and a nested conditional in the false branch.
#[test]
fn test_parse_ternary_false_branch_assignment() {
    let test = TestParser::new("condition ? first : target = nested ? yes : no");
    let mut parser = test.prepare();
    let expression = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression, Expression::If { else_expression: Some(else_expression), .. } => {
        assert_node!(parser.tree, *else_expression, Expression::Assign { operator, left, right } => {
            assert_eq!(*operator, AssignOperator::Assign);
            assert_expression_path!(parser, parser.tree.get(*left), "target");
            assert_node!(parser.tree, *right, Expression::If { condition, then_expression, else_expression: Some(else_expression), .. } => {
                let condition = condition.as_expression().expect("expected expression condition");
                assert_expression_path!(parser, parser.tree.get(condition), "nested");
                assert_expression_path!(parser, parser.tree.get(*then_expression), "yes");
                assert_expression_path!(parser, parser.tree.get(*else_expression), "no");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Keep statement-position identifier and array ternaries out of object parsing.
#[test]
fn test_parse_statement_position_ternaries() {
    let test = TestParser::new("{ condition ? first : second; [condition] ? third : fourth; }");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);

    assert_node!(parser.tree, expressions[0], Expression::Block(block_id) => {
        let expressions = block_expression_ids(parser.tree.get(*block_id));
        assert_eq!(expressions.len(), 2);

        assert_node!(parser.tree, expressions[0], Expression::If { condition, then_expression, else_expression: Some(else_expression), .. } => {
            let condition_id = condition.as_expression().expect("expected expression condition");
            assert_expression_path!(parser, parser.tree.get(condition_id), "condition");
            assert_expression_path!(parser, parser.tree.get(*then_expression), "first");
            assert_expression_path!(parser, parser.tree.get(*else_expression), "second");
        });

        assert_node!(parser.tree, expressions[1], Expression::If { condition, then_expression, else_expression: Some(else_expression), .. } => {
            let condition_id = condition.as_expression().expect("expected expression condition");
            assert_node!(parser.tree, condition_id, Expression::ArrayExpression { elements } => {
                assert_eq!(elements.len(), 1);
            });
            assert_expression_path!(parser, parser.tree.get(*then_expression), "third");
            assert_expression_path!(parser, parser.tree.get(*else_expression), "fourth");
        });
    });
}

/// Parse an arrow expression as the false branch.
#[test]
fn test_parse_ternary_arrow_else_expression() {
    let test = TestParser::new("ready ? value : item => item");
    let mut parser = test.prepare();
    let if_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, if_id, Expression::If { form, else_expression, .. } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
            });
        });
    });
    test.assert_no_errors(&parser);
}

/// Parse multiline `true ? 1 : 2`.
#[test]
fn test_parse_if_ternary_multiline() {
    let test = TestParser::new(
        r#"true
    ? 1
    : 2"#,
    );
    let mut parser = test.prepare();
    let if_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Literal(Literal::Boolean(true)));
        assert_node!(parser.tree, *then_expression, Expression::Literal(Literal::Integer(1)));
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Literal(Literal::Integer(2)));
    });
}

/// Parse multiline `cond ? a : b` with comment-only branch boundaries.
#[test]
fn test_parse_if_ternary_multiline_with_comments() {
    let test = TestParser::new(
        r#"
 cond
    ? // comment
      a
    : // comment
      b"#,
    );
    let mut parser = test.prepare();

    let if_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");

        // cond
        assert_expression_path!(parser, parser.tree.get(condition_id), "cond");

        // a
        assert_expression_path!(parser, parser.tree.get(*then_expression), "a");

        // b
        assert_expression_path!(parser, parser.tree.get(else_expression.unwrap()), "b");
    });
}

/// Keep `// then-boundary` and `// else-boundary` on ternary branch boundaries.
#[test]
fn test_parse_if_ternary_boundary_comments_attach_to_branch_owners() {
    let test = TestParser::new("cond ? // then-boundary\nleft : // else-boundary\nright");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let else_expression_id = else_expression.expect("expected ternary else branch");
        let condition_id = condition.as_expression().expect("unexpected ternary let condition");

        let condition_annotations = parser.tree.get_decorators(condition_id.id);
        assert!(condition_annotations.is_empty());

        let then_annotations = parser.tree.get_decorators(then_expression.id);
        assert!(then_annotations.is_empty());

        let else_annotations = parser.tree.get_decorators(else_expression_id.id);
        assert!(else_annotations.is_empty());

        let _ = then_expression;
        let _ = else_expression_id;
    });

    assert_eq!(parser.comments().len(), 2);
    assert_comment!(parser, 0, CommentKind::Line, "then-boundary");
    assert_comment!(parser, 1, CommentKind::Line, "else-boundary");
}

/// Keep inline branch comments outside ternary branch spans.
#[test]
fn test_parse_if_ternary_inline_branch_comment_keeps_branch_token_span() {
    let test = TestParser::new("condition ? null /* branch-note */ : other");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::If { then_expression, else_expression, .. } => {
        let then_span = parser.tree.get_span(*then_expression);
        let then_head_span = parser
            .tree
            .get_head_span(*then_expression)
            .unwrap_or(then_span);
        let else_expression = else_expression.expect("expected ternary else branch");
        let else_span = parser.tree.get_span(else_expression);

        assert_eq!(parser.span_str(then_span), "null");
        assert_eq!(parser.span_str(then_head_span), "null");
        assert_eq!(parser.span_str(else_span), "other");
    });

    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " branch-note");
}

/// Keep nested tree branch spans on the branch tokens.
#[test]
fn test_parse_if_ternary_nested_tree_branches_keep_token_spans() {
    let test = TestParser::new("condition ? null /* branch-note */ : other ? <A /> : <B />");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    parser.finalize_comments();

    assert_node!(parser.tree, expression_id, Expression::If { else_expression, .. } => {
        let nested_id = else_expression.expect("expected nested else ternary");
        assert_node!(parser.tree, nested_id, Expression::If { then_expression, else_expression, .. } => {
            let then_span = parser.tree.get_span(*then_expression);
            let then_head_span = parser
                .tree
                .get_head_span(*then_expression)
                .unwrap_or(then_span);
            let else_expression = else_expression.expect("expected nested else branch");
            let else_span = parser.tree.get_span(else_expression);
            let else_head_span = parser
                .tree
                .get_head_span(else_expression)
                .unwrap_or(else_span);

            assert_eq!(parser.span_str(then_span), "<A />");
            assert_eq!(parser.span_str(then_head_span), "<A />");
            assert_eq!(parser.span_str(else_span), "<B />");
            assert_eq!(parser.span_str(else_head_span), "<B />");
        });
    });
}

/// Parse a long right-associative ternary ladder.
#[test]
fn test_parse_long_ternary_ladder() {
    let mut source = String::from("flag0");

    for index in 0..2_100 {
        source.push_str(&format!(" ? value{index} : flag{}", index + 1));
    }

    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::If { .. });

    test.assert_no_errors(&parser);
}

/// Parse `x ? () : ()`.
#[test]
fn test_parse_if_ternary_with_parenthesis() {
    let test = TestParser::new("x ? () : ()");
    let mut parser = test.prepare();
    let if_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Identifier { name } => {
            assert_string!(parser, *name, "x");
        });
        assert_node!(parser.tree, *then_expression, Expression::TupleExpression { elements, .. } => {
            assert_eq!(elements.len(), 0);
        });
        assert_node!(parser.tree, else_expression.unwrap(), Expression::TupleExpression { elements, .. } => {
            assert_eq!(elements.len(), 0);
        });
    });
}

/// Parse `x ? [] : []`.
#[test]
fn test_parse_if_ternary_with_brackets() {
    let test = TestParser::new("x ? [] : []");
    let mut parser = test.prepare();
    let if_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Identifier { name } => {
            assert_string!(parser, *name, "x");
        });
        assert_node!(parser.tree, *then_expression, Expression::ArrayExpression { elements } => {
            assert_eq!(elements.len(), 0);
        });
        assert_node!(parser.tree, else_expression.unwrap(), Expression::ArrayExpression { elements } => {
            assert_eq!(elements.len(), 0);
        });
    });
}

/// Parse `x ? {} : {}`.
#[test]
fn test_parse_if_ternary_with_braces() {
    let test = TestParser::new("x ? {} : {}");
    let mut parser = test.prepare();
    let if_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Identifier { name } => {
            assert_string!(parser, *name, "x");
        });
        assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 0);
        });
        assert_node!(parser.tree, else_expression.unwrap(), Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 0);
        });
    });
}

/// Parse `x == 0 ? 1 : 2`.
#[test]
fn test_parse_if_ternary_with_binary_condition() {
    let test = TestParser::new("x == 0 ? 1 : 2");
    let mut parser = test.prepare();
    let if_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::Equal);
            assert_node!(parser.tree, *left, Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
            });
            assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(0)));
        });
        assert_node!(parser.tree, *then_expression, Expression::Literal(Literal::Integer(1)));
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Literal(Literal::Integer(2)));
    });
}

#[test]
fn test_parse_export_const_ternary_object_literal_arrow_value() {
    let test = TestParser::new(
        r#"export const reproValue = true ? {} : {
    reproFunc: (_: unknown): unknown => { },
};"#,
    );
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_node!(parser.tree, expression_id, Expression::Let { export, declarators, .. } => {
        assert_eq!(*export, Some(ExportKind::Named));
        assert_eq!(declarators.len(), 1);
        assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern, .. } => {
                assert_string!(parser, *name, "reproValue");
                assert!(pattern.is_none());
            });
            let value_id = value.expect("expected initializer");
            assert_node!(parser.tree, value_id, Expression::If { form, then_expression, else_expression, .. } => {
                assert_eq!(*form, IfForm::Ternary);
                assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
                    assert!(properties.is_empty());
                });
                let else_expression = else_expression.expect("expected else branch");
                assert_node!(parser.tree, else_expression, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { name, value, .. } => {
                        assert_node!(name, Name::Identifier(name) => {
                            assert_string!(parser, *name, "reproFunc");
                        });
                        assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                                assert_eq!(signature.form, FunctionForm::Lambda);
                                let body_id = body.expect("expected function body");
                                assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                                    assert_node!(parser.tree, *block_id, Block { leading_expressions, tail_expression, .. } => {
                                        assert!(leading_expressions.is_empty());
                                        assert!(tail_expression.is_none());
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_ternary_object_literal_arrow_value_expression() {
    let test = TestParser::new(
        r#"true ? {} : {
    reproFunc: (_: unknown): unknown => { },
}"#,
    );
    let mut parser = test.prepare();
    let result = parser.parse_expression(ExpressionPosition::Value, ExpressionStop::default());
    match result {
        Ok(expr_id) => {
            assert_node!(parser.tree, expr_id, Expression::If { form, then_expression, else_expression, .. } => {
                assert_eq!(*form, IfForm::Ternary);
                assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
                    assert!(properties.is_empty());
                });
                let else_expression = else_expression.expect("expected else branch");
                assert_node!(parser.tree, else_expression, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 1);
                    assert_node!(parser.tree, properties[0], Property::Field { name, value, .. } => {
                        assert_node!(name, Name::Identifier(name) => {
                            assert_string!(parser, *name, "reproFunc");
                        });
                        assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                                assert_eq!(signature.form, FunctionForm::Lambda);
                                let body_id = body.expect("expected function body");
                                assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                                    assert_node!(parser.tree, *block_id, Block { leading_expressions, tail_expression, .. } => {
                                        assert!(leading_expressions.is_empty());
                                        assert!(tail_expression.is_none());
                                    });
                                });
                            });
                        });
                    });
                });
            });
        }
        Err(err) => panic!("unexpected error: {err:?}"),
    }
}

#[test]
fn test_parse_assignment_object_spread_ternary_value() {
    let test = TestParser::new("target = { ...tls ? { cert: tls.cert } : {}, ...node }");
    let mut parser = test.prepare();

    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right } => {
        assert_eq!(*operator, AssignOperator::Assign);
        assert_expression_path!(parser, parser.tree.get(*left), "target");

        assert_node!(parser.tree, *right, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 2);

            // ...(tls ? { tls } : {})
            assert_node!(parser.tree, properties[0], Property::Spread { value, .. } => {
                assert_node!(parser.tree, *value, Expression::If {
                    form: IfForm::Ternary,
                    condition,
                    then_expression,
                    else_expression,
                } => {
                    let condition = condition.as_expression().expect("expected expression condition");
                    assert_expression_path!(parser, parser.tree.get(condition), "tls");
                    assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 1);
                    });
                    let else_expression = else_expression.expect("expected ternary else branch");
                    assert_node!(parser.tree, else_expression, Expression::ObjectExpression { properties, .. } => {
                        assert!(properties.is_empty());
                    });
                });
            });

            // ...node
            assert_node!(parser.tree, properties[1], Property::Spread { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "node");
            });
        });
    });
}
