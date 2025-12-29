use destack_ast::{
    Block, BlockFormat, Expression, Keyword, LocalNodeId, NodeType, TokenType, YieldCardinality,
};

use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

impl Parser {
    /// Eat a block or a single statement wrapped in a block.
    pub fn eat_block_or_statement(&mut self) -> ParseResult<LocalNodeId<Block>> {
        // if it's a block, just eat it
        if self.peek_block().is_ok() {
            return self.eat_block();
        }

        let start = self.mark();

        // empty statement (just semicolon, e.g., `for (x of y);`)
        if self.peek_token(TokenType::Semicolon).is_ok() {
            self.bump();
            let block_id = self.tree.insert(
                Block {
                    format: BlockFormat::Implicit,
                    expressions: vec![],
                },
                self.get_span_from(start),
            );
            return Ok(block_id);
        }

        // otherwise, eat a single statement and wrap it in a block
        let expression_id = self.with_options(self.options.in_statement_position(), |parser| {
            parser.eat_expression()
        })?;

        // consume trailing semicolon if present (e.g., `do x; while (true)`)
        if self.peek_token(TokenType::Semicolon).is_ok() {
            self.bump();
        }

        // wrap in a block
        let block_id = self.tree.insert(
            Block {
                format: BlockFormat::Implicit,
                expressions: vec![expression_id],
            },
            self.get_span_from(start),
        );
        Ok(block_id)
    }

