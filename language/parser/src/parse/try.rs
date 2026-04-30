use crate::{ParseResult, Parser};
use destack_ast::{BlockContext, Expression, Keyword, LocalNodeId, NodeType, TokenType};

impl Parser {
    /// Eat a try expression.
    ///
    /// Examples:
    /// ```
    /// try {
    ///     fileOperation()?;
    /// } catch (e) {
    ///     handle(e);
    /// }
    ///
    /// try {
    ///     let a = riskyOperationA()?;
    ///     riskyOperationB(a)?;
    /// } catch (e) {
    ///     log("failed", e);
    /// } finally {
    ///     cleanup();
    /// }
    ///
    /// try {
    ///     riskyOperationA()?;
    /// } catch match (e) {
    ///     NumericError(x) => Error(`bad number: ${x}`)
    ///     FormatError => Error(`bad format ${e}`)
    ///     _ => Error(`unknown error: ${e}`)
    /// }
    /// ```
    ///
    /// The parser accepts `try <expr>` without catch/finally, but Analyze rejects it.
    pub fn eat_try(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Try)?;

        // try block
        if self.is_block_start() {
            // try block
            let try_block = self.eat_block(BlockContext::Expression)?;
            let try_block_span = self.tree.get_span(try_block);
            let try_expression = self.insert_node(Expression::Block(try_block), try_block_span);

            // catch
            let (catch_pattern, catch_ty, catch_expression) = if self.is_keyword(Keyword::Catch) {
                self.bump(); // eat keyword

                // no pattern or catch match
                if self.is_block_start() || self.is_keyword(Keyword::Match) {
                    let catch_expression = self
                        .with_flags(self.flags.not_in_position(), |parser| {
                            parser.eat_statement_expression()
                        })?;
                    (None, None, Some(catch_expression))
                }
                // catch pattern with expression content
                else {
                    // parse catch binding pattern
                    self.eat_token(TokenType::OpenParenthesis)?;

                    let catch_pattern_flags = self
                        .flags
                        .not_in_position()
                        .in_before_type()
                        .in_before_block();
                    let catch_pattern =
                        self.with_flags(catch_pattern_flags, |parser| parser.eat_pattern())?;

                    let catch_ty = if self.peek_colon_is() {
                        self.bump(); // eat :
                        let catch_ty = self.eat_type_expression_node_or_recover_missing(
                            self.flags.not_in_position().in_type().in_before_block(),
                            NodeType::Pattern,
                        )?;
                        Some(catch_ty)
                    } else {
                        None
                    };

                    self.eat_close_token_or_recover_missing_with(
                        TokenType::CloseParenthesis,
                        NodeType::Pattern,
                        |parser, token_type| {
                            Self::is_close_delimiter_boundary_token(token_type)
                                || parser.is_block_start()
                                || parser.is_keyword(Keyword::Match)
                        },
                    )?;

                    let catch_expression = self
                        .with_flags(self.flags.not_in_position(), |parser| {
                            parser.eat_statement_expression()
                        })?;
                    (Some(catch_pattern), catch_ty, Some(catch_expression))
                }
            } else {
                (None, None, None)
            };

            // finally
            let finally_expression = if self.is_keyword(Keyword::Finally) {
                self.bump(); // eat keyword
                let finally_expression = self
                    .with_flags(self.flags.not_in_position(), |parser| {
                        parser.eat_statement_expression()
                    })?;
                Some(finally_expression)
            } else {
                None
            };

            // try
            let try_id = self.insert_node(
                Expression::Try {
                    try_expression,
                    catch_pattern,
                    catch_ty,
                    catch_expression,
                    finally_expression,
                },
                self.get_span_from(&start),
            );
            Ok(try_id)
        }
        // try expression
        else {
            let expression_flags = self.flags.not_in_position();
            let expression_id = self.with_flags(expression_flags, |parser| {
                parser.eat_expression(parser.flags)
            })?;
            let try_id = self.insert_node(
                Expression::Try {
                    try_expression: expression_id,
                    catch_pattern: None,
                    catch_ty: None,
                    catch_expression: None,
                    finally_expression: None,
                },
                self.get_span_from(&start),
            );
            Ok(try_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{Block, Expression, Name, Pattern, PatternField, TypeExpression};
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_expression_path, assert_node, assert_string, block_expression_ids,
    };

    #[test]
    fn test_try_expression() {
        let mut test = TestParser::new(
            r###"
try foo()
"###,
        );
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { try_expression, catch_pattern: None, catch_ty: None, catch_expression: None, finally_expression: None } => {
            assert_node!(parser.tree, *try_expression, Expression::Call { position: _, left, generic_arguments: _, arguments: _ } => {
                assert_expression_path!(parser, parser.tree.get(*left), "foo");
            });
        });
    }

