use tspp_dir::{
    Argument, Asynchrony, BinaryOperator, BindingKeyword, Block, ConditionOperand, Declarator,
    Expression, ForEachBinding, GenericArgument, Keyword, LetKind, Literal, Name, Pattern,
    PatternField, TokenType, TypeExpression, TypeLiteral, TypeMember, UnaryOperator, WhileForm,
};

use crate::{
    TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    block_expression_ids,
};

#[test]
fn test_parse_loop() {
    let test = TestParser::new(
        r###"
loop {
    x
}
"###,
    );
    let mut parser = test.prepare();

    let loop_id = parser.parse_loop().unwrap();
    assert_node!(parser.tree, loop_id, Expression::Loop { body, .. } => {
        let _block = parser.tree.get(*body);
    });
    let main_span = parser.tree.get_main_span(loop_id).unwrap();
    assert_eq!(parser.span_str(main_span), "loop");
}

#[test]
fn test_parse_for_of_loop() {
    let test = TestParser::new(
        r###"
for (const item of items) {
    x
}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, keyword }, iterator, .. } => {
        assert_eq!(*keyword, Some(BindingKeyword::Const));
        // item
        assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "item");
        });
        // items
        assert_expression_path!(parser, parser.tree.get(*iterator), "items");
    });
    let main_span = parser.tree.get_main_span(for_id).unwrap();
    assert_eq!(parser.span_str(main_span), "for");
}

#[test]
fn test_reject_for_in_loop() {
    let test = TestParser::new("for (const key in target) {}");
    let mut parser = test.prepare();

    assert!(parser.parse_for().is_err());
}

#[test]
fn test_parse_for_loop_with_missing_close_parenthesis() {
    let test = TestParser::new("for (item of items { body }");
    let mut parser = test.prepare();
    let for_id = parser.parse_for().unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body, .. } => {
        assert_eq!(*keyword, None);
        assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "item");
        });
        assert_expression_path!(parser, parser.tree.get(*iterator), "items");
        let _body = parser.tree.get(*body);
    });
}

#[test]
fn test_parse_for_loop_with_line_comment_after_keyword() {
    let test = TestParser::new("for // comment\n(;;);");
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body, .. } => {
        assert!(initialization.is_none());
        assert!(condition.is_none());
        assert!(increment.is_none());
        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert!(expressions.is_empty());
        });
    });
}

#[test]
fn test_parse_for_loop_with_block_comment_after_keyword() {
    let test = TestParser::new("for /* comment */(;;);");
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, body, .. } => {
        assert!(initialization.is_none());
        assert!(condition.is_none());
        assert!(increment.is_none());
        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert!(expressions.is_empty());
        });
    });
}

#[test]
fn test_parse_for_loop_with_async_in_parentheses() {
    let test = TestParser::new(
        r###"
for await (const item of items) {
    x
}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, binding: ForEachBinding::Pattern { pattern, keyword }, iterator, .. } => {
        assert_eq!(*asynchrony, Asynchrony::Async);
        assert_eq!(*keyword, Some(BindingKeyword::Const));
        // item
        assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "item");
        });
        // items
        assert_expression_path!(parser, parser.tree.get(*iterator), "items");
    });
}

/// Parse for of headers with multiline destructured bindings.
#[test]
fn test_parse_for_of_multiline_destructured_binding() {
    let test = TestParser::new(
        r###"
for (
  const {
    relation,
  }
  of selectedRelations
) {}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    // for (const { ... } of selectedRelations) {}
    assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, binding, iterator, body, .. } => {
        assert_eq!(*asynchrony, Asynchrony::Sync);

        // const { relation }
        assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
            assert_eq!(*keyword, Some(BindingKeyword::Const));
            assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                assert_eq!(fields.len(), 1);
                assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None, .. } => {
                    assert_name!(parser, *name, "relation");
                });
            });
        });

        // selectedRelations
        assert_expression_path!(parser, parser.tree.get(*iterator), "selectedRelations");
        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert!(expressions.is_empty());
        });
    });
}

