use destack_dir::{
    Block, BlockContext, CommentKind, Declaration, Expression, FunctionDeclaration, FunctionForm,
    IfForm, Key, LetKind, MatchCase, Name, NodeType, Property, ScalarLiteral, TokenType,
    YieldCardinality,
};
use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

use crate::{
    ParserOptions, ParserTriviaMode, TestParser, assert_comment, assert_expression_path,
    assert_node, assert_string, block_expression_ids,
};

#[test]
fn test_parse_empty_block() {
    let mut test = TestParser::new("{}");
    let mut parser = test.prepare();
    let block_id = parser.eat_block(BlockContext::Expression).unwrap();
    let block = parser.tree.get(block_id);
    assert!(block.is_empty());
}

#[test]
fn test_parse_block_with_missing_close_brace() {
    let mut test = TestParser::new("{ value");
    let mut parser = test.prepare();
    let block_id = parser.eat_block(BlockContext::Expression).unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, block_id, Block { leading_expressions, tail_expression, .. } => {
        assert!(leading_expressions.is_empty());
        let tail_expression = tail_expression.expect("expected tail expression");
        assert_expression_path!(parser, parser.tree.get(tail_expression), "value");
    });
}

#[test]
fn test_parse_root_unmatched_close_brace_recovery() {
    let mut test = TestParser::new("}\nnextValue");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Error);
    assert_expression_path!(parser, parser.tree.get(expressions[1]), "nextValue");
}

#[test]
fn test_parse_root_unmatched_close_parenthesis_recovery() {
    let mut test = TestParser::new(")\nnextValue");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Error);
    assert_expression_path!(parser, parser.tree.get(expressions[1]), "nextValue");
}

#[test]
fn test_statement_expression_separator_with_comment_newline() {
    let mut test =
        TestParser::new_with_language("'use strict' /**/ \n nextValue", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let (directive_id, is_statement) = parser.try_eat_statement_expression_classified().unwrap();
    assert!(!is_statement);
    assert_node!(parser.tree, directive_id, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
        assert_string!(parser, *string_id, "use strict");
    });

    let next_id = parser.try_eat_statement_expression().unwrap();
    assert_expression_path!(parser, parser.tree.get(next_id), "nextValue");
}

#[test]
fn test_break_no_label_no_value() {
    let mut test = TestParser::new("break");
    let mut parser = test.prepare();
    let break_id = parser.eat_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label: None, value } => {
        assert!(value.is_none());
    });
}

#[test]
fn test_break_with_label() {
    let mut test = TestParser::new("break label");
    let mut parser = test.prepare();
    let break_id = parser.eat_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
        assert_string!(parser, label.unwrap(), "label");
        assert!(value.is_none());
    });

    let main_span = parser
        .tree
        .get_main_span(break_id)
        .expect("expected break label span");
    assert_eq!(parser.get_span_str(main_span), "label");
}

#[test]
fn test_break_with_label_and_value() {
    let mut test = TestParser::new("break label: 17");
    let mut parser = test.prepare();
    let break_id = parser.eat_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
        assert_string!(parser, label.unwrap(), "label");
        assert!(value.is_some());
        assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(17)));
    });
}

#[test]
fn test_break_with_parenthesized_identifier_value() {
    let mut test = TestParser::new("break (value)");
    let mut parser = test.prepare();
    let break_id = parser.eat_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label: None, value } => {
        assert!(value.is_some());
        assert_node!(parser.tree, value.unwrap(), Expression::Parenthesized { expression } => {
            assert_expression_path!(parser, parser.tree.get(*expression), "value");
        });
    });
}

#[test]
fn test_continue_no_label() {
    let mut test = TestParser::new("continue");
    let mut parser = test.prepare();
    let continue_id = parser.eat_continue().unwrap();
    assert_node!(parser.tree, continue_id, Expression::Continue { label: None } => {
    });
}

#[test]
fn test_continue_with_label() {
    let mut test = TestParser::new("continue label");
    let mut parser = test.prepare();
    let continue_id = parser.eat_continue().unwrap();
    assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
        assert_string!(parser, label.unwrap(), "label");
    });

    let main_span = parser
        .tree
        .get_main_span(continue_id)
        .expect("expected continue label span");
    assert_eq!(parser.get_span_str(main_span), "label");
}