    #[test]
    fn test_try_expression_without_catch_or_finally() {
        let mut test = TestParser::new(
            r###"
try {
    foo()
}
"###,
        );
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { try_expression, catch_pattern: None, catch_ty: None, catch_expression: None, finally_expression: None } => {
            assert_node!(parser.tree, *try_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    let call_id = parser.unwrap_labelled_expression(expressions[0]);
                    assert_node!(parser.tree, call_id, Expression::Call { position: _, left, generic_arguments: _, arguments: _ } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    });
                });
            });
        });
    }

    #[test]
    fn test_try_block_wrapper_keeps_block_span() {
        let mut test = TestParser::new(
            r###"
try /* comment */ {
    foo()
}
"###,
        );
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { try_expression, .. } => {
            let try_expression_span = parser.tree.get_span(*try_expression);
            assert_eq!(parser.get_span_str(try_expression_span), "{\n    foo()\n}");
        });
    }

    #[test]
    fn test_try_expression_with_catch() {
        let mut test = TestParser::new(
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

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { try_expression, catch_pattern: Some(catch_pattern), catch_ty: None, catch_expression: Some(catch_expression), finally_expression: Some(finally_expression) } => {
            // try
            assert_node!(parser.tree, *try_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    let try_call_id = parser.unwrap_labelled_expression(expressions[0]);
                    assert_node!(parser.tree, try_call_id, Expression::Call { position: _, left, generic_arguments: _, arguments: _ } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "foo");
                    });
                });
            });
            // catch (e)
            assert_node!(parser.tree, *catch_pattern, Pattern::Binding { mutability: _, name, pattern: _ } => {
                assert_string!(parser, *name, "e");
            });
            // catch expression
            assert_node!(parser.tree, *catch_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    let catch_call_id = parser.unwrap_labelled_expression(expressions[0]);
                    assert_node!(parser.tree, catch_call_id, Expression::Call { position: _, left, generic_arguments: _, arguments: _ } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "bar");
                    });
                });
            });
            // finally expression
            assert_node!(parser.tree, *finally_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    let finally_call_id = parser.unwrap_labelled_expression(expressions[0]);
                    assert_node!(parser.tree, finally_call_id, Expression::Call { position: _, left, generic_arguments: _, arguments: _ } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "baz");
                    });
                });
            });
        });
    }

    #[test]
    fn test_try_expression_with_typed_catch_pattern() {
        let mut test = TestParser::new_with_language(
            r###"
try {
    foo()
} catch (ex: Error) {
    bar()
}
"###,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { catch_pattern: Some(catch_pattern), catch_ty: Some(catch_ty), .. } => {
            // catch binding
            assert_node!(parser.tree, *catch_pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "ex");
            });
            // catch type
            assert_expression_path!(parser, parser.tree.get(*catch_ty), "Error");
        });
    }

    /// Parse explicit branch semicolons as statements inside try branches.
    #[test]
    fn test_parse_try_keeps_explicit_branch_semicolons_as_statements() {
        let mut test = TestParser::new(
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

        let try_id = parser.eat_try().unwrap();

        assert_node!(parser.tree, try_id, Expression::Try { try_expression, catch_expression: Some(catch_expression), finally_expression: Some(finally_expression), .. } => {
            assert_node!(parser.tree, *try_expression, Expression::Block(try_block_id) => {
                let try_block = parser.tree.get(*try_block_id);
                assert_eq!(try_block.leading_expressions.len(), 1);
                assert!(try_block.tail_expression.is_none());
            });

            assert_node!(parser.tree, *catch_expression, Expression::Block(catch_block_id) => {
                let catch_block = parser.tree.get(*catch_block_id);
                assert_eq!(catch_block.leading_expressions.len(), 1);
                assert!(catch_block.tail_expression.is_none());
            });

            assert_node!(parser.tree, *finally_expression, Expression::Block(finally_block_id) => {
                let finally_block = parser.tree.get(*finally_block_id);
                assert_eq!(finally_block.leading_expressions.len(), 1);
                assert!(finally_block.tail_expression.is_none());
            });
        });
    }

    /// Recover a missing catch close parenthesis in place.
    #[test]
    fn test_try_expression_with_missing_catch_close_parenthesis() {
        let mut test = TestParser::new_with_language(
            r###"
try {
    foo()
} catch (ex: Error {
    bar()
}
"###,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, try_id, Expression::Try { catch_pattern: Some(catch_pattern), catch_ty: Some(catch_ty), catch_expression: Some(catch_expression), .. } => {
            assert_node!(parser.tree, *catch_pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "ex");
            });
            assert_expression_path!(parser, parser.tree.get(*catch_ty), "Error");
            assert_node!(parser.tree, *catch_expression, Expression::Block(..));
        });
    }

    /// Parse typed destructuring catch patterns.
    #[test]
    fn test_try_expression_with_typed_destructuring_catch_pattern() {
        let mut test = TestParser::new_with_language(
            r###"
try {
    foo()
} catch ({ name, message }: any) {
    bar()
}
"###,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { catch_pattern: Some(catch_pattern), catch_ty: Some(catch_ty), catch_expression: Some(catch_expression), .. } => {
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
            assert_node!(parser.tree, *catch_ty, TypeExpression::Literal { value } => {
                assert_eq!(*value, destack_ast::TypeLiteral::Any);
            });

            // catch body
            assert_node!(parser.tree, *catch_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    assert_node!(parser.tree, expressions[0], Expression::Call { left, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "bar");
                    });
                });
            });
        });
    }

    /// Parse untyped catch expression parameters as expression patterns.
    #[test]
    fn test_parse_untyped_catch_expression_parameter() {
        // source: try {} catch (answer()) {}
        let mut test =
            TestParser::new_with_language("try {} catch (answer()) {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { catch_pattern: Some(catch_pattern), catch_ty: None, catch_expression: Some(catch_expression), .. } => {
            assert_node!(parser.tree, *catch_pattern, Pattern::TaggedTuple { ty, fields } => {
                assert_expression_path!(parser, parser.tree.get(*ty), "answer");
                assert!(fields.is_empty());
            });
            assert_node!(parser.tree, *catch_expression, Expression::Block(..));
        });
    }

    /// Parse untyped catch literal parameters as expression patterns.
    #[test]
    fn test_parse_untyped_catch_literal_parameter() {
        // source: try {} catch (42) {}
        let mut test =
            TestParser::new_with_language("try {} catch (42) {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { catch_pattern: Some(catch_pattern), catch_ty: None, catch_expression: Some(catch_expression), .. } => {
            assert_node!(parser.tree, *catch_pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(..));
            });
            assert_node!(parser.tree, *catch_expression, Expression::Block(..));
        });
    }

    /// Parse untyped catch blocks separated from try by a newline.
    #[test]
    fn test_parse_untyped_catch_without_binding_after_newline() {
        let mut test = TestParser::new_with_language(
            "try {\n  foo()\n}\ncatch {\n  bar()\n}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { catch_pattern: None, catch_ty: None, catch_expression: Some(catch_expression), finally_expression: None, .. } => {
            assert_node!(parser.tree, *catch_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    assert_node!(parser.tree, expressions[0], Expression::Call { left, .. } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "bar");
                    });
                });
            });
        });
    }

    /// Parse untyped try/catch/finally with comment and newline breaks around keyword boundaries.
    #[test]
    fn test_parse_untyped_try_with_comment_newline_boundaries() {
        let mut test = TestParser::new_with_language(
            "try // Comment 1\n{\n}\ncatch(\n// Comment 2\ne\n) {\n}\nfinally // Comment 3\n{\n}\n",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let try_id = parser.eat_try().unwrap();
        assert_node!(parser.tree, try_id, Expression::Try { catch_pattern: Some(catch_pattern), catch_ty: None, catch_expression: Some(catch_expression), finally_expression: Some(finally_expression), .. } => {
            assert_node!(parser.tree, *catch_pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "e");
            });
            assert_node!(parser.tree, *catch_expression, Expression::Block(..));
            assert_node!(parser.tree, *finally_expression, Expression::Block(..));
        });
    }
}
