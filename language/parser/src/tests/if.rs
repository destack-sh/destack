use tspp_dir::{
    BinaryOperator, Block, CommentKind, ConditionOperand, Declaration, Declarator, Expression,
    FunctionDeclaration, FunctionForm, LetKind, Literal, Mutability, Pattern, PatternField,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::{
    TestParser, assert_comment, assert_expression_path, assert_name, assert_node, assert_string,
    block_expression_ids,
};

#[test]
fn test_parse_if_basic() {
    let test = TestParser::new("if (true) {}");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    let main_span = parser
        .tree
        .get_main_span(if_id)
        .expect("expected if keyword main span");
    assert_eq!(parser.span_str(main_span), "if");

    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        // condition is boolean true
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Literal(Literal::Boolean(true)));
        // empty then block
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert!(expressions.is_empty());
            });
        });
    });
}

#[test]
fn test_parse_if_else_with_empty_blocks() {
    let test = TestParser::new("if (false) {} else {}");
    let mut parser = test.prepare();

    // if (false) { } else { }
    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // false
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Literal(Literal::Boolean(false)));
        // { }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert!(expressions.is_empty());
            });
        });
        // { }
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert!(expressions.is_empty());
            });
        });
    });
}

#[test]
fn test_parse_if_else_records_else_clause_span() {
    let test = TestParser::new(
        r#"
if (ready) {
    run()
}
// boundary
else {
    stop()
}
"#,
    );
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    let else_span = parser
        .tree
        .get_side_span(if_id, NodeSpanType::Region(NodeSpanRegion::Else))
        .expect("expected else clause span");

    assert_eq!(parser.span_str(else_span), "else");
}

#[test]
fn test_parse_if_else_parenthesized_with_trivial_blocks() {
    let test = TestParser::new("if (cond) { a } else { b }");
    let mut parser = test.prepare();

    // if (cond) { a } else { b }
    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // cond
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_expression_path!(parser, parser.tree.get(condition_id), "cond");
        // { a }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let then_statement_id = expressions[0];
                assert_expression_path!(parser, parser.tree.get(then_statement_id), "a");
            });
        });
        // { b }
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let else_statement_id = expressions[0];
                assert_expression_path!(parser, parser.tree.get(else_statement_id), "b");
            });
        });
    });
}

#[test]
fn test_parse_if_parenthesized_condition_keeps_inner_span() {
    let test = TestParser::new("if (cond) {}");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");

        assert_expression_path!(parser, parser.tree.get(condition_id), "cond");

        let condition_span = parser.tree.get_span(condition_id);
        let condition_text = &parser.file.text()
            [condition_span.start as usize..condition_span.end as usize];
        assert_eq!(condition_text, "cond");
    });
}

#[test]
fn test_parse_if_with_nested_parenthesized_condition() {
    let test = TestParser::new(
        r"
if (cond) {
    if (cond) {
        a
    } else {
        b
    }
}
",
    );
    let mut parser = test.prepare();

    // if (cond) { if (cond) { a } else { b } }
    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // cond
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_expression_path!(parser, parser.tree.get(condition_id), "cond");
        // { if (cond) { a } else { b } }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                // if (cond) { a } else { b }
                let inner_if_id = expressions[0];
                assert_node!(parser.tree, inner_if_id, Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
                    // cond
                    let inner_condition_id = inner_condition.as_expression().expect("expected expression condition");
                    assert_expression_path!(parser, parser.tree.get(inner_condition_id), "cond");
                    // { a }
                    assert_node!(parser.tree, *inner_then, Expression::Block(inner_block_id) => {
                        assert_node!(parser.tree, *inner_block_id, Block { .. } => {
                            let expressions = block_expression_ids(parser.tree.get(*inner_block_id));
                            assert_eq!(expressions.len(), 1);
                            let inner_then_statement_id = expressions[0];
                            assert_expression_path!(parser, parser.tree.get(inner_then_statement_id), "a");
                        });
                    });
                    // { b }
                    assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(inner_block_id) => {
                        assert_node!(parser.tree, *inner_block_id, Block { .. } => {
                            let expressions = block_expression_ids(parser.tree.get(*inner_block_id));
                            assert_eq!(expressions.len(), 1);
                            let inner_else_statement_id = expressions[0];
                            assert_expression_path!(parser, parser.tree.get(inner_else_statement_id), "b");
                        });
                    });
                });
            });
        });
        assert!(else_expression.is_none());
    });
}