#[test]
fn test_parse_for_of_await_generic_call_with_object_type_argument() {
    let test = TestParser::new(
        r###"
for (const { item } of await fetchList<{ item: string }>(values)) {}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, binding, iterator, body, .. } => {
        assert_eq!(*asynchrony, Asynchrony::Sync);

        // const { item }
        assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
            assert_eq!(*keyword, Some(BindingKeyword::Const));
            assert_node!(parser.tree, *pattern, Pattern::Object { fields } => {
                assert_eq!(fields.len(), 1);
                assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None, .. } => {
                    assert_name!(parser, *name, "item");
                });
            });
        });

        // await fetchList<{ item: string }>(values)
        assert_node!(parser.tree, *iterator, Expression::Await { expression } => {
            assert_node!(parser.tree, *expression, Expression::Call { left, generic_arguments, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "fetchList");
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "values");
                });

                assert_eq!(generic_arguments.len(), 1);

                assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                        assert_node!(parser.tree, *value, TypeExpression::Object { members: properties } => {
                            assert_eq!(properties.len(), 1);
                            assert_node!(parser.tree, properties[0], TypeMember::Field { name: Name::Identifier(name), declared_type: value, .. } => {
                                assert_string!(parser, *name, "item");
                                assert_node!(parser.tree, value.expect("expected declared type"), TypeExpression::Keyword { value } => {
                                    assert_eq!(*value, TypeLiteral::String);
                                });
                            });
                        });
                });
            });
        });

        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert!(expressions.is_empty());
        });
    });
}

#[test]
fn test_parse_for_each_binding_const_object_stops_before_of() {
    let test = TestParser::new(
        r###"
for (const { item } of fetchList<{ item: string }>(values)) {}
"###,
    );
    let mut parser = test.prepare();

    parser.eat_keyword(Keyword::For).unwrap();
    parser.eat_token(TokenType::OpenParenthesis).unwrap();

    let binding = parser.parse_for_each_binding().unwrap();
    assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
        assert_eq!(keyword, Some(BindingKeyword::Const));
        assert_node!(parser.tree, pattern, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
        });
    });

    assert_eq!(parser.peek_keyword(), Some(Keyword::Of));
}

#[test]
fn test_parse_for_of_tagged_object_binding() {
    let test = TestParser::new(
        r###"
for (const Shape.Line { start: Point { x, y }, end } of lines) {}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();

    assert!(parser.errors.is_empty());

    assert_node!(parser.tree, for_id, Expression::ForEach { binding, iterator, body, .. } => {

        assert_node!(binding, ForEachBinding::Pattern { pattern, keyword } => {
            assert_eq!(*keyword, Some(BindingKeyword::Const));
            assert_node!(parser.tree, *pattern, Pattern::NominalObject { ty, fields } => {
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                    assert_path!(parser, *path, "Shape.Line");
                });
                assert_eq!(fields.len(), 2);

                assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), .. } => {
                    assert_name!(parser, *name, "start");
                    assert_node!(parser.tree, *pattern, Pattern::NominalObject { ty, fields } => {
                        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                            assert_path!(parser, *path, "Point");
                        });
                        assert_eq!(fields.len(), 2);
                    });
                });

                assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                    assert_name!(parser, *name, "end");
                });
            });
        });

        assert_expression_path!(parser, parser.tree.get(*iterator), "lines");
        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert!(expressions.is_empty());
        });
    });
}

#[test]
fn test_parse_for_loop_with_inline_if_body() {
    let test = TestParser::new(
        r###"
for (let r of t)
    if (r !== "default" && !Object.prototype.hasOwnProperty.call(e, r)) i(e, t, r)
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, keyword }, iterator, body, .. } => {
        assert_eq!(*keyword, Some(BindingKeyword::Let));
        // r
        assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "r");
        });
        // t
        assert_expression_path!(parser, parser.tree.get(*iterator), "t");
        // body
        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert_eq!(expressions.len(), 1);
            assert_node!(parser.tree, expressions[0], Expression::If { .. } => {});
        });
    });
}

#[test]
fn test_parse_for_loop_with_label() {
    let test = TestParser::new(
        r###"
for (const item of items) outer: {
    x
}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Pattern { pattern, keyword }, iterator, .. } => {
        assert_eq!(*keyword, Some(BindingKeyword::Const));
        // item
        assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "item");
        });
        // items
        assert_expression_path!(parser, parser.tree.get(*iterator), "items");
    });
}

#[test]
fn test_parse_for_loop_with_using_binding() {
    let test = TestParser::new(
        r###"
for (using item of items) {
    x
}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::ForEach { binding: ForEachBinding::Using { asynchrony, pattern }, iterator, .. } => {
        assert_eq!(*asynchrony, Asynchrony::Sync);
        assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "item");
        });
        assert_expression_path!(parser, parser.tree.get(*iterator), "items");
    });
}

#[test]
fn test_parse_for_loop_with_await_using_of_binding() {
    let test = TestParser::new("for await (await using of of items);");
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::ForEach { asynchrony, binding, iterator, .. } => {
        assert_eq!(*asynchrony, Asynchrony::Async);

        assert_node!(binding, ForEachBinding::Using { asynchrony, pattern } => {
            assert_eq!(*asynchrony, Asynchrony::Async);
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "of");
            });
        });

        assert_expression_path!(parser, parser.tree.get(*iterator), "items");
    });
}

