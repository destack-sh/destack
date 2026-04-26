use crate::parse::parser::ParserOptions;
use crate::{ParseError, ParseResult, Parser};
use destack_ast::{
    Block, BlockContext, BlockFormat, Expression, IfCondition, IfKind, Keyword, LocalNodeId,
    NodeType, TokenType,
};

impl Parser {
    /// Return parser contexts for `if` conditions.
    #[inline]
    fn if_condition_contexts(&self) -> (ParserOptions, ParserOptions) {
        let ambient_context = self.options.nested().with_before_block(true);
        let expression_context = self.options.nested();
        (ambient_context, expression_context)
    }

    /// Eat something as a block (if it's not a block expression OR an if, wrap in a block expression).
    fn eat_expression_as_block(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();

        // semicolon statement forms allow empty branches
        if !self.language.is_destack() && self.peek_is(TokenType::Semicolon) {
            self.bump();
            let block_id = self.insert_node(
                Block {
                    context: BlockContext::Statement,
                    format: BlockFormat::Implicit,
                    leading_expressions: vec![],
                    tail_expression: None,
                },
                self.get_span_from(&start),
            );
            let expression_id = self
                .tree
                .insert(Expression::Block(block_id), self.get_span_from(&start));
            return Ok(expression_id);
        }

        // parse one statement expression in statement mode
        let ambient_context = self.options.with_before_block(true);
        let expression_id = self.with_options(
            self.options.with_ambient_context(ambient_context),
            |parser| parser.eat_statement_expression(),
        )?;

        // semicolon statement forms reject declarations in single-statement contexts
        if !self.language.is_destack() && self.is_single_statement_declaration(expression_id) {
            return Err(ParseError::unexpected(self.tree.get_span(expression_id)));
        }

        // keep existing block-like expressions
        if matches!(
            self.tree.get(expression_id),
            Expression::Block { .. } | Expression::If { .. }
        ) {
            return Ok(expression_id);
        }

        // block-value mode keeps branch values as tail expressions
        if self.language.is_destack() {
            let block_id = self.insert_node(
                Block {
                    context: BlockContext::Expression,
                    format: BlockFormat::Implicit,
                    leading_expressions: vec![],
                    tail_expression: Some(expression_id),
                },
                self.get_span_from(&start),
            );
            let expression_id = self
                .tree
                .insert(Expression::Block(block_id), self.get_span_from(&start));

            return Ok(expression_id);
        }

        // semicolon statement forms keep branch statements in statement-position blocks
        let block_id = self.insert_node(
            Block {
                context: BlockContext::Statement,
                format: BlockFormat::Implicit,
                leading_expressions: vec![expression_id],
                tail_expression: None,
            },
            self.get_span_from(&start),
        );
        let expression_id = self
            .tree
            .insert(Expression::Block(block_id), self.get_span_from(&start));

        Ok(expression_id)
    }

    /// Eat an optional else expression for an if expression.
    fn eat_if_else_expression_maybe(&mut self) -> ParseResult<Option<LocalNodeId<Expression>>> {
        // save state so missing else can rewind cleanly
        let else_mark = self.mark();
        let else_tree_mark = self.tree.next_id();

        // semicolon statement forms consume optional separators before else
        if !self.language.is_destack() {
            self.eat_newlines_maybe()?;
            while self.peek_is(TokenType::Semicolon) {
                self.bump();
                self.eat_newlines_maybe()?;
            }
        } else {
            self.eat_newlines_maybe()?;
        }

        // no else: restore speculative state
        if !self.is_keyword(Keyword::Else) {
            self.restore(else_mark, else_tree_mark);
            return Ok(None);
        }

        // else keyword
        self.eat_keyword(Keyword::Else)?;
        self.eat_newlines_maybe()?;

        // else body
        let (ambient_context, expression_context) = self.statement_position_contexts();
        let else_expression_id = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_expression_as_block(),
        )?;

