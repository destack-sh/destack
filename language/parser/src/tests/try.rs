use tspp_dir::{
    Argument, Block, Catch, Expression, Name, Pattern, PatternField, TypeExpression, TypeLiteral,
};

use crate::{TestParser, assert_expression_path, assert_node, assert_string, block_expression_ids};

#[test]
fn test_parse_try_expression() {
    let test = TestParser::new(
        r###"
try foo()
"###,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    let main_span = parser
        .tree
        .get_main_span(try_id)
        .expect("expected try keyword main span");
    assert_eq!(parser.span_str(main_span), "try");

    assert_node!(parser.tree, try_id, Expression::Try { body, catch: None, finally: None } => {
        assert_node!(parser.tree, *body, Expression::Call { left, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "foo");
        });
    });
}

#[test]
fn test_parse_try_block_without_catch_or_finally() {
    let test = TestParser::new(
        r###"
try {
    foo()
}
"###,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    let main_span = parser
        .tree
        .get_main_span(try_id)
        .expect("expected try keyword main span");
    assert_eq!(parser.span_str(main_span), "try");

    assert_node!(parser.tree, try_id, Expression::Try { body, catch: None, finally: None } => {
        assert_node!(parser.tree, *body, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let call_id = expressions[0];
                assert_node!(parser.tree, call_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                });
            });
        });
    });
}

#[test]
fn test_try_block_wrapper_keeps_block_span() {
    let test = TestParser::new(
        r###"
try /* comment */ {
    foo()
}
"###,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    assert_node!(parser.tree, try_id, Expression::Try { body, .. } => {
        let body_span = parser.tree.get_span(*body);
        assert_eq!(parser.span_str(body_span), "{\n    foo()\n}");
    });
}

#[test]
fn test_parse_try_with_catch_and_finally() {
    let test = TestParser::new(
        r###"
try {
    foo()
} catch (e) {
    bar()
} finally {
    baz()
}
"###,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    assert_node!(parser.tree, try_id, Expression::Try { body, catch: Some(catch), finally: Some(finally) } => {
        // try
        assert_node!(parser.tree, *body, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let try_call_id = expressions[0];
                assert_node!(parser.tree, try_call_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "foo");
                });
            });
        });
        // catch
        assert_node!(parser.tree, *catch, Catch { pattern: Some(catch_pattern), ty: None, body } => {
            assert_node!(parser.tree, *catch_pattern, Pattern::Binding { name, pattern: _ } => {
                assert_string!(parser, *name, "e");
            });
            assert_node!(parser.tree, *body, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    let catch_call_id = expressions[0];
                    assert_node!(parser.tree, catch_call_id, Expression::Call { left, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "bar");
                    });
                });
            });
        });
        // finally expression
        assert_node!(parser.tree, *finally, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                let finally_call_id = expressions[0];
                assert_node!(parser.tree, finally_call_id, Expression::Call { left, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "baz");
                });
            });
        });
    });
}

#[test]
fn test_parse_try_with_typed_catch_pattern() {
    let test = TestParser::new(
        r###"
try {
    foo()
} catch (ex: Error) {
    bar()
}
"###,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    assert_node!(parser.tree, try_id, Expression::Try { catch: Some(catch), .. } => {
        assert_node!(parser.tree, *catch, Catch { pattern: Some(catch_pattern), ty: Some(catch_ty), .. } => {
        // catch binding
        assert_node!(parser.tree, *catch_pattern, Pattern::Binding { name, pattern: None, .. } => {
            assert_string!(parser, *name, "ex");
        });
        // catch type
        assert_expression_path!(parser, parser.tree.get(*catch_ty), "Error");
        });
    });
}