#[test]
fn test_break_label_before_newline() {
    let mut test = TestParser::new("break foo\n");
    let mut parser = test.prepare();
    let break_id = parser.eat_break().unwrap();
    assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
        assert_string!(parser, label.unwrap(), "foo");
        assert!(value.is_none());
    });
}

#[test]
fn test_continue_label_before_newline() {
    let mut test = TestParser::new("continue foo\n");
    let mut parser = test.prepare();
    let continue_id = parser.eat_continue().unwrap();
    assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
        assert_string!(parser, label.unwrap(), "foo");
    });
}

#[test]
fn test_await_expression() {
    let mut test = TestParser::new("await someFunction()");
    let mut parser = test.prepare();
    let await_id = parser.eat_await().unwrap();
    // await someFunction()
    assert_node!(parser.tree, await_id, Expression::Await { expression } => {
        // someFunction()
        assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_await_maybe_expression() {
    let mut test = TestParser::new("await? someFunction()");
    let mut parser = test.prepare();
    let await_id = parser.eat_await().unwrap();
    // await? someFunction()
    assert_node!(parser.tree, await_id, Expression::AwaitMaybe { expression } => {
        // someFunction()
        assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_await_must_expression() {
    let mut test = TestParser::new("await! someFunction()");
    let mut parser = test.prepare();
    let await_id = parser.eat_await().unwrap();

    // await! someFunction()
    assert_node!(parser.tree, await_id, Expression::AwaitMust { expression } => {
        // someFunction()
        assert_node!(parser.tree, *expression, Expression::Call { position: _, left, generic_arguments: _, arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_comptime_expression() {
    let mut test = TestParser::new("comptime factorial(10)");
    let mut parser = test.prepare();
    let comptime_id = parser.eat_comptime().unwrap();
    // comptime factorial(10)
    assert_node!(parser.tree, comptime_id, Expression::Comptime { body } => {
        // factorial(10)
        assert_node!(parser.tree, *body, Expression::Call { position: _, left, generic_arguments: _, arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "factorial");
            assert_eq!(arguments.len(), 1);
        });
    });
}

#[test]
fn test_comptime_expression_simple() {
    // comptime 1 + 2
    let mut test = TestParser::new("comptime 1 + 2");
    let mut parser = test.prepare();
    let comptime_id = parser.eat_comptime().unwrap();
    // comptime 1 + 2
    assert_node!(parser.tree, comptime_id, Expression::Comptime { body } => {
        // 1 + 2
        assert_node!(parser.tree, *body, Expression::Binary { .. } => {
            // binary addition
        });
    });
}

#[test]
fn test_comptime_block_expression() {
    let mut test = TestParser::new("comptime { let x = 1; x + 2 }");
    let mut parser = test.prepare();
    let comptime_id = parser.eat_comptime().unwrap();
    assert_node!(parser.tree, comptime_id, Expression::Comptime { body } => {
        assert_node!(parser.tree, *body, Expression::Block(_));
    });
}

#[test]
fn test_yield_expression() {
    let mut test = TestParser::new("yield someFunction()");
    let mut parser = test.prepare();
    let yield_id = parser.eat_yield().unwrap();
    // yield someFunction()
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        // someFunction()
        assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, generic_arguments: _, arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_yield_expression_no_value() {
    let mut test = TestParser::new("yield");
    let mut parser = test.prepare();
    let yield_id = parser.eat_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none());
    });
}

#[test]
fn test_yield_expression_no_value_before_close_parenthesis() {
    // source: yield)
    let mut test = TestParser::new("yield)");
    let mut parser = test.prepare();
    let yield_id = parser.eat_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none());
    });
    assert!(parser.peek_is(TokenType::CloseParenthesis));
}

#[test]
fn test_yield_expression_no_value_before_close_bracket() {
    // source: yield]
    let mut test = TestParser::new("yield]");
    let mut parser = test.prepare();
    let yield_id = parser.eat_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none());
    });
    assert!(parser.peek_is(TokenType::CloseBracket));
}

#[test]
fn test_yield_expression_generator() {
    let mut test = TestParser::new("yield* someFunction()");
    let mut parser = test.prepare();
    let yield_id = parser.eat_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Generator);
        // someFunction()
        assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, generic_arguments: _, arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
            assert!(arguments.is_empty());
        });
    });
}

