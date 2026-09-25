use crate::{
    CommentRetention, ExpressionPosition, ExpressionStop, TestParser, assert_comment,
    assert_expression_path, assert_node, assert_string, assert_value_expression_path,
    block_expression_ids,
};
use tspp_dir::{
    Block, BlockContext, BlockForm, CommentKind, Declaration, Expression, FunctionDeclaration,
    FunctionForm, IfForm, LetKind, Literal, MatchArm, Name, NodeType, Property, TokenType,
    YieldCardinality,
};

#[test]
fn test_parse_empty_block() {
    let test = TestParser::new("{}");
    let mut parser = test.prepare();
    let block_id = parser.parse_block(BlockContext::Expression).unwrap();
    let block = parser.tree.get(block_id);
    assert!(block.is_empty());
}

#[test]
fn test_parse_arrow_expression_statement() {
    let test = TestParser::new("value => value;");
    let mut parser = test.prepare();
    let expressions = parser
        .parse_block_body(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.parameters.len(), 1);
        });
    });
}

#[test]
fn test_parse_generic_arrow_expression_statement() {
    let test = TestParser::new("<T,>() => 1;");
    let mut parser = test.prepare();
    let expressions = parser
        .parse_block_body(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
            assert_eq!(signature.form, FunctionForm::Lambda);
            assert_eq!(signature.generic_parameters.len(), 1);
            assert!(signature.parameters.is_empty());
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_ternary_arrow_block_expression_statement() {
    let test = TestParser::new("ready ? (value) : item => {};");
    let mut parser = test.prepare();
    let expressions = parser
        .parse_block_body(BlockForm::Implicit, BlockContext::Statement)
        .unwrap();

    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::If { form, else_expression, .. } => {
        assert_eq!(*form, IfForm::Ternary);
        assert_node!(parser.tree, else_expression.unwrap(), Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, body, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
                assert_node!(parser.tree, body.expect("expected lambda block body"), Expression::Block(_));
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_block_with_missing_close_brace() {
    let test = TestParser::new("{ value");
    let mut parser = test.prepare();
    let block_id = parser.parse_block(BlockContext::Expression).unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, block_id, Block { leading_expressions, tail_expression, .. } => {
        assert!(leading_expressions.is_empty());
        let tail_expression = tail_expression.expect("expected tail expression");
        assert_expression_path!(parser, parser.tree.get(tail_expression), "value");
    });
}

#[test]
fn test_parse_root_unmatched_close_brace_recovery() {
    let test = TestParser::new("}\nnextValue");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Error);
    assert_expression_path!(parser, parser.tree.get(expressions[1]), "nextValue");
}

#[test]
fn test_parse_root_unmatched_close_parenthesis_recovery() {
    let test = TestParser::new(")\nnextValue");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Error);
    assert_expression_path!(parser, parser.tree.get(expressions[1]), "nextValue");
}

#[test]
fn test_break_no_label_no_value() {
    let test = TestParser::new("break");
    let mut parser = test.prepare();
    let break_id = parser.parse_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label: None, value } => {
        assert!(value.is_none());
    });
}

#[test]
fn test_break_with_label() {
    let test = TestParser::new("break label");
    let mut parser = test.prepare();
    let break_id = parser.parse_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
        assert_string!(parser, label.unwrap(), "label");
        assert!(value.is_none());
    });

    let main_span = parser
        .tree
        .get_main_span(break_id)
        .expect("expected break label span");
    assert_eq!(parser.span_str(main_span), "label");
}

#[test]
fn test_break_with_label_and_value() {
    let test = TestParser::new("break label: 17");
    let mut parser = test.prepare();
    let break_id = parser.parse_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
        assert_string!(parser, label.unwrap(), "label");
        assert!(value.is_some());
        assert_node!(parser.tree, value.unwrap(), Expression::Literal(Literal::Integer(17)));
    });
}