#[test]
fn test_parse_for_loop_condition_empty() {
    let test = TestParser::new(
        r###"
for (;;) {}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, .. } => {
        assert!(initialization.is_none());
        assert!(condition.is_none());
        assert!(increment.is_none());
    });
    let main_span = parser.tree.get_main_span(for_id).unwrap();
    assert_eq!(parser.span_str(main_span), "for");
}

#[test]
fn test_parse_for_loop_condition_with_initialization() {
    let test = TestParser::new(
        r###"
for (let x = 0; x < 10; x++) {
    x
}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::For { initialization, condition, increment, .. } => {
        // let x = 0
        assert_node!(parser.tree, initialization.unwrap(), Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "x");
                });
                assert_node!(parser.tree, value.unwrap(), Expression::Literal(Literal::Integer(0)));
            });
        });
        // x < 10
        assert_node!(parser.tree, condition.unwrap(), Expression::Binary { left, operator, right } => {
            // x
            assert_expression_path!(parser, parser.tree.get(*left), "x");
            // <
            assert_eq!(*operator, BinaryOperator::LessThan);
            // 10
            assert_node!(parser.tree, *right, Expression::Literal(Literal::Integer(10)));
        });
        // x++
        assert_node!(parser.tree, increment.unwrap(), Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::PostIncrement);
            assert_node!(parser.tree, *right, Expression::Identifier { name } => {
                assert_string!(parser, *name, "x");
            });
        });
    });
}
#[test]
fn test_parse_for_of_with_type_identifier_binding() {
    let test = TestParser::new(
        r###"
for (type of values) {}
"###,
    );
    let mut parser = test.prepare();

    let for_id = parser.parse_for().unwrap();
    assert_node!(parser.tree, for_id, Expression::ForEach { binding, iterator, .. } => {
        // binding
        assert!(matches!(binding, ForEachBinding::Pattern { .. }));
        let ForEachBinding::Pattern { pattern, keyword } = binding else {
            panic!("expected for each pattern binding");
        };
        assert!(keyword.is_none());
        assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "type");
        });

        // iterator
        assert_expression_path!(parser, parser.tree.get(*iterator), "values");
    });
}

#[test]
fn test_parse_while_loop() {
    let test = TestParser::new(
        r###"
while (x) {}
"###,
    );
    let mut parser = test.prepare();

    let while_id = parser.parse_while().unwrap();
    assert_node!(parser.tree, while_id, Expression::While { condition, .. } => {
        let condition = condition.as_expression().unwrap();
        assert_expression_path!(parser, parser.tree.get(condition), "x");
    });
    let main_span = parser.tree.get_main_span(while_id).unwrap();
    assert_eq!(parser.span_str(main_span), "while");
}

/// Parse one while binding condition.
#[test]
fn test_parse_while_binding_condition() {
    let test = TestParser::new("while (let value! = queue.tryPop()) { value }");
    let mut parser = test.prepare();

    let while_id = parser.parse_while().unwrap();
    assert_node!(parser.tree, while_id, Expression::While { condition, .. } => {
        let (kind, _, declarator) = condition.as_binding().unwrap();
        assert_eq!(kind, LetKind::Let);
        assert_node!(parser.tree, declarator, Declarator { pattern, value, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Must(pattern) => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "value");
                });
            });

            let value = value.unwrap();
            assert_node!(parser.tree, value, Expression::Call { .. });
        });
    });
    test.assert_no_errors(&parser);
}