#[test]
fn test_parse_if_else_if_with_empty_blocks() {
    let test = TestParser::new("if (true) {} else if (false) {}");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // true
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Literal(Literal::Boolean(true)));
        // then block
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert!(expressions.is_empty());
            });
        });
        // nested else-if should be simple If
        assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, .. } => {
            let inner_condition_id = inner_condition.as_expression().expect("expected expression condition");
            assert_node!(parser.tree, inner_condition_id, Expression::Literal(Literal::Boolean(false)));
            assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
        });
    });
}

#[test]
fn test_parse_if_else_if_ambiguous() {
    // ambiguous because y and z could be interpreted as struct literals
    // this is disambiguated in a condition / guard clause
    let test = TestParser::new(
        r"
if (x > y) {
    y
} else if (y == z) {
    x
}",
    );
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // if x > y
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::GreaterThan);
            // x
            assert_expression_path!(parser, parser.tree.get(*left), "x");
            // y
            assert_expression_path!(parser, parser.tree.get(*right), "y");
        });
        // { y }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
            });
        });
        // else if y == z
        assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, .. } => {
            // y == z
            let inner_condition_id = inner_condition.as_expression().expect("expected expression condition");
            assert_node!(parser.tree, inner_condition_id, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Equal);
                // y
                assert_expression_path!(parser, parser.tree.get(*left), "y");
                // z
                assert_expression_path!(parser, parser.tree.get(*right), "z");
            });
            // { x }
            assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
        });
    });
}

#[test]
fn test_parse_if_else_if_else_with_empty_blocks() {
    let test = TestParser::new("if (true) {} else if (false) {} else {}");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // if true
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Literal(Literal::Boolean(true)));
        // { }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert!(expressions.is_empty());
            });
        });
        // else if (false) { } else { }
        assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
            // else if (false)
            let inner_condition_id = inner_condition.as_expression().expect("expected expression condition");
            assert_node!(parser.tree, inner_condition_id, Expression::Literal(Literal::Boolean(false)));
            // { }
            assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
            // else { }
            assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
        });
    });
}

#[test]
fn test_parse_if_else_if_else_multiline() {
    let test = TestParser::new(
        r"
if (v < lo) { lo }
else if (v > hi) { hi }
else { v }
",
    );
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // if (v < lo)
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::LessThan);
            // v
            assert_expression_path!(parser, parser.tree.get(*left), "v");
            // lo
            assert_expression_path!(parser, parser.tree.get(*right), "lo");
        });
        // { lo }
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
            });
        });
        // else if (v > hi) { hi } else { v }
        assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
            // else if (v > hi)
            let inner_condition_id = inner_condition.as_expression().expect("expected expression condition");
            assert_node!(parser.tree, inner_condition_id, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                // v
                assert_expression_path!(parser, parser.tree.get(*left), "v");
                // hi
                assert_expression_path!(parser, parser.tree.get(*right), "hi");
            });
            // { hi }
            assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
            // else { v }
            assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
        });
    });
}

#[test]
fn test_parse_if_with_expression_condition() {
    let test = TestParser::new(
        r###"
if (x > y) {
    print("positive")
}
"###,
    );
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        // condition is binary expression x > y
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition_id, Expression::Binary { left: _, operator: _, right: _ });
        // then block has one expression
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
            });
        });
    });
}