#[test]
fn test_break_with_parenthesized_identifier_value() {
    let test = TestParser::new("break (value)");
    let mut parser = test.prepare();
    let break_id = parser.parse_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label: None, value } => {
        assert!(value.is_some());
        crate::assert_parenthesized!(parser.tree, value.unwrap(), expression => {
            assert_expression_path!(parser, parser.tree.get(*expression), "value");
        });
    });
}

#[test]
fn test_continue_no_label() {
    let test = TestParser::new("continue");
    let mut parser = test.prepare();
    let continue_id = parser.parse_continue().unwrap();
    assert_node!(parser.tree, continue_id, Expression::Continue { label: None } => {
    });
}

#[test]
fn test_continue_with_label() {
    let test = TestParser::new("continue label");
    let mut parser = test.prepare();
    let continue_id = parser.parse_continue().unwrap();
    assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
        assert_string!(parser, label.unwrap(), "label");
    });

    let main_span = parser
        .tree
        .get_main_span(continue_id)
        .expect("expected continue label span");
    assert_eq!(parser.span_str(main_span), "label");
}

#[test]
fn test_break_label_before_newline() {
    let test = TestParser::new("break foo\n");
    let mut parser = test.prepare();
    let break_id = parser.parse_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
        assert_string!(parser, label.unwrap(), "foo");
        assert!(value.is_none());
    });
}

#[test]
fn test_continue_label_before_newline() {
    let test = TestParser::new("continue foo\n");
    let mut parser = test.prepare();
    let continue_id = parser.parse_continue().unwrap();
    assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
        assert_string!(parser, label.unwrap(), "foo");
    });
}

#[test]
fn test_await_expression() {
    let test = TestParser::new("await someFunction()");
    let mut parser = test.prepare();
    let await_id = parser.parse_await().unwrap();
    // await someFunction()
    assert_node!(parser.tree, await_id, Expression::Await { expression } => {
        // someFunction()
        assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_await_maybe_expression() {
    let test = TestParser::new("await? someFunction()");
    let mut parser = test.prepare();
    let await_id = parser.parse_await().unwrap();
    // await? someFunction()
    assert_node!(parser.tree, await_id, Expression::AwaitMaybe { expression } => {
        // someFunction()
        assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_await_must_expression() {
    let test = TestParser::new("await! someFunction()");
    let mut parser = test.prepare();
    let await_id = parser.parse_await().unwrap();

    // await! someFunction()
    assert_node!(parser.tree, await_id, Expression::AwaitMust { expression } => {
        // someFunction()
        assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_parse_const_expression() {
    let test = TestParser::new("const factorial(10)");
    let mut parser = test.prepare();
    let const_id = parser.parse_const_evaluation().unwrap();
    // const factorial(10)
    assert_node!(parser.tree, const_id, Expression::Const { body } => {
        // factorial(10)
        assert_node!(parser.tree, *body, Expression::Call { position: _, left, generic_arguments: _, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "factorial");
            assert_eq!(arguments.len(), 1);
        });
    });
}

#[test]
fn test_parse_const_expression_simple() {
    // const 1 + 2
    let test = TestParser::new("const 1 + 2");
    let mut parser = test.prepare();
    let const_id = parser.parse_const_evaluation().unwrap();
    // const 1 + 2
    assert_node!(parser.tree, const_id, Expression::Const { body } => {
        // 1 + 2
        assert_node!(parser.tree, *body, Expression::Binary { .. } => {
            // binary addition
        });
    });
}

#[test]
fn test_parse_const_block_expression() {
    let test = TestParser::new("const { let x = 1; x + 2 }");
    let mut parser = test.prepare();
    let const_id = parser.parse_const_evaluation().unwrap();
    assert_node!(parser.tree, const_id, Expression::Const { body } => {
        assert_node!(parser.tree, *body, Expression::Block(_));
    });
}

#[test]
fn test_yield_expression() {
    let test = TestParser::new("yield someFunction()");
    let mut parser = test.prepare();
    let yield_id = parser.parse_yield().unwrap();
    // yield someFunction()
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        // someFunction()
        assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, generic_arguments: _, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_yield_expression_no_value() {
    let test = TestParser::new("yield");
    let mut parser = test.prepare();
    let yield_id = parser.parse_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none());
    });
}

#[test]
fn test_yield_expression_no_value_before_close_parenthesis() {
    // source: yield)
    let test = TestParser::new("yield)");
    let mut parser = test.prepare();
    let yield_id = parser.parse_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none());
    });
    assert!(parser.peek_is(TokenType::CloseParenthesis));
}

#[test]
fn test_yield_expression_no_value_before_close_bracket() {
    // source: yield]
    let test = TestParser::new("yield]");
    let mut parser = test.prepare();
    let yield_id = parser.parse_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none());
    });
    assert!(parser.peek_is(TokenType::CloseBracket));
}