        Ok(Some(else_expression_id))
    }

    /// Parse an if / else expression.
    ///
    /// Examples:
    /// ```
    /// // ternary
    /// cond ? a : b
    ///
    /// // if
    /// if (x > 0) {
    ///     print("positive")
    /// }
    ///
    /// // if else
    /// if (x > 0) {
    ///     print("positive")
    /// } else {
    ///     print("not positive")
    /// }
    ///
    /// // if else if
    /// if (x > 0) {
    ///     print("positive")
    /// } else if (x == 0) {
    ///     print("zero")
    /// } else {
    ///     print("negative")
    /// }
    /// ```
    pub fn eat_if(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();

        // NOTE: ternary if is parsed in expression loop, not in eat_if

        // keyword
        self.eat_keyword(Keyword::If)?;

        // open parenthesis
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // condition
        let (ambient_context, expression_context) = self.if_condition_contexts();
        let condition: IfCondition = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| {
                if matches!(parser.peek_any_keyword().ok(), Some(Keyword::Let))
                    || parser.peek_mutability_is()
                {
                    parser.eat_let_kind().and_then(|(kind, mutability)| {
                        parser
                            .eat_declarator(true, true)
                            .map(|declarator| IfCondition::Let {
                                kind,
                                mutability,
                                declarator,
                            })
                    })
                } else {
                    parser
                        .eat_expression(parser.options)
                        .map(|condition| IfCondition::Expression { condition })
                }
            },
        )?;

        // close parenthesis
        self.eat_newlines_maybe()?;
        self.eat_close_token_or_recover_missing_with(
            TokenType::CloseParenthesis,
            NodeType::Expression,
            |parser, token_type| {
                Self::is_close_delimiter_boundary_token(token_type) || parser.is_block_start()
            },
        )?;
        self.eat_newlines_maybe()?;

        // then block
        let (ambient_context, expression_context) = self.statement_position_contexts();
        let then_expression_id = self.with_options(
            self.options
                .with_ambient_context(ambient_context)
                .with_expression_context(expression_context),
            |parser| parser.eat_expression_as_block(),
        )?;

        // semicolon statement forms allow a trailing then semicolon
        if !self.language.is_destack() && self.peek_is(TokenType::Semicolon) {
            self.bump();
        }

        // optional else branch
        let else_expression_id = self.eat_if_else_expression_maybe()?;

        // if ...
        let if_node = Expression::If {
            kind: IfKind::If,
            condition,
            then_expression: then_expression_id,
            else_expression: else_expression_id,
        };

        let if_id = self.insert_node(if_node, self.get_span_from(&start));
        Ok(if_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        BinaryOperator, Block, CommentKind, Declaration, Declarator, Expression,
        FunctionDeclaration, FunctionKind, IfCondition, LetKind, Mutability, Pattern, PatternField,
        ScalarLiteral,
    };
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_comment, assert_expression_path, assert_name, assert_node,
        block_expression_ids,
    };

    #[test]
    fn test_parse_if_basic() {
        let mut test = TestParser::new("if (true) {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
            // condition is boolean true
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // empty then block
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_with_empty_blocks() {
        let mut test = TestParser::new("if (false) {} else {}");
        let mut parser = test.prepare();

        // if (false) { } else { }
        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // false
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
            // { }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
            // { }
            assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_parenthesized_with_trivial_blocks() {
        let mut test = TestParser::new("if (cond) { a } else { b }");
        let mut parser = test.prepare();

        // if (cond) { a } else { b }
        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // cond
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_expression_path!(parser, parser.tree.get(condition_id), "cond");
            // { a }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    let then_statement_id = parser.unwrap_labelled_expression(expressions[0]);
                    assert_expression_path!(parser, parser.tree.get(then_statement_id), "a");
                });
            });
            // { b }
            assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    let else_statement_id = parser.unwrap_labelled_expression(expressions[0]);
                    assert_expression_path!(parser, parser.tree.get(else_statement_id), "b");
                });
            });
        });
    }

    #[test]
    fn test_parse_if_parenthesized_condition_keeps_inner_span() {
        let mut test = TestParser::new("if (cond) {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, .. } => {
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };

            assert_expression_path!(parser, parser.tree.get(condition_id), "cond");

            let condition_span = parser.tree.get_span(condition_id);
            let condition_text = &parser.file.text()
                [condition_span.start as usize..condition_span.end as usize];
            assert_eq!(condition_text, "cond");
        });
    }

    #[test]
    fn test_parse_if_empty_statement() {
        let mut test = TestParser::new_with_options("if (cond);", LanguageType::TypeScript);
        let mut parser = test.prepare();

        // if (cond);
        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
            // cond
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_expression_path!(parser, parser.tree.get(condition_id), "cond");
            // empty then block
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
        });
    }

    #[test]
    fn test_parse_if_with_nested_parenthesized_condition() {
        let mut test = TestParser::new(
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
        parser.eat_newline().unwrap();

        // if (cond) { if (cond) { a } else { b } }
        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // cond
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_expression_path!(parser, parser.tree.get(condition_id), "cond");
            // { if (cond) { a } else { b } }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                    // if (cond) { a } else { b }
                    let inner_if_id = parser.unwrap_labelled_expression(expressions[0]);
                    assert_node!(parser.tree, inner_if_id, Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
                        // cond
                        let inner_condition_id = match inner_condition {
                            IfCondition::Expression { condition } => *condition,
                            IfCondition::Let { .. } => panic!("expected expression condition"),
                        };
                        assert_expression_path!(parser, parser.tree.get(inner_condition_id), "cond");
                        // { a }
                        assert_node!(parser.tree, *inner_then, Expression::Block(inner_block_id) => {
                            assert_node!(parser.tree, *inner_block_id, Block { format: _, .. } => {
                                let expressions = block_expression_ids(parser.tree.get(*inner_block_id));
                                assert_eq!(expressions.len(), 1);
                                let inner_then_statement_id = parser.unwrap_labelled_expression(expressions[0]);
                                assert_expression_path!(parser, parser.tree.get(inner_then_statement_id), "a");
                            });
                        });
                        // { b }
                        assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(inner_block_id) => {
                            assert_node!(parser.tree, *inner_block_id, Block { format: _, .. } => {
                                let expressions = block_expression_ids(parser.tree.get(*inner_block_id));
                                assert_eq!(expressions.len(), 1);
                                let inner_else_statement_id = parser.unwrap_labelled_expression(expressions[0]);
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
        let mut test = TestParser::new("if (true) {} else if (false) {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // true
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // then block
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
            // nested else-if should be simple If
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, .. } => {
                let inner_condition_id = match inner_condition {
                    IfCondition::Expression { condition } => *condition,
                    IfCondition::Let { .. } => panic!("expected expression condition"),
                };
                assert_node!(parser.tree, inner_condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
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
        //  (this is disambiguated in a condition / guard clause, see ExpressionParserOptions)
        let mut test = TestParser::new(
            r"
if (x > y) {
    y
} else if (y == z) {
    x
}",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // if x > y
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::GreaterThan);
                // x
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                // y
                assert_expression_path!(parser, parser.tree.get(*right), "y");
            });
            // { y }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
            // else if y == z
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, .. } => {
                // y == z
                let inner_condition_id = match inner_condition {
                    IfCondition::Expression { condition } => *condition,
                    IfCondition::Let { .. } => panic!("expected expression condition"),
                };
                assert_node!(parser.tree, inner_condition_id, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::Equal);
                    // y
                    assert_expression_path!(parser, parser.tree.get(*left), "y");
                    // z
                    assert_expression_path!(parser, parser.tree.get(*right), "z");
                });
                // { x }
                assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                        let expressions = block_expression_ids(parser.tree.get(*block_id));
                        assert_eq!(expressions.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if_else_with_empty_blocks() {
        let mut test = TestParser::new("if (true) {} else if (false) {} else {}");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // if true
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            // { }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert!(expressions.is_empty());
                });
            });
            // else if (false) { } else { }
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
                // else if (false)
                let inner_condition_id = match inner_condition {
                    IfCondition::Expression { condition } => *condition,
                    IfCondition::Let { .. } => panic!("expected expression condition"),
                };
                assert_node!(parser.tree, inner_condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                // { }
                assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                        let expressions = block_expression_ids(parser.tree.get(*block_id));
                        assert!(expressions.is_empty());
                    });
                });
                // else { }
                assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                        let expressions = block_expression_ids(parser.tree.get(*block_id));
                        assert!(expressions.is_empty());
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_if_else_multiline() {
        let mut test = TestParser::new(
            r"
if (v < lo) { lo }
else if (v > hi) { hi }
else { v }
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // if (v < lo)
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                // v
                assert_expression_path!(parser, parser.tree.get(*left), "v");
                // lo
                assert_expression_path!(parser, parser.tree.get(*right), "lo");
            });
            // { lo }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
            // else if (v > hi) { hi } else { v }
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition: inner_condition, then_expression: inner_then, else_expression: inner_else, .. } => {
                // else if (v > hi)
                let inner_condition_id = match inner_condition {
                    IfCondition::Expression { condition } => *condition,
                    IfCondition::Let { .. } => panic!("expected expression condition"),
                };
                assert_node!(parser.tree, inner_condition_id, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    // v
                    assert_expression_path!(parser, parser.tree.get(*left), "v");
                    // hi
                    assert_expression_path!(parser, parser.tree.get(*right), "hi");
                });
                // { hi }
                assert_node!(parser.tree, *inner_then, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                        let expressions = block_expression_ids(parser.tree.get(*block_id));
                        assert_eq!(expressions.len(), 1);
                    });
                });
                // else { v }
                assert_node!(parser.tree, inner_else.unwrap(), Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                        let expressions = block_expression_ids(parser.tree.get(*block_id));
                        assert_eq!(expressions.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_if_with_expression_condition() {
        let mut test = TestParser::new(
            r###"
if (x > y) {
    print("positive")
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
            // condition is binary expression x > y
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Binary { left: _, operator: _, right: _ });
            // then block has one expression
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
        });
    }

    /// If the then expression is not a block, it should be coerced to a block.
    #[test]
    fn test_parse_if_else_if_coerce_to_block() {
        let mut test = TestParser::new(
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
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // if (x)
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_expression_path!(parser, parser.tree.get(condition_id), "x");
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                    let expressions = block_expression_ids(parser.tree.get(*block_id));
                    assert_eq!(expressions.len(), 1);
                });
            });
            // else if (y)
            assert_node!(parser.tree, else_expression.unwrap(), Expression::If { condition, then_expression, else_expression, .. } => {
                let condition_id = match condition {
                    IfCondition::Expression { condition } => *condition,
                    IfCondition::Let { .. } => panic!("expected expression condition"),
                };
                assert_expression_path!(parser, parser.tree.get(condition_id), "y");
                assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                        let expressions = block_expression_ids(parser.tree.get(*block_id));
                        assert_eq!(expressions.len(), 1);
                    });
                });
                // else z
                assert_node!(parser.tree, else_expression.unwrap(), Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { format: _, .. } => {
                        let expressions = block_expression_ids(parser.tree.get(*block_id));
                        assert_eq!(expressions.len(), 1);
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_if_else_with_typed_parenthesized_arrow_statement() {
        let mut test = TestParser::new_with_options(
            r###"
if (payments) res.status(200).json({ payments });
else
  (error: Error) =>
    res.status(404).json({
      message: "No Payments were found",
      error,
    });
"###,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_if().unwrap();
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
                            assert_eq!(signature.kind, FunctionKind::Lambda);
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
        let mut test = TestParser::new("if (let (x, _) = value) { x } else { 0 }");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            let declarator_id = match condition {
                IfCondition::Let {
                    kind,
                    mutability,
                    declarator,
                } => {
                    assert_eq!(*kind, LetKind::Let);
                    assert_eq!(*mutability, Mutability::Mutable);
                    *declarator
                }
                IfCondition::Expression { .. } => panic!("expected if let condition"),
            };
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
    fn test_parse_if_let_tagged_object_pattern() {
        let mut test = TestParser::new("if (let Point { x, y } = value) { x }");
        let mut parser = test.prepare();

        let if_id = parser.eat_if().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, .. } => {
            let declarator_id = match condition {
                IfCondition::Let {
                    kind,
                    mutability,
                    declarator,
                } => {
                    assert_eq!(*kind, LetKind::Let);
                    assert_eq!(*mutability, Mutability::Mutable);
                    *declarator
                }
                IfCondition::Expression { .. } => panic!("expected if let condition"),
            };
            assert_node!(parser.tree, declarator_id, Declarator { pattern, ty, value } => {
                assert!(ty.is_none());
                let value_id = value.expect("expected if let value");
                assert_expression_path!(parser, parser.tree.get(value_id), "value");
                assert_node!(parser.tree, *pattern, Pattern::TaggedObject { ty, fields } => {
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

    #[test]
    fn test_parse_if_head_trailing_comment_on_condition_owner() {
        let mut test = TestParser::new_with_options(
            "if (ready) // if-head\n    run()",
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

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::If { condition, .. } => {
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };

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

        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "if-head");
    }

    #[test]
    fn test_parse_if_else_boundary_comment_on_else_owner() {
        let mut test = TestParser::new_with_options(
            "if (ready) {\n  run()\n}\n// else-boundary\nelse {\n  stop()\n}\n",
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

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::If { else_expression, .. } => {
            let else_expression_id = else_expression.expect("expected else expression");
            let annotations = parser.tree.get_decorators(else_expression_id.id);
            assert!(annotations.is_empty());
        });

        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "else-boundary");
    }

    #[test]
    fn test_parse_if_else_after_then_semicolon_with_leading_boundary_comment() {
        let input = "if (foo) a = b;\n/* foo */ else foo.split;";
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::If { else_expression, .. } => {
            assert!(else_expression.is_some());
        });
    }

    #[test]
    fn test_parse_if_else_after_then_semicolon_with_trailing_boundary_comment() {
        let input = "if (foo) a = b;\nelse /* foo */ foo.split;";
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::If { else_expression, .. } => {
            assert!(else_expression.is_some());
        });
    }
}