#[test]
fn test_parse_try_with_catch_match_object_patterns() {
    let test = TestParser::new(
        r###"
try {
    first + second
} catch match (failure) {
    MissingError { path } => path.length;
    FormatError { line } => line;
}
"###,
    );
    let mut parser = test.prepare();

    let _try_id = parser.parse_try().unwrap();

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_try_stress_error_form() {
    let test = TestParser::new(
        r###"
async function errorCase(): Promise<int32> {
    const first = read()?;
    const second = await? readAsync();
    const third = await! readAsync();
    const local = read() ?? fallback();
    const forced = read()!;
    const recovered = try {
        first + second + third + local + forced
    } catch match (failure) {
        MissingError { path } => path.length;
        FormatError { line } => line;
    };
    return recovered;
}
"###,
    );
    let mut parser = test.prepare();

    parser.parse_in_place();

    test.assert_no_errors(&parser);
}

/// Parse branch tails as value expressions inside try branches.
#[test]
fn test_parse_try_keeps_branch_tail_expression_values() {
    let test = TestParser::new(
        r#"
try {
    value()
} catch (error) {
    fallback(error)
} finally {
    cleanup()
}
"#,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();

    assert_node!(parser.tree, try_id, Expression::Try { body, catch: Some(catch), finally: Some(finally), .. } => {
        assert_node!(parser.tree, *body, Expression::Block(try_block_id) => {
            let try_block = parser.tree.get(*try_block_id);
            assert_eq!(try_block.leading_expressions.len(), 0);
            let tail_expression = try_block.tail_expression.expect("expected try tail");
            assert_node!(parser.tree, tail_expression, Expression::Call { left, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "value");
                assert_eq!(arguments.len(), 0);
            });
        });

        assert_node!(parser.tree, *catch, Catch { body, .. } => {
        assert_node!(parser.tree, *body, Expression::Block(catch_block_id) => {
            let catch_block = parser.tree.get(*catch_block_id);
            assert_eq!(catch_block.leading_expressions.len(), 0);
            let tail_expression = catch_block.tail_expression.expect("expected catch tail");
            assert_node!(parser.tree, tail_expression, Expression::Call { left, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "fallback");
                assert_eq!(arguments.len(), 1);
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "error");
                });
            });
        });
        });

        assert_node!(parser.tree, *finally, Expression::Block(finally_block_id) => {
            let finally_block = parser.tree.get(*finally_block_id);
            assert_eq!(finally_block.leading_expressions.len(), 0);
            let tail_expression = finally_block.tail_expression.expect("expected finally tail");
            assert_node!(parser.tree, tail_expression, Expression::Call { left, arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "cleanup");
                assert_eq!(arguments.len(), 0);
            });
        });
    });
}

/// Parse explicit branch semicolons as statements inside try branches.
#[test]
fn test_parse_try_keeps_explicit_branch_semicolons_as_statements() {
    let test = TestParser::new(
        r#"
try {
    value();
} catch (error) {
    fallback(error);
} finally {
    cleanup();
}
"#,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();

    assert_node!(parser.tree, try_id, Expression::Try { body, catch: Some(catch), finally: Some(finally), .. } => {
        assert_node!(parser.tree, *body, Expression::Block(try_block_id) => {
            let try_block = parser.tree.get(*try_block_id);
            assert_eq!(try_block.leading_expressions.len(), 1);
            assert!(try_block.tail_expression.is_none());
        });

        assert_node!(parser.tree, *catch, Catch { body, .. } => {
        assert_node!(parser.tree, *body, Expression::Block(catch_block_id) => {
            let catch_block = parser.tree.get(*catch_block_id);
            assert_eq!(catch_block.leading_expressions.len(), 1);
            assert!(catch_block.tail_expression.is_none());
        });
        });

        assert_node!(parser.tree, *finally, Expression::Block(finally_block_id) => {
            let finally_block = parser.tree.get(*finally_block_id);
            assert_eq!(finally_block.leading_expressions.len(), 1);
            assert!(finally_block.tail_expression.is_none());
        });
    });
}

/// Recover a missing catch close parenthesis in place.
#[test]
fn test_parse_try_with_missing_catch_close_parenthesis() {
    let test = TestParser::new(
        r###"
try {
    foo()
} catch (ex: Error {
    bar()
}
"###,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, try_id, Expression::Try { catch: Some(catch), .. } => {
        assert_node!(parser.tree, *catch, Catch { pattern: Some(catch_pattern), ty: Some(catch_ty), body } => {
        assert_node!(parser.tree, *catch_pattern, Pattern::Binding { name, pattern: None, .. } => {
            assert_string!(parser, *name, "ex");
        });
        assert_expression_path!(parser, parser.tree.get(*catch_ty), "Error");
        assert_node!(parser.tree, *body, Expression::Block(..));
        });
    });
}

/// Parse typed destructuring catch patterns.
#[test]
fn test_parse_try_with_typed_destructuring_catch_pattern() {
    let test = TestParser::new(
        r###"
try {
    foo()
} catch ({ name, message }: unknown) {
    bar()
}
"###,
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    assert_node!(parser.tree, try_id, Expression::Try { catch: Some(catch), .. } => {
        assert_node!(parser.tree, *catch, Catch { pattern: Some(catch_pattern), ty: Some(catch_ty), body } => {
        // catch { name, message }
        assert_node!(parser.tree, *catch_pattern, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                assert_node!(name, Name::Identifier(name) => {
                    assert_string!(parser, *name, "name");
                });
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                assert_node!(name, Name::Identifier(name) => {
                    assert_string!(parser, *name, "message");
                });
            });
        });

        // catch annotation
        assert_node!(parser.tree, *catch_ty, TypeExpression::Keyword { value } => {
            assert_eq!(*value, TypeLiteral::Unknown);
        });

        // catch body
        assert_node!(parser.tree, *body, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                assert_node!(parser.tree, expressions[0], Expression::Call { left, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "bar");
                });
            });
        });
        });
    });
}