#[test]
fn test_yield_expression_generator() {
    let test = TestParser::new("yield* someFunction()");
    let mut parser = test.prepare();
    let yield_id = parser.parse_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Generator);
        // someFunction()
        assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, generic_arguments: _, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_yield_expression_generator_with_space() {
    let test = TestParser::new("yield *a");
    let mut parser = test.prepare();
    let yield_id = parser.parse_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Generator);
        assert!(value.is_some());
    });
}

#[test]
fn test_recover_yield_star_without_operand() {
    // source: yield*
    let test = TestParser::new("yield*");
    let mut parser = test.prepare();
    let yield_id = parser.parse_yield().unwrap();

    // diagnostics
    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // yield*
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Generator);
        assert_node!(parser.tree, value.expect("expected missing generator operand"), Expression::Missing);
    });
}

/// `yield\n*a` should NOT be parsed as `yield* a` due to ASI restricted production.
#[test]
fn test_yield_asi_with_newline() {
    let test = TestParser::new("yield\n*a");
    let mut parser = test.prepare();
    let yield_id = parser.parse_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none()); // ASI applied, no value
    });
}

#[test]
fn test_return_no_value() {
    let test = TestParser::new("return");
    let mut parser = test.prepare();
    let return_id = parser.parse_return().unwrap();
    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_none());
    });
}

#[test]
fn test_return_with_value() {
    let test = TestParser::new("return 42");
    let mut parser = test.prepare();
    let return_id = parser.parse_return().unwrap();
    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_some());
        assert_node!(parser.tree, value.unwrap(), Expression::Literal(Literal::Integer(42)));
    });
}

#[test]
fn test_parse_block_const_then_return_cast() {
    let test = TestParser::new(
        "{\n  const result = CreateRecord(IntegerKey, value)\n  return result as never\n}",
    );
    let mut parser = test.prepare();
    let block_id = parser.parse_block(BlockContext::Expression).unwrap();
    let block = parser.tree.get(block_id);
    assert_eq!(block.leading_expressions.len(), 2);
    assert!(block.tail_expression.is_none());

    let let_expression_id = block.leading_expressions[0];
    assert_node!(parser.tree, let_expression_id, Expression::Let { kind, declarators, .. } => {
        assert_eq!(*kind, LetKind::Const);
        assert_eq!(declarators.len(), 1);
    });

    let return_expression_id = block.leading_expressions[1];
    assert_node!(parser.tree, return_expression_id, Expression::Return { value } => {
        let value = value.expect("expected return value");
        assert_node!(parser.tree, value, Expression::As { .. } => {
        });
    });
}

#[test]
fn test_parse_return_ternary_with_newline_before_question() {
    let test = TestParser::new(
        "return Result.IsExtendsTrueLike(check)\n  ? TryInferResults(tail, right, [...result, head])\n  : undefined",
    );
    let mut parser = test.prepare();
    let return_id = parser.parse_return().unwrap();

    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        let value = value.expect("expected return value");
        assert_node!(parser.tree, value, Expression::If { form, .. } => {
            assert_eq!(*form, IfForm::Ternary);
        });
    });
}