    /// Peek a block. Optional `do` prefix for disambiguation.
    #[inline]
    pub fn peek_block(&self) -> ParseResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_keyword(Keyword::Do).is_ok()
                && self.peek_next_token(TokenType::OpenBrace).is_ok()
        {
            Ok(())
        } else {
            Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::OpenBrace,
            ))
        }
    }

    /// Peek a next block. Optional `do` prefix for disambiguation.
    #[inline]
    pub fn peek_next_block(&self) -> ParseResult<()> {
        if self.peek_next_token(TokenType::OpenBrace).is_ok()
            || self.peek_next_keyword(Keyword::Do).is_ok()
                && self.peek_next_next_token(TokenType::OpenBrace).is_ok()
        {
            Ok(())
        } else {
            Err(ParseError::expected(
                self.peek_next().unwrap_or(&self.eof_token).span,
                TokenType::OpenBrace,
            ))
        }
    }

    /// Eat a block (including the label, `{`, and `}`). Optional `do` prefix for disambiguation.
    ///
    /// Examples:
    /// ```
    /// { ... }
    /// block: { ... }
    pub fn eat_block(&mut self) -> ParseResult<LocalNodeId<Block>> {
        let start = self.mark();

        // `do` prefix
        if self.peek_keyword(Keyword::Do).is_ok() {
            self.bump(); // eat keyword
        }

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Block)?;
        let expressions = self
            .eat_block_body(BlockFormat::Explicit)
            .for_node_type(NodeType::Block)?;
        self.eat_token(TokenType::CloseBrace)?;

        // block
        let block_id = self.tree.insert(
            Block {
                format: BlockFormat::Explicit,
                expressions,
            },
            self.get_span_from(start),
        );
        Ok(block_id)
    }

    /// Eat a block of expressions (without the label, `{`, and `}`).
    /// ASI rules apply such that expressions are automatically coerced into statements in relevant positions.
    pub fn eat_block_body(
        &mut self,
        format: BlockFormat,
    ) -> ParseResult<Vec<LocalNodeId<Expression>>> {
        // parse expressions
        let mut expressions: Vec<LocalNodeId<Expression>> = Vec::new();
        while self.peek().is_ok() {
            // break if we're at the end of the block
            if self.peek().is_err()
                || format == BlockFormat::Explicit && self.peek_token(TokenType::CloseBrace).is_ok()
                || self.peek_token(TokenType::End).is_ok()
            {
                break;
            }
            // consume any expression stops (semicolon or newline)
            else if self.peek_statement_stop().is_ok() {
                self.eat_statement_stop_with_newlines()
                    .for_node_type(NodeType::Expression)?;
            }
            // eat expressions
            else {
                let expression_id = self
                    .with_options(self.options.nested(), |parser| {
                        parser.try_eat_statement_expression()
                    })
                    .for_node_type(NodeType::Expression)?;
                expressions.push(expression_id);
            }
        }

        // wrap expressions
        let mut statements: Vec<LocalNodeId<Expression>> = Vec::new();
        let expression_count = expressions.len();
        for (i, expression_id) in expressions.into_iter().enumerate() {
            let expression = self.tree.get(expression_id);
            // keep existing statements
            if matches!(expression, Expression::Statement(_)) || expression.is_top_level_statement()
            {
                statements.push(expression_id);
            }
            // wrap other expressions in statements (except last)
            else if i < expression_count - 1 || format == BlockFormat::Implicit {
                let statement_id = self.tree.insert(
                    Expression::Statement(expression_id),
                    self.tree.get_span(expression_id),
                );
                statements.push(statement_id);
            }
            // keep as expression
            else {
                statements.push(expression_id);
            }
        }
        Ok(statements)
    }

    /// Try to eat a statement expression (return Expression::Error if error and recovery is possible).
    /// Wraps semicolon expressions in a Statement expression, otherwise just returns the expression.
    #[inline]
    pub fn try_eat_statement_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        match self.with_options(self.options.in_statement_position(), |parser| {
            parser.eat_expression()
        }) {
            Ok(expression_id) => {
                if self.peek_token(TokenType::Semicolon).is_ok() {
                    self.bump(); // eat semicolon
                    let expression_id = self.tree.insert(
                        Expression::Statement(expression_id),
                        self.get_span_from(start),
                    );
                    Ok(expression_id)
                } else {
                    Ok(expression_id)
                }
            }
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize);
                self.try_recover(start, TokenType::Newline, Some(err))?;
                let error_id = self
                    .tree
                    .insert(Expression::Error, self.get_span_from(start));
                Ok(error_id)
            }
        }
    }

    /// Eat a break expression.
    ///
    /// Examples:
    /// ```
    /// break
    /// break :label
    /// break label      // JS-style (no colon)
    /// break :label 15  // Destack extension: label + value
    /// ```
    pub fn eat_break(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Break)?;

        // label and value:
        // 1. `:identifier` → Destack style label, optionally followed by value
        // 2. `identifier` at statement stop → JS style label (no value)
        // 3. Otherwise → value expression (Destack extension, no label)
        let (label, value_id) = if self.peek_token(TokenType::Colon).is_ok() {
            // Destack style: break :label [value]
            self.bump(); // eat colon
            let label = Some(self.eat_identifier()?);
            let value_id = if self.peek().is_ok() && self.peek_statement_stop().is_err() {
                let value_id = self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_expression()
                })?;
                Some(value_id)
            } else {
                None
            };
            (label, value_id)
        } else if self.peek_token(TokenType::Identifier).is_ok()
            && self
                .peek_next_token_in(&[
                    TokenType::Newline,
                    TokenType::Semicolon,
                    TokenType::End,
                    TokenType::CloseBrace,
                ])
                .is_ok()
        {
            // JS style: break label (identifier followed by statement stop)
            let label = Some(self.eat_identifier()?);
            (label, None)
        } else if self.peek().is_ok() && self.peek_statement_stop().is_err() {
            // Destack extension: break value (no label)
            let value_id = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            (None, Some(value_id))
        } else {
            (None, None)
        };

        // break
        let break_id = self.tree.insert(
            Expression::Break {
                label,
                value: value_id,
            },
            self.get_span_from(start),
        );
        Ok(break_id)
    }

    /// Eat a continue expression.
    ///
    /// Examples:
    /// ```
    /// continue
    /// continue :label  // Destack style
    /// continue label   // JS style (no colon)
    /// ```
    pub fn eat_continue(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Continue)?;

        // label parsing:
        // 1. `:identifier` → Destack style label
        // 2. `identifier` at statement stop → JS style label
        let label = if self.peek_token(TokenType::Colon).is_ok() {
            // Destack style: continue :label
            self.bump(); // eat colon
            Some(self.eat_identifier()?)
        } else if self.peek_token(TokenType::Identifier).is_ok()
            && self
                .peek_next_token_in(&[
                    TokenType::Newline,
                    TokenType::Semicolon,
                    TokenType::End,
                    TokenType::CloseBrace,
                ])
                .is_ok()
        {
            // JS style: continue label
            Some(self.eat_identifier()?)
        } else {
            None
        };

        // continue
        let continue_id = self
            .tree
            .insert(Expression::Continue { label }, self.get_span_from(start));
        Ok(continue_id)
    }

    /// Eat an await expression.
    ///
    /// Examples:
    /// ```
    /// await someFunction()
    /// await? someFallibleAsync()
    /// ```
    pub fn eat_await(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Await)?;

        // check for await? (sugar for (await expr)?)
        let is_maybe = self.peek_token(TokenType::Maybe).is_ok();
        if is_maybe {
            self.bump(); // eat ?
        }

        // expression
        let expression_id = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_expression()
        })?;

        // await or await?
        let expression = if is_maybe {
            Expression::AwaitMaybe {
                expression: expression_id,
            }
        } else {
            Expression::Await {
                expression: expression_id,
            }
        };
        let await_id = self.tree.insert(expression, self.get_span_from(start));
        Ok(await_id)
    }

    /// Eat a comptime expression.
    ///
    /// Examples:
    /// ```
    /// comptime 1 + 2
    /// comptime factorial(10)
    /// comptime { generateLookupTable() }
    /// ```
    pub fn eat_comptime(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Comptime)?;

        // body expression
        self.eat_newlines_maybe()?;
        let body_id = self.with_options(
            self.options
                .not_in_position()
                .in_statement_position()
                .in_comptime(),
            |parser| parser.eat_expression(),
        )?;

        // comptime
        let comptime_id = self.tree.insert(
            Expression::Comptime { body: body_id },
            self.get_span_from(start),
        );
        Ok(comptime_id)
    }

    /// Eat a yield expression.
    ///
    /// Examples:
    /// ```
    /// yield
    /// yield someValue
    /// yield* someIterator
    /// yield *a  // same as yield* a (only if no newline after yield)
    /// ```
    pub fn eat_yield(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Yield)?;

        // check for restricted production: newline after yield triggers ASI
        // if there's a newline, don't look for `*` or value
        if self.peek_statement_stop().is_ok() {
            let yield_id = self.tree.insert(
                Expression::Yield {
                    cardinality: YieldCardinality::Scalar,
                    value: None,
                },
                self.get_span_from(start),
            );
            return Ok(yield_id);
        }

        // cardinality: `yield*` or `yield *` (space before *, but no newline)
        let cardinality = if self.peek_token(TokenType::Multiply).is_ok() {
            self.bump(); // eat *
            YieldCardinality::Generator
        } else {
            YieldCardinality::Scalar
        };

        // value (optional, like return/throw)
        // yield without value is valid JS: `function* a() { yield }`
        let value_id = if self.peek().is_ok() && self.peek_statement_stop().is_err() {
            let value_id = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            Some(value_id)
        } else {
            None
        };

        // yield
        let yield_id = self.tree.insert(
            Expression::Yield {
                cardinality,
                value: value_id,
            },
            self.get_span_from(start),
        );
        Ok(yield_id)
    }

    /// Eat a throw expression.
    ///
    /// `throw` is a restricted production: a newline after `throw` triggers ASI,
    /// but unlike `return`, `throw` REQUIRES an expression, so `throw;` is invalid.
    ///
    /// Examples:
    /// ```
    /// throw someError
    /// throw anyOldExpression()
    /// ```
    pub fn eat_throw(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Throw)?;

        // value
        if self.peek_statement_stop().is_ok() || self.peek().is_err() {
            return Err(ParseError::unexpected_for(
                self.get_span_from(start),
                NodeType::Expression,
            ));
        }
        let value_id = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_expression()
        })?;

        // throw
        let throw_id = self.tree.insert(
            Expression::Throw { value: value_id },
            self.get_span_from(start),
        );
        Ok(throw_id)
    }

    /// Eat a return expression.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 17
    /// ```
    pub fn eat_return(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Return)?;
        // value
        let value_id = if self.peek().is_ok() && self.peek_statement_stop().is_err() {
            let value_id = self
                .with_options(self.options.not_in_position(), |parser| {
                    parser.eat_expression()
                })
                .for_node_type(NodeType::Expression)?;
            Some(value_id)
        } else {
            None
        };
        // return
        let return_id = self.tree.insert(
            Expression::Return { value: value_id },
            self.get_span_from(start),
        );
        Ok(return_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{Expression, ScalarLiteral, YieldCardinality};
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_empty_block() {
        let mut test = TestParser::new("{}");
        let mut parser = test.prepare();
        let block_id = parser.eat_block().unwrap();
        let block = parser.tree.get(block_id);
        assert!(block.expressions.is_empty());
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
        let mut test = TestParser::new("break :label");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert_string!(parser, label.unwrap(), "label");
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_break_with_label_and_value() {
        let mut test = TestParser::new("break :label 17");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert_string!(parser, label.unwrap(), "label");
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(17)));
        });
    }

    #[test]
    fn test_break_with_value() {
        let mut test = TestParser::new("break 15");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label: None, value } => {
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(15)));
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
        let mut test = TestParser::new("continue :label");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
            assert_string!(parser, label.unwrap(), "label");
        });
    }

    #[test]
    fn test_break_js_style_label() {
        let mut test = TestParser::new("break foo;");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert_string!(parser, label.unwrap(), "foo");
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_break_js_style_label_newline() {
        let mut test = TestParser::new("break foo\n");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert_string!(parser, label.unwrap(), "foo");
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_continue_js_style_label() {
        let mut test = TestParser::new("continue foo;");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
            assert_string!(parser, label.unwrap(), "foo");
        });
    }

    #[test]
    fn test_continue_js_style_label_newline() {
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
            assert_node!(parser.tree, *expression, Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
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
            assert_node!(parser.tree, *expression, Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
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
            assert_node!(parser.tree, *body, Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "factorial");
                assert_eq!(dynamic_arguments.len(), 1);
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
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
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
    fn test_yield_expression_generator() {
        let mut test = TestParser::new("yield* someFunction()");
        let mut parser = test.prepare();
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Generator);
            // someFunction()
            assert_node!(parser.tree, value.unwrap(), Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
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
        let options = LanguageType::JavaScript;
        let mut test = TestParser::new_with_options("yield\n*a", options);
        let mut parser = test.prepare();

        // yield parses fine with ASI
        let yield_id = parser.eat_yield().unwrap();
        assert_node!(parser.tree, yield_id, Expression::Yield { cardinality, value } => {
            assert_eq!(*cardinality, YieldCardinality::Scalar);
            assert!(value.is_none()); // ASI applied, no value
        });

        // try to parse *a as next statement - should fail in JS mode
        // (because * is not valid as unary prefix in JS)
        let result = parser.eat_expression();
        assert!(
            result.is_err() || !parser.diagnostics.is_empty(),
            "*a should fail in JavaScript mode"
        );
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
}