#[test]
fn test_yield_expression_generator_with_space() {
    let mut test = TestParser::new("yield *a");
    let mut parser = test.prepare();
    let yield_id = parser.eat_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Generator);
        assert!(value.is_some());
    });
}

#[test]
fn test_reject_yield_star_without_operand() {
    // source: yield*
    let mut test = TestParser::new("yield*");
    let mut parser = test.prepare();
    let yield_id = parser.eat_yield().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // yield*
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Generator);
        assert_node!(parser.tree, value.expect("expected missing generator operand"), Expression::Missing);
    });
}

/// `yield\n*a` should NOT be parsed as `yield* a` due to ASI restricted production.
#[test]
fn test_yield_asi_with_newline() {
    let mut test = TestParser::new("yield\n*a");
    let mut parser = test.prepare();
    let yield_id = parser.eat_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none()); // ASI applied, no value
    });
}

/// `yield\n*a` should NOT be parsed as `yield* a` due to ASI restricted production.
#[test]
fn test_yield_asi_with_newline_js_mode() {
    let language = LanguageType::JavaScript;
    let mut test = TestParser::new_with_language("yield\n*a", language);
    let mut parser = test.prepare();

    // yield parses fine with ASI
    let yield_id = parser.eat_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none()); // ASI applied, no value
    });

    // try to parse *a as next statement: should fail in untyped value mode
    // because * is not valid as a unary prefix there
    let error = parser.eat_expression(parser.flags).unwrap_err();

    // *
    assert_eq!(parser.get_span_str(error.leaf_span()), "*");
}

/// `yield/*\n*/*a` should not be parsed as `yield* a`.
#[test]
fn test_yield_asi_with_block_comment_newline_js_mode() {
    // source: yield/*\n*/*a
    let mut test = TestParser::new_with_language("yield/*\n*/*a", LanguageType::JavaScript);
    let mut parser = test.prepare();

    let yield_id = parser.eat_yield().unwrap();
    assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
        assert_eq!(*cardinality, YieldCardinality::Scalar);
        assert!(value.is_none());
    });

    let error = parser.eat_expression(parser.flags).unwrap_err();

    // *
    assert_eq!(parser.get_span_str(error.leaf_span()), "*");
}

#[test]
fn test_throw_expression_with_value() {
    let mut test = TestParser::new("throw 17");
    let mut parser = test.prepare();
    let throw_id = parser.eat_throw().unwrap();
    assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(17)));
    });
}

/// Reject throw expressions split by a block comment newline.
#[test]
fn test_reject_throw_expression_with_block_comment_newline() {
    // source: throw /*\n*/ e
    let mut test = TestParser::new_with_language("throw /*\n*/ e", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let throw_id = parser.eat_throw().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "e")]);

    // throw /*\n*/
    assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
        assert_node!(parser.tree, *value, Expression::Missing);
    });

    // e
    assert!(parser.peek_is(TokenType::Identifier));
}

/// Reject throw expressions split by unicode line separator comments.
#[test]
fn test_reject_throw_expression_with_line_separator_comment() {
    // source: throw /* \u{2028} */ e
    let mut test =
        TestParser::new_with_language("throw /* \u{2028} */ e", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let throw_id = parser.eat_throw().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "e")]);

    // throw /* \u{2028} */
    assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
        assert_node!(parser.tree, *value, Expression::Missing);
    });

    // e
    assert!(parser.peek_is(TokenType::Identifier));
}

#[test]
fn test_parse_throw_without_value_recovers_missing_expression() {
    let mut test = TestParser::new("throw");
    let mut parser = test.prepare();
    let throw_id = parser.eat_throw().unwrap();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // throw
    assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
        assert_node!(parser.tree, *value, Expression::Missing);
    });
}