#[test]
fn test_parse_block_statement_before_close_brace_without_semicolon() {
    let test = TestParser::new("{ process.exit(1)}");
    let mut parser = test.prepare();
    let block_id = parser.parse_block(BlockContext::Expression).unwrap();
    let block = parser.tree.get(block_id);

    // final block values remain value-producing tails
    assert!(block.leading_expressions.is_empty());
    let tail_expression = block.tail_expression.expect("expected block tail");
    assert_node!(parser.tree, tail_expression, Expression::Call { .. });
}

/// Parse semicolon led parenthesized calls as canonical call expressions.
#[test]
fn test_parse_statement_leading_semicolon_parenthesized_arrow_call() {
    let test = TestParser::new("{\n;(()=>{})()\n}");
    let mut parser = test.prepare_with_comment_retention(CommentRetention::All);
    let block_id = parser.parse_block(BlockContext::Expression).unwrap();
    let block = parser.tree.get(block_id);
    let expressions = block_expression_ids(block);

    assert_eq!(expressions.len(), 1);

    assert_node!(parser.tree, expressions[0], Expression::Call { left, arguments, .. } => {
        assert!(arguments.is_empty());
        assert_node!(parser.tree, *left, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function(FunctionDeclaration { signature, .. }) => {
                assert_eq!(signature.form, FunctionForm::Lambda);
            });
        });
    });
}

/// Parse if-body block tails as value expressions.
#[test]
fn test_parse_if_block_keeps_tail_expression_value() {
    let test = TestParser::new("if (x) { foo() }");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // if (x) { foo() }
    assert_eq!(expressions.len(), 1);
    let if_expression_id = expressions[0];
    assert_node!(parser.tree, if_expression_id, Expression::If { then_expression, .. } => {
        assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
            let block = parser.tree.get(*block_id);
            assert!(block.leading_expressions.is_empty());
            let tail_expression = block.tail_expression.expect("expected tail expression");
            assert_node!(parser.tree, tail_expression, Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
            });
        });
    });
}

/// Parse function body tails as value expressions.
#[test]
fn test_parse_function_body_keeps_tail_expression_value() {
    let test = TestParser::new("function run() { foo() }");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // function run() { foo() }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = expressions[0];
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert!(block.leading_expressions.is_empty());
                let tail_expression = block.tail_expression.expect("expected tail expression");
                assert_node!(parser.tree, tail_expression, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                });
            });
        });
    });
}

/// Parse object literal function body tails as value expressions.
#[test]
fn test_parse_function_body_keeps_object_literal_tail_expression_value() {
    let input = r#"
function next(value: number): IteratorResult<number> {
    drop(value);
    { done: true, value }
}
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);

    // function next(...) { drop(value); { done: true, value } }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = expressions[0];
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert_node!(parser.tree, block.leading_expressions[0], Expression::Call { left, arguments, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "drop");
                    assert_eq!(arguments.len(), 1);
                });

                let tail_expression = block.tail_expression.expect("expected tail expression");
                assert_node!(parser.tree, tail_expression, Expression::ObjectExpression { properties, .. } => {
                    assert_eq!(properties.len(), 2);
                    assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, is_shorthand } => {
                        assert_string!(parser, *name, "done");
                        assert!(!*is_shorthand);
                        assert_node!(parser.tree, *value, Expression::Literal(Literal::Boolean(true)));
                    });
                    assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, is_shorthand } => {
                        assert_string!(parser, *name, "value");
                        assert!(*is_shorthand);
                        assert_expression_path!(parser, parser.tree.get(*value), "value");
                    });
                });
            });
        });
    });
}