/// Parse catch expression parameters as expression patterns.
#[test]
fn test_parse_catch_expression_parameter() {
    // source: try {} catch (answer()) {}
    let test = TestParser::new("try {} catch (answer()) {}");
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    assert_node!(parser.tree, try_id, Expression::Try { catch: Some(catch), .. } => {
        assert_node!(parser.tree, *catch, Catch { pattern: Some(catch_pattern), ty: None, body } => {
        assert_node!(parser.tree, *catch_pattern, Pattern::NominalTuple { ty, fields } => {
            assert_expression_path!(parser, parser.tree.get(*ty), "answer");
            assert!(fields.is_empty());
        });
        assert_node!(parser.tree, *body, Expression::Block(..));
        });
    });
}

/// Parse catch literal parameters as expression patterns.
#[test]
fn test_parse_catch_literal_parameter() {
    // source: try {} catch (42) {}
    let test = TestParser::new("try {} catch (42) {}");
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    assert_node!(parser.tree, try_id, Expression::Try { catch: Some(catch), .. } => {
        assert_node!(parser.tree, *catch, Catch { pattern: Some(catch_pattern), ty: None, body } => {
        assert_node!(parser.tree, *catch_pattern, Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(..));
        });
        assert_node!(parser.tree, *body, Expression::Block(..));
        });
    });
}

/// Parse catch blocks separated from try by a newline.
#[test]
fn test_parse_catch_without_binding_after_newline() {
    let test = TestParser::new("try {\n  foo()\n}\ncatch {\n  bar()\n}");
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    assert_node!(parser.tree, try_id, Expression::Try { catch: Some(catch), finally: None, .. } => {
        assert_node!(parser.tree, *catch, Catch { pattern: None, ty: None, body } => {
        assert_node!(parser.tree, *body, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { .. } => {
                let expressions = block_expression_ids(parser.tree.get(*block_id));
                assert_eq!(expressions.len(), 1);
                assert_node!(parser.tree, expressions[0], Expression::Call { left, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "bar");
                });
            });
        });
        });
    });
}

/// Parse try/catch/finally with comment and newline breaks around keyword boundaries.
#[test]
fn test_parse_try_with_comment_newline_boundaries() {
    let test = TestParser::new(
        "try // Comment 1\n{\n}\ncatch(\n// Comment 2\ne\n) {\n}\nfinally // Comment 3\n{\n}\n",
    );
    let mut parser = test.prepare();

    let try_id = parser.parse_try().unwrap();
    assert_node!(parser.tree, try_id, Expression::Try { catch: Some(catch), finally: Some(finally), .. } => {
        assert_node!(parser.tree, *catch, Catch { pattern: Some(catch_pattern), ty: None, body } => {
        assert_node!(parser.tree, *catch_pattern, Pattern::Binding { name, pattern: None, .. } => {
            assert_string!(parser, *name, "e");
        });
        assert_node!(parser.tree, *body, Expression::Block(..));
        });
        assert_node!(parser.tree, *finally, Expression::Block(..));
    });
}