#[test]
fn test_parse_throw_without_value_before_newline_keeps_following_statement_shape() {
    let mut test = TestParser::new(
        r#"
throw
next()
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "next")]);

    // statements
    assert_eq!(expressions.len(), 2);

    // throw
    assert_node!(parser.tree, expressions[0], Expression::Throw { value } => {
            assert_node!(parser.tree, *value, Expression::Missing);
    });

    // next()
    assert_node!(parser.tree, expressions[1], Expression::Call { left, generic_arguments, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "next");
            assert!(generic_arguments.is_empty());
            assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_throw_without_value_before_newline_keeps_following_const_shape() {
    let mut test = TestParser::new(
        r#"
throw
const value = 1
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "const")]);

    // statements
    assert_eq!(expressions.len(), 2);

    // throw
    assert_node!(parser.tree, expressions[0], Expression::Throw { value } => {
            assert_node!(parser.tree, *value, Expression::Missing);
    });

    // const value = 1
    assert_node!(parser.tree, expressions[1], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
    });
}

#[test]
fn test_return_no_value() {
    let mut test = TestParser::new("return");
    let mut parser = test.prepare();
    let return_id = parser.eat_return().unwrap();
    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_none());
    });
}

#[test]
fn test_return_with_value() {
    let mut test = TestParser::new("return 42");
    let mut parser = test.prepare();
    let return_id = parser.eat_return().unwrap();
    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_some());
        assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(42)));
    });
}

#[test]
fn test_parse_block_const_then_return_cast() {
    let mut test = TestParser::new_with_language(
        "{\n  const result = CreateRecord(IntegerKey, value)\n  return result as never\n}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let block_id = parser.eat_block(BlockContext::Expression).unwrap();
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
    let mut test = TestParser::new_with_language(
        "return Result.IsExtendsTrueLike(check)\n  ? TryInferResults(tail, right, [...result, head])\n  : undefined",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let return_id = parser.eat_return().unwrap();

    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        let value = value.expect("expected return value");
        assert_node!(parser.tree, value, Expression::If { form, .. } => {
            assert_eq!(*form, IfForm::Ternary);
        });
    });
}