/// Parse object literal match branch tails as value expressions.
#[test]
fn test_parse_match_branch_keeps_object_literal_tail_expression_value() {
    let input = r#"
function apply(result: Result): IteratorResult<number> {
    match (result) {
        Yield { value } => {
            this.value = value;
            { done: false, value }
        }
        Return { value } => {
            { done: true, value }
        }
    }
}
"#;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);

    // function apply(...) { match (...) { ... } }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = expressions[0];
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_node!(parser.tree, *body_id, Expression::Block(function_block_id) => {
                let function_block = parser.tree.get(*function_block_id);
                let tail_expression = function_block.tail_expression.expect("expected match tail");

                assert_node!(parser.tree, tail_expression, Expression::Match { arms, .. } => {
                    assert_eq!(arms.len(), 2);
                    for arm_id in arms {
                        assert_node!(parser.tree, *arm_id, MatchArm::Block { body: case_block_id, .. } => {
                            let case_block = parser.tree.get(*case_block_id);
                            let case_tail = case_block.tail_expression.expect("expected object tail");

                            assert_node!(parser.tree, case_tail, Expression::ObjectExpression { properties, .. } => {
                                assert_eq!(properties.len(), 2);
                                assert_node!(parser.tree, properties[0], Property::Field { name: Name::Identifier(name), value, .. } => {
                                    assert_string!(parser, *name, "done");
                                    assert_node!(parser.tree, *value, Expression::Literal(Literal::Boolean(_)));
                                });
                                assert_node!(parser.tree, properties[1], Property::Field { name: Name::Identifier(name), value, is_shorthand } => {
                                    assert_string!(parser, *name, "value");
                                    assert!(*is_shorthand);
                                    assert_expression_path!(parser, parser.tree.get(*value), "value");
                                });
                            });
                        });
                    }
                });
            });
        });
    });
}

/// Parse multiline function body tails as value expressions.
#[test]
fn test_parse_multiline_function_body_keeps_tail_expression_value() {
    let test = TestParser::new(
        r#"
function run() {
    foo()
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // function run() { foo() }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = expressions[0];
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert!(block.leading_expressions.is_empty());
                let tail_expression = block.tail_expression.expect("expected tail expression");
                assert_node!(parser.tree, tail_expression, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                });
            });
        });
    });
}

/// Parse function body if-else tails as value expressions.
#[test]
fn test_parse_function_body_keeps_if_else_tail_expression_value() {
    let test = TestParser::new(
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { a } else { b }
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // function choose(...) { if (flag) { a } else { b } }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = expressions[0];
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_node!(parser.tree, *body_id, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 0);

                let tail_expression = block.tail_expression.expect("expected tail expression");
                assert_node!(parser.tree, tail_expression, Expression::If { then_expression, else_expression, .. } => {
                    assert_node!(parser.tree, *then_expression, Expression::Block(then_block_id) => {
                        let then_block = parser.tree.get(*then_block_id);
                        assert_eq!(then_block.leading_expressions.len(), 0);
                        assert_expression_path!(
                            parser,
                            parser.tree.get(then_block.tail_expression.expect("expected then tail")),
                            "a"
                        );
                    });

                    let else_expression = else_expression.expect("expected else expression");
                    assert_node!(parser.tree, else_expression, Expression::Block(else_block_id) => {
                        let else_block = parser.tree.get(*else_block_id);
                        assert_eq!(else_block.leading_expressions.len(), 0);
                        assert_expression_path!(
                            parser,
                            parser.tree.get(else_block.tail_expression.expect("expected else tail")),
                            "b"
                        );
                    });
                });
            });
        });
    });
}