/// Parse expression and binding operands in one while condition.
#[test]
fn test_parse_while_binding_condition_chain() {
    let test =
        TestParser::new("while (ready && let value! = queue.tryPop() && value > 0) { value }");
    let mut parser = test.prepare();

    let while_id = parser.parse_while().unwrap();
    assert_node!(parser.tree, while_id, Expression::While { condition, .. } => {
        assert_eq!(condition.operands.len(), 3);
        assert!(matches!(condition.operands[0], ConditionOperand::Expression { .. }));
        assert!(matches!(condition.operands[1], ConditionOperand::Binding { .. }));
        assert!(matches!(condition.operands[2], ConditionOperand::Expression { .. }));
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_while_loop_nested() {
    let test = TestParser::new(
        r###"
while (x > y) {
    while (a < b) {
        inner_work()
    }
    outer_work()
}
"###,
    );
    let mut parser = test.prepare();

    let while_id = parser.parse_while().unwrap();

    // while (x > y)
    assert_node!(parser.tree, while_id, Expression::While { condition, body, .. } => {
        let condition = condition.as_expression().unwrap();

        // x > y
        assert_node!(parser.tree, condition, Expression::Binary { left, operator, right } => {
            // x
            assert_expression_path!(parser, parser.tree.get(*left), "x");
            // >
            assert_eq!(*operator, BinaryOperator::GreaterThan);
            // y
            assert_expression_path!(parser, parser.tree.get(*right), "y");
        });

        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert_eq!(expressions.len(), 2);

            // while a < b
            assert_node!(parser.tree, expressions[0], Expression::While { condition: nested_condition, .. } => {
                let nested_condition = nested_condition.as_expression().unwrap();

                // a < b
                assert_node!(parser.tree, nested_condition, Expression::Binary { left, operator, right } => {
                    // a
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    // <
                    assert_eq!(*operator, BinaryOperator::LessThan);
                    // b
                    assert_expression_path!(parser, parser.tree.get(*right), "b");
                });
            });
        });
    });
}

#[test]
fn test_parse_while_parenthesized_condition_keeps_inner_span() {
    let test = TestParser::new("while (x > y) {}");
    let mut parser = test.prepare();

    let while_id = parser.parse_while().unwrap();
    assert_node!(parser.tree, while_id, Expression::While { condition, .. } => {
        let condition = condition.as_expression().unwrap();
        assert_node!(parser.tree, condition, Expression::Binary { .. });

        let condition_span = parser.tree.get_span(condition);
        let condition_text = &parser.file.text()
            [condition_span.start as usize..condition_span.end as usize];
        assert_eq!(condition_text, "x > y");
    });
}

#[test]
fn test_parse_do_while_loop() {
    let test = TestParser::new(
        r###"
do { x } while (true)
"###,
    );

    let mut parser = test.prepare();

    // do { x } while true
    let do_while_id = parser.parse_while().unwrap();
    assert_node!(parser.tree, do_while_id, Expression::While { form, condition, .. } => {
        assert_eq!(*form, WhileForm::DoWhile);
        let condition = condition.as_expression().unwrap();
        assert_node!(parser.tree, condition, Expression::Literal(Literal::Boolean(true)));
    });
    let main_span = parser.tree.get_main_span(do_while_id).unwrap();
    assert_eq!(parser.span_str(main_span), "do");
}

/// Parse a do-while expression whose block contains statement separators.
#[test]
fn test_parse_do_while_block_with_semicolons() {
    let test = TestParser::new("do { a++; b--; } while (a < 1)");
    let mut parser = test.prepare();

    let roots = parser.parse_in_place();
    assert_eq!(roots.len(), 1);
    assert_node!(parser.tree, roots[0], Expression::While { form, condition, body, .. } => {
        assert_eq!(*form, WhileForm::DoWhile);
        let condition = condition.as_expression().unwrap();
        assert_node!(parser.tree, condition, Expression::Binary { .. });
        assert_node!(parser.tree, *body, Block { .. });
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_do_while_parenthesized_condition_keeps_inner_span() {
    let test = TestParser::new("do x; while (value)");
    let mut parser = test.prepare();

    let while_id = parser.parse_while().unwrap();
    assert_node!(parser.tree, while_id, Expression::While { form, condition, .. } => {
        assert_eq!(*form, WhileForm::DoWhile);
        let condition = condition.as_expression().unwrap();
        assert_expression_path!(parser, parser.tree.get(condition), "value");

        let condition_span = parser.tree.get_span(condition);
        let condition_text = &parser.file.text()
            [condition_span.start as usize..condition_span.end as usize];
        assert_eq!(condition_text, "value");
    });
}

/// Parse a single statement loop body without braces.
#[test]
fn test_parse_do_while_single_statement() {
    let test = TestParser::new("do x; while (true)");

    let mut parser = test.prepare();

    let do_while_id = parser.parse_while().unwrap();
    assert_node!(parser.tree, do_while_id, Expression::While { form, condition, body, .. } => {
        assert_eq!(*form, WhileForm::DoWhile);
        let condition = condition.as_expression().unwrap();
        assert_node!(parser.tree, condition, Expression::Literal(Literal::Boolean(true)));
        // body should be a block with single expression
        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert_eq!(expressions.len(), 1);
        });
    });
}

#[test]
fn test_parse_do_while_continue_statement() {
    let test = TestParser::new("do continue; while (true)");

    let mut parser = test.prepare();

    let do_while_id = parser.parse_while().unwrap();
    assert_node!(parser.tree, do_while_id, Expression::While { form, condition, body, .. } => {
        assert_eq!(*form, WhileForm::DoWhile);
        let condition = condition.as_expression().unwrap();
        assert_node!(parser.tree, condition, Expression::Literal(Literal::Boolean(true)));
        // body should be a block with continue statement
        assert_node!(parser.tree, *body, Block { .. } => {
            let expressions = block_expression_ids(parser.tree.get(*body));
            assert_eq!(expressions.len(), 1);
            assert_node!(parser.tree, expressions[0], Expression::Continue { label: None });
        });
    });
}