/// If the then expression is not a block, it should be coerced to a block.
#[test]
fn test_parse_if_else_if_coerce_to_block() {
    let test = TestParser::new(
        r###"
if (x)
    x
else if (y)
    y
else
    z
"###,
    );
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        // if (x)
        let condition_id = condition.as_expression().expect("expected expression condition");
        assert_expression_path!(parser, parser.tree.get(condition_id), "x");
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
            });
        });
        // else if (y)
        assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition, then_expression, else_expression, .. } => {
            let condition_id = condition.as_expression().expect("expected expression condition");
            assert_expression_path!(parser, parser.tree.get(condition_id), "y");
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
            // else z
            assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
        });
    });
}

#[test]
fn test_parse_if_else_with_typed_parenthesized_arrow_statement() {
    let test = TestParser::new(
        r###"
if (payments) res.status(200).json({ payments });
else
  (error: Error) =>
    res.status(404).json({
      message: "No Payments were found",
      error,
    });
"###,
    );
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { else_expression, .. } => {
        let else_expression_id = else_expression.expect("expected else expression");
        assert_node!(parser.tree, else_expression_id, Expression::Block(block_id) => {
            let block = parser.tree.get(*block_id);
            let expressions = block_expression_ids(block);
            assert_eq!(expressions.len(), 1);

            let else_item = expressions[0];
            match parser.tree.get(else_item) {
                Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                        assert_eq!(signature.form, FunctionForm::Lambda);
                        assert_eq!(signature.parameters.len(), 1);
                    });
                }
                _ => panic!("expected lambda declaration in else branch"),
            }
        });
    });
}

#[test]
fn test_parse_if_let_condition() {
    let test = TestParser::new("if (let (x, _) = value) { x } else { 0 }");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
        let (kind, _, declarator_id) = condition
            .as_binding()
            .expect("expected binding condition");
        assert_eq!(kind, LetKind::Let);
        assert_node!(parser.tree, declarator_id, Declarator { pattern, ty, value } => {
            assert!(ty.is_none());
            let value_id = value.expect("expected if let value");
            assert_expression_path!(parser, parser.tree.get(value_id), "value");
            assert_node!(parser.tree, *pattern, Pattern::Tuple { fields } => {
                assert_eq!(fields.len(), 2);
            });
        });
        assert_node!(parser.tree, *then_expression, Expression::Block(_));
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(_));
    });
}

#[test]
fn test_parse_if_const_condition() {
    let test = TestParser::new("if (const value = maybe) { value }");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        let (kind, mutability, declarator_id) = condition
            .as_binding()
            .expect("expected binding condition");
        assert_eq!(kind, LetKind::Const);
        assert_eq!(mutability, Mutability::Immutable);
        assert_node!(parser.tree, declarator_id, Declarator { pattern, value, .. } => {
            assert_expression_path!(parser, parser.tree.get(value.expect("expected value")), "maybe");
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "value");
            });
        });
        assert_node!(parser.tree, *then_expression, Expression::Block(_));
    });
}

#[test]
fn test_parse_if_let_tagged_object_pattern() {
    let test = TestParser::new("if (let Point { x, y } = value) { x }");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        let (kind, _, declarator_id) = condition
            .as_binding()
            .expect("expected binding condition");
        assert_eq!(kind, LetKind::Let);
        assert_node!(parser.tree, declarator_id, Declarator { pattern, ty, value } => {
            assert!(ty.is_none());
            let value_id = value.expect("expected if let value");
            assert_expression_path!(parser, parser.tree.get(value_id), "value");
            assert_node!(parser.tree, *pattern, Pattern::NominalObject { ty, fields } => {
                assert_expression_path!(parser, parser.tree.get(*ty), "Point");
                assert_eq!(fields.len(), 2);
                // x
                assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                    assert_name!(parser, *name, "x");
                });
                // y
                assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                    assert_name!(parser, *name, "y");
                });
            });
        });
        assert_node!(parser.tree, *then_expression, Expression::Block(_));
    });
}