/// Parse explicit branch semicolons as statements inside value-tail if expressions.
#[test]
fn test_parse_function_body_keeps_if_else_branch_semicolons_as_statements() {
    let test = TestParser::new(
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { a; } else { b; }
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    let function_expression_id = expressions[0];
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_node!(parser.tree, *body_id, Expression::Block(function_block_id) => {
                let function_block = parser.tree.get(*function_block_id);
                let tail_expression = function_block.tail_expression.expect("expected tail expression");

                assert_node!(parser.tree, tail_expression, Expression::If { then_expression, else_expression, .. } => {
                    assert_node!(parser.tree, *then_expression, Expression::Block(then_block_id) => {
                        let then_block = parser.tree.get(*then_block_id);
                        assert_eq!(then_block.leading_expressions.len(), 1);
                        assert!(then_block.tail_expression.is_none());
                    });

                    let else_expression = else_expression.expect("expected else expression");
                    assert_node!(parser.tree, else_expression, Expression::Block(else_block_id) => {
                        let else_block = parser.tree.get(*else_block_id);
                        assert_eq!(else_block.leading_expressions.len(), 1);
                        assert!(else_block.tail_expression.is_none());
                    });
                });
            });
        });
    });
}

/// Parse a function declaration followed by a call on the same line.
#[test]
fn test_parse_function_declaration_followed_by_call_without_newline() {
    let test = TestParser::new(
        "function main(){return 1}main().catch((function handle(error){console.error(error);process.exit(1)}));",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // function main(){...} main().catch(...)
    assert_eq!(expressions.len(), 2);

    // first expression: function declaration
    let declaration_id = expressions[0];
    assert_node!(parser.tree, declaration_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { .. }));
    });

    // second expression: call expression on `main().catch`
    let call_id = expressions[1];
    assert_node!(parser.tree, call_id, Expression::Call { left, .. } => {
        assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "catch");
            assert_node!(parser.tree, *left, Expression::Call { left, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "main");
            });
        });
    });
}

#[test]
fn test_return_no_value_before_close_brace() {
    let test = TestParser::new("return }");
    let mut parser = test.prepare();
    let return_id = parser.parse_return().unwrap();

    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_none());
    });
    assert!(parser.peek_is(TokenType::CloseBrace));
}

/// `return/*\n*/value` should omit the operand due to line terminator trivia.
#[test]
fn test_return_asi_with_block_comment_newline() {
    let test = TestParser::new("return/*\n*/value");
    let mut parser = test.prepare();
    let return_id = parser.parse_return().unwrap();

    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_none());
    });
    let value_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();
    assert_expression_path!(parser, parser.tree.get(value_id), "value");
}

#[test]
fn test_parse_return_semicolon_trailing_comment_on_statement_wrapper_owner() {
    let test = TestParser::new("return value; // return-tail");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let return_id = expressions[0];
    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_some());
    });
    let return_annotations = parser.tree.get_decorators(return_id.id);
    assert_eq!(return_annotations.len(), 0);

    let annotations = parser.tree.get_decorators(return_id.id);
    assert!(annotations.is_empty());
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "return-tail");
}