#[test]
fn test_parse_block_statement_before_close_brace_without_semicolon() {
    let mut test = TestParser::new_with_language("{ process.exit(1)}", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let block_id = parser.eat_block(BlockContext::Expression).unwrap();
    let block = parser.tree.get(block_id);

    // block should contain one leading statement expression
    assert_eq!(block.leading_expressions.len(), 1);
    assert!(block.tail_expression.is_none());
    assert_node!(
        parser.tree,
        block.leading_expressions[0],
        Expression::Call { .. }
    );
}

/// Parse semicolon led parenthesized calls without parenthesized wrappers.
#[test]
fn test_parse_statement_leading_semicolon_parenthesized_arrow_call_without_wrappers() {
    let mut test = TestParser::new_with_language("{\n;(()=>{})()\n}", LanguageType::Destack);
    let mut parser = test.prepare();
    parser.apply_options(ParserOptions {
        trivia_mode: ParserTriviaMode::Full,
        preserve_parenthesized_wrappers: false,
        ..ParserOptions::default()
    });
    let block_id = parser.eat_block(BlockContext::Expression).unwrap();
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

/// Record statement source spans when parenthesized wrappers are skipped.
#[test]
fn test_parse_statement_span_preserves_skipped_parenthesized_wrapper() {
    let mut test = TestParser::new_with_language("(() => value);", LanguageType::TypeScript);
    let mut parser = test.prepare();
    parser.apply_options(ParserOptions {
        trivia_mode: ParserTriviaMode::Full,
        preserve_parenthesized_wrappers: false,
        ..ParserOptions::default()
    });

    let (expression_id, is_statement) = parser.try_eat_statement_expression_classified().unwrap();
    let statement_span = parser
        .tree
        .get_side_span(
            expression_id,
            NodeSpanType::Region(NodeSpanRegion::Statement),
        )
        .expect("missing statement span");

    assert!(is_statement);
    assert_eq!(parser.get_span_str(statement_span), "(() => value);");
}

/// Record statement source spans for root tail statements.
#[test]
fn test_parse_root_statement_span_preserves_skipped_parenthesized_wrapper() {
    let mut test = TestParser::new_with_language("(() => value);", LanguageType::TypeScript);
    let mut parser = test.prepare();
    parser.apply_options(ParserOptions {
        trivia_mode: ParserTriviaMode::Full,
        preserve_parenthesized_wrappers: false,
        ..ParserOptions::default()
    });

    let expressions = parser.parse();
    let statement_span = parser
        .tree
        .get_side_span(
            expressions[0],
            NodeSpanType::Region(NodeSpanRegion::Statement),
        )
        .expect("missing statement span");

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.get_span_str(statement_span), "(() => value);");
}

/// Record root expression statement spans before skipped wrappers.
#[test]
fn test_parse_root_statement_span_preserves_leading_parenthesized_wrapper() {
    let mut test = TestParser::new_with_language(
        "const a = 1\n\n;(function() {})()",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    parser.apply_options(ParserOptions {
        trivia_mode: ParserTriviaMode::Full,
        preserve_parenthesized_wrappers: false,
        ..ParserOptions::default()
    });

    let expressions = parser.parse();
    let statement_span = parser
        .tree
        .get_side_span(
            expressions[1],
            NodeSpanType::Region(NodeSpanRegion::Statement),
        )
        .expect("missing statement span");

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.get_span_str(statement_span), "(function() {})()");
}

/// Parse if-body block tails as value expressions.
#[test]
fn test_parse_if_block_keeps_tail_expression_value() {
    let mut test = TestParser::new("if (x) { foo() }");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // if (x) { foo() }
    assert_eq!(expressions.len(), 1);
    let if_expression_id = parser.unwrap_label_expression(expressions[0]);
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
    let mut test = TestParser::new("function run() { foo() }");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function run() { foo() }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = parser.unwrap_label_expression(expressions[0]);
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
    let mut test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);

    // function next(...) { drop(value); { done: true, value } }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = parser.unwrap_label_expression(expressions[0]);
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
                    assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
                        assert_string!(parser, *name, "done");
                        assert!(!*is_shorthand);
                        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                    });
                    assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
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
    let mut test = TestParser::new(input);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);

    // function apply(...) { match (...) { ... } }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body_id), .. }) => {
            assert_node!(parser.tree, *body_id, Expression::Block(function_block_id) => {
                let function_block = parser.tree.get(*function_block_id);
                let tail_expression = function_block.tail_expression.expect("expected match tail");

                assert_node!(parser.tree, tail_expression, Expression::Match { cases, .. } => {
                    assert_eq!(cases.len(), 2);
                    for case_id in cases {
                        assert_node!(parser.tree, *case_id, MatchCase::Block { body: case_block_id, .. } => {
                            let case_block = parser.tree.get(*case_block_id);
                            let case_tail = case_block.tail_expression.expect("expected object tail");

                            assert_node!(parser.tree, case_tail, Expression::ObjectExpression { properties, .. } => {
                                assert_eq!(properties.len(), 2);
                                assert_node!(parser.tree, properties[0], Property::Field { key: Key::Name(Name::Identifier(name)), value, .. } => {
                                    assert_string!(parser, *name, "done");
                                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(_)));
                                });
                                assert_node!(parser.tree, properties[1], Property::Field { key: Key::Name(Name::Identifier(name)), value, is_shorthand } => {
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
    let mut test = TestParser::new(
        r#"
function run() {
    foo()
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function run() { foo() }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = parser.unwrap_label_expression(expressions[0]);
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
    let mut test = TestParser::new(
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { a } else { b }
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function choose(...) { if (flag) { a } else { b } }
    assert_eq!(expressions.len(), 1);
    let function_expression_id = parser.unwrap_label_expression(expressions[0]);
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
    let mut test = TestParser::new(
        r#"
function choose(flag: boolean, a: int32, b: int32): int32 {
    if (flag) { a; } else { b; }
}
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    let function_expression_id = parser.unwrap_label_expression(expressions[0]);
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

/// Parse a function declaration followed by a call on the same line in JavaScript.
#[test]
fn test_parse_function_declaration_followed_by_call_without_newline() {
    let mut test = TestParser::new_with_language(
        "function main(){return 1}main().catch((function(error){console.error(error);process.exit(1)}));",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // function main(){...} main().catch(...)
    assert_eq!(expressions.len(), 2);

    // first expression: function declaration
    let declaration_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, declaration_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { .. }));
    });

    // second expression: call expression on `main().catch`
    let call_id = parser.unwrap_label_expression(expressions[1]);
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
fn test_parse_block_sequence_statement_with_newlines_after_commas() {
    let mut test = TestParser::new_with_language(
        r#"{
  callA(),
  callB(),
  callC()
}"#,
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let block_id = parser.eat_block(BlockContext::Expression).unwrap();
    let block = parser.tree.get(block_id);

    let expressions = block_expression_ids(block);
    assert_eq!(expressions.len(), 1);
    assert!(block.tail_expression.is_none());

    // leading expression should be one sequence expression
    assert_node!(parser.tree, expressions[0], Expression::SequenceExpression { expressions } => {
        assert_eq!(expressions.len(), 3);
        assert_node!(parser.tree, expressions[0], Expression::Call { left, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "callA");
        });
        assert_node!(parser.tree, expressions[1], Expression::Call { left, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "callB");
        });
        assert_node!(parser.tree, expressions[2], Expression::Call { left, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "callC");
        });
    });
}

#[test]
fn test_return_no_value_before_close_brace() {
    let mut test = TestParser::new("return }");
    let mut parser = test.prepare();
    let return_id = parser.eat_return().unwrap();

    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_none());
    });
    assert!(parser.peek_is(TokenType::CloseBrace));
}

/// `return/*\n*/value` should omit the operand due to line terminator trivia.
#[test]
fn test_return_asi_with_block_comment_newline_js_mode() {
    let mut test = TestParser::new_with_language("return/*\n*/value", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let return_id = parser.eat_return().unwrap();

    assert_node!(parser.tree, return_id, Expression::Return { value } => {
        assert!(value.is_none());
    });
    let value_id = parser.eat_expression(parser.flags).unwrap();
    assert_expression_path!(parser, parser.tree.get(value_id), "value");
}

#[test]
fn test_parse_throw_trailing_comment_on_statement_wrapper_owner() {
    let mut test =
        TestParser::new_with_language("throw error // throw-tail", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let throw_id = expressions[0];
    assert_node!(parser.tree, throw_id, Expression::Throw { .. } => {});
    let throw_annotations = parser.tree.get_decorators(throw_id.id);
    assert_eq!(throw_annotations.len(), 0);

    let annotations = parser.tree.get_decorators(throw_id.id);
    assert!(annotations.is_empty());
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "throw-tail");
}

#[test]
fn test_parse_throw_semicolon_trailing_comment_on_statement_wrapper_owner() {
    let mut test =
        TestParser::new_with_language("throw error; // throw-tail", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let throw_id = expressions[0];
    assert_node!(parser.tree, throw_id, Expression::Throw { .. } => {});
    let throw_annotations = parser.tree.get_decorators(throw_id.id);
    assert_eq!(throw_annotations.len(), 0);

    let annotations = parser.tree.get_decorators(throw_id.id);
    assert!(annotations.is_empty());
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "throw-tail");
}

#[test]
fn test_parse_return_semicolon_trailing_comment_on_statement_wrapper_owner() {
    let mut test =
        TestParser::new_with_language("return value; // return-tail", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

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
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "return-tail");
}

#[test]
fn test_parse_new_without_receiver_as_statement_recovers_missing_constructor() {
    let mut test = TestParser::new("new");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // top level statements
    assert_eq!(expressions.len(), 1);

    // new
    let new_id = expressions[0];
    assert_node!(parser.tree, new_id, Expression::New { left, generic_arguments, arguments } => {
        // missing constructor
        assert_node!(parser.tree, *left, Expression::Missing);

        // no generic arguments
        assert!(generic_arguments.is_empty());

        // no dynamic arguments
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_without_receiver_before_newline_recovers_missing_constructor() {
    let mut test = TestParser::new(
        r#"
new
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "")]);

    // top level statements
    assert_eq!(expressions.len(), 1);

    // new
    let new_id = expressions[0];
    assert_node!(parser.tree, new_id, Expression::New { left, generic_arguments, arguments } => {
        // missing constructor
        assert_node!(parser.tree, *left, Expression::Missing);

        // no generic arguments
        assert!(generic_arguments.is_empty());

        // no dynamic arguments
        assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_without_receiver_before_following_call_keeps_statement_shape() {
    let mut test = TestParser::new(
        r#"
new
next()
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "next")]);

    // statements
    assert_eq!(expressions.len(), 2);

    // new
    assert_node!(parser.tree, expressions[0], Expression::New { left, generic_arguments, arguments } => {
            assert_node!(parser.tree, *left, Expression::Missing);
            assert!(generic_arguments.is_empty());
            assert!(arguments.is_empty());
    });

    // next()
    assert_node!(parser.tree, expressions[1], Expression::Call { left, generic_arguments, arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "next");
            assert!(generic_arguments.is_empty());
            assert!(arguments.is_empty());
    });
}

#[test]
fn test_parse_new_without_receiver_before_following_const_keeps_statement_shape() {
    let mut test = TestParser::new(
        r#"
new
const value = 1
"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    // diagnostics
    test.assert_error_leaves(&parser, &[(Some(NodeType::Expression), None, "const")]);

    // statements
    assert_eq!(expressions.len(), 2);

    // new
    assert_node!(parser.tree, expressions[0], Expression::New { left, generic_arguments, arguments } => {
            assert_node!(parser.tree, *left, Expression::Missing);
            assert!(generic_arguments.is_empty());
            assert!(arguments.is_empty());
    });

    // const value = 1
    assert_node!(parser.tree, expressions[1], Expression::Let { declarators, .. } => {
        assert_eq!(declarators.len(), 1);
    });
}

#[test]
fn test_parse_function_throw_semicolon_trailing_comment_on_statement_wrapper_owner() {
    let mut test = TestParser::new_with_language(
        "function fail() {\n    throw error; // throw-tail\n}",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let function_expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body, .. }) => {
            let body_id = body.expect("expected function body");
            assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());

                let throw_id = block.leading_expressions[0];
                assert_node!(parser.tree, throw_id, Expression::Throw { .. } => {});
                let throw_annotations = parser.tree.get_decorators(throw_id.id);
                assert_eq!(throw_annotations.len(), 0);

                let annotations = parser.tree.get_decorators(throw_id.id);
                assert!(annotations.is_empty());
                assert_eq!(parser.tree.comments().len(), 1);
                assert_comment!(parser, 0, CommentKind::Line, "throw-tail");
            });
        });
    });
}
#[test]
fn test_parse_return_tree_literal_with_close_paren_text_in_ternary_before_tree() {
    let mut test = TestParser::new_with_language(
        "function render(isEnabled) {
  return (
    <div>
      {isEnabled ? (
        <div>)</div>
      ) : null}
    </div>
  )
}",
        LanguageType::TypeScriptXml,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let function_expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, function_expression_id, Expression::Declaration(function_id) => {
        assert_node!(parser.tree, *function_id, Declaration::Function(FunctionDeclaration { body: Some(body), .. }) => {
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                let block = parser.tree.get(*block_id);
                assert_eq!(block.leading_expressions.len(), 1);
                assert!(block.tail_expression.is_none());
                let return_id =
                    parser.unwrap_label_expression(block.leading_expressions[0]);
                assert_node!(parser.tree, return_id, Expression::Return { value: Some(value) } => {
                    assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                        assert_node!(parser.tree, *expression, Expression::TreeExpression { .. });
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_statement_separator_comment_before_semicolon_attaches_to_previous_statement() {
    let mut test = TestParser::new_with_language(
        "declare const PAGE_PATH: string\n  //<- keep-marker\n;(()=>{})()",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 2);

    let first_annotations = parser.tree.get_decorators(expressions[0].id);
    assert!(first_annotations.is_empty());
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "<- keep-marker");
}

/// Parse deeply nested JavaScript if statements without overflowing the parser stack.
#[test]
fn test_parse_deeply_nested_if_statement() {
    let depth = 512;
    let mut source = String::new();

    // open nested if blocks
    for _ in 0..depth {
        source.push_str("if (true) {");
    }

    // terminal block expression
    source.push('0');

    // close nested if blocks
    for _ in 0..depth {
        source.push('}');
    }

    let mut test = TestParser::new_with_language(&source, LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 1);
    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
}