/// Logical conditions without bindings should stay regular expressions.
#[test]
fn test_parse_if_logical_condition_as_expression() {
    let test = TestParser::new("if (ready && enabled) { run() }");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        let condition = condition.as_expression().expect("expected expression condition");
        assert_node!(parser.tree, condition, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::And);
            assert_expression_path!(parser, parser.tree.get(*left), "ready");
            assert_expression_path!(parser, parser.tree.get(*right), "enabled");
        });

        assert_node!(parser.tree, *then_expression, Expression::Block(_));
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_if_condition_chain() {
    let test = TestParser::new("if (ready && let (count, label) = pair && count > 0) { label }");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        assert_eq!(condition.operands.len(), 3);

        assert_node!(&condition.operands[0], ConditionOperand::Expression { condition } => {
            assert_expression_path!(parser, parser.tree.get(*condition), "ready");
        });

        assert_node!(&condition.operands[1], ConditionOperand::Binding { kind, declarator, .. } => {
            assert_eq!(*kind, LetKind::Let);
            assert_node!(parser.tree, *declarator, Declarator { pattern, value: Some(value), .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "pair");
                assert_node!(parser.tree, *pattern, Pattern::Tuple { fields } => {
                    assert_eq!(fields.len(), 2);
                });
            });
        });

        assert_node!(&condition.operands[2], ConditionOperand::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                assert_expression_path!(parser, parser.tree.get(*left), "count");
                assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(0)));
            });
        });

        assert_node!(parser.tree, *then_expression, Expression::Block(_));
    });
}

/// Binding condition chains should allow expression operands before bindings.
#[test]
fn test_parse_if_condition_chain_after_comparison() {
    let test = TestParser::new("if (value < limit && let item = maybe) { item }");
    let mut parser = test.prepare();

    let if_id = parser.parse_if().unwrap();
    assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
        assert_eq!(condition.operands.len(), 2);

        assert_node!(&condition.operands[0], ConditionOperand::Expression { condition } => {
            assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                assert_expression_path!(parser, parser.tree.get(*left), "value");
                assert_expression_path!(parser, parser.tree.get(*right), "limit");
            });
        });

        assert_node!(&condition.operands[1], ConditionOperand::Binding { kind, declarator, .. } => {
            assert_eq!(*kind, LetKind::Let);
            assert_node!(parser.tree, *declarator, Declarator { pattern, value: Some(value), .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "maybe");
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "item");
                });
            });
        });

        assert_node!(parser.tree, *then_expression, Expression::Block(_));
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_if_head_trailing_comment_on_condition_owner() {
    let test = TestParser::new("if (ready) // if-head\n    run()");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::If { condition, .. } => {
        let condition_id = condition.as_expression().expect("expected expression condition");

        let annotations = parser.tree.get_decorators(condition_id.id);
        assert!(annotations.is_empty());
    });

    assert_node!(parser.tree, expression_id, Expression::If { then_expression, .. } => {
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let then_annotations = parser.tree.get_decorators(expressions[0].id);
                assert!(then_annotations.is_empty());
            });
        });
    });

    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "if-head");
}

#[test]
fn test_parse_if_else_boundary_comment_on_else_owner() {
    let test = TestParser::new("if (ready) {\n  run()\n}\n// else-boundary\nelse {\n  stop()\n}\n");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::If { else_expression, .. } => {
        let else_expression_id = else_expression.expect("expected else expression");
        let annotations = parser.tree.get_decorators(else_expression_id.id);
        assert!(annotations.is_empty());
    });

    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "else-boundary");
}

#[test]
fn test_parse_if_else_after_then_semicolon_with_leading_boundary_comment() {
    let input = "if (foo) a = b;\n/* foo */ else foo.split;";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::If { else_expression, .. } => {
        assert!(else_expression.is_some());
    });
}

#[test]
fn test_parse_if_else_after_then_semicolon_with_trailing_boundary_comment() {
    let input = "if (foo) a = b;\nelse /* foo */ foo.split;";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::If { else_expression, .. } => {
        assert!(else_expression.is_some());
    });
}