#[test]
fn test_parse_new_without_receiver_as_statement_recovers_missing_constructor() {
    let test = TestParser::new("new");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // top level statements
    assert_eq!(expressions.len(), 1);

    // new
    let new_id = expressions[0];
    assert_node!(parser.tree, new_id, Expression::New { left, arguments, .. } => {
        // missing constructor
        assert_node!(parser.tree, *left, Expression::Missing);

        // no dynamic arguments
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_without_receiver_before_newline_recovers_missing_constructor() {
    let test = TestParser::new(
        r#"
new
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "")]);

    // top level statements
    assert_eq!(expressions.len(), 1);

    // new
    let new_id = expressions[0];
    assert_node!(parser.tree, new_id, Expression::New { left, arguments, .. } => {
        // missing constructor
        assert_node!(parser.tree, *left, Expression::Missing);

        // no dynamic arguments
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_without_receiver_before_following_call_keeps_statement_shape() {
    let test = TestParser::new(
        r#"
new
next()
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(&parser, &[(Some(NodeType::Expression), None, None, "next")]);

    // statements
    assert_eq!(expressions.len(), 2);

    // new
    assert_node!(parser.tree, expressions[0], Expression::New { left, arguments, .. } => {
        assert_node!(parser.tree, *left, Expression::Missing);
        assert!(arguments.is_empty());
    });

    // next()
    assert_node!(parser.tree, expressions[1], Expression::Call { left, generic_arguments, arguments, .. } => {
        assert_value_expression_path!(parser, parser.tree.get(*left), "next");
        assert!(generic_arguments.is_empty());
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_without_receiver_before_following_const_keeps_statement_shape() {
    let test = TestParser::new(
        r#"
new
const value = 1
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    // diagnostics
    test.assert_errors(
        &parser,
        &[(Some(NodeType::Expression), None, None, "const")],
    );

    // statements
    assert_eq!(expressions.len(), 2);

    // new
    assert_node!(parser.tree, expressions[0], Expression::New { left, arguments, .. } => {
        assert_node!(parser.tree, *left, Expression::Missing);
        assert!(arguments.is_empty());
    });

    // const value = 1
    assert_node!(parser.tree, expressions[1], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
    });
}

#[test]
fn test_parse_return_tree_literal_with_close_paren_text_in_ternary_before_tree() {
    let test = TestParser::new(
        "function render(isEnabled) {
  return (
    <div>
      {isEnabled ? (
        <div>)</div>
      ) : null}
    </div>
  )
}",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let function_expression_id = expressions[0];
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body), .. }) => {
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                let return_id =
                    block.leading_expressions[0];
                assert_node!(parser.tree, return_id, Expression::Return { value: Some(value) } => {
                    crate::assert_parenthesized!(parser.tree, *value, expression => {
                        assert_node!(parser.tree, *expression, Expression::TreeExpression { .. });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_statement_separator_comment_before_semicolon_attaches_to_previous_statement() {
    let test = TestParser::new("declare const PAGE_PATH: string\n  //<- keep-marker\n;(()=>{})()");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 2);

    let first_annotations = parser.tree.get_decorators(expressions[0].id);
    assert!(first_annotations.is_empty());
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "<- keep-marker");
}

/// Build nested if statements with explicit block bodies.
fn nested_if_block_source(depth: usize) -> String {
    let mut source = String::new();

    // open nested if blocks
    for _ in 0..depth {
        source.push_str("if (true) {");
    }

    source.push('0');

    // close nested if blocks
    for _ in 0..depth {
        source.push('}');
    }

    source
}

/// Build nested if statements with single statement bodies.
fn nested_unbraced_if_source(depth: usize) -> String {
    let mut source = String::new();

    // open nested if statements
    for _ in 0..depth {
        source.push_str("if (true) ");
    }

    source.push_str("0;");

    source
}

/// Build nested while statements with single statement bodies.
fn nested_unbraced_while_source(depth: usize) -> String {
    let mut source = String::new();

    // open nested while statements
    for _ in 0..depth {
        source.push_str("while (true) ");
    }

    source.push(';');

    source
}

/// Parse deeply nested unbraced if statements without overflowing the parser stack.
#[test]
fn test_parse_deeply_nested_if_statement() {
    let source = nested_if_block_source(512);
    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    test.assert_no_errors(&parser);
}

/// Parse excessive statement nesting without overflowing the parser stack.
#[test]
fn test_parse_excessively_nested_if_statement() {
    let source = nested_if_block_source(1025);
    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    test.assert_no_errors(&parser);
}

/// Parse deeply nested unbraced if statements without overflowing the parser stack.
#[test]
fn test_parse_deeply_nested_unbraced_if_statement() {
    let source = nested_unbraced_if_source(1024);
    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    test.assert_no_errors(&parser);
}

/// Parse deeply nested unbraced while statements without overflowing the parser stack.
#[test]
fn test_parse_deeply_nested_unbraced_while_statement() {
    let source = nested_unbraced_while_source(1024);
    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    test.assert_no_errors(&parser);
}
