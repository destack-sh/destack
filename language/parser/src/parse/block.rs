use dyst_ast::{
    Block, BlockFormat, Expression, Keyword, NodeId, NodeType, TokenType, YieldCardinality,
};

use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

impl<'a> Parser<'a> {
    /// Peek a block (with and without label). Optional `do` prefix for disambiguation.
    #[inline]
    pub fn peek_block(&self) -> ParseResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_keyword(Keyword::Do).is_ok()
                && self.peek_next_token(TokenType::OpenBrace).is_ok()
            || self.peek_token(TokenType::Identifier).is_ok()
                && self.peek_next_token(TokenType::Colon).is_ok()
                && self.peek_next_next_token(TokenType::OpenBrace).is_ok()
        {
            Ok(())
        } else {
            Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::OpenBrace,
            ))
        }
    }

    /// Peek a next block (with and without label). Optional `do` prefix for disambiguation.
    #[inline]
    pub fn peek_next_block(&self) -> ParseResult<()> {
        if self.peek_next_token(TokenType::OpenBrace).is_ok()
            || self.peek_next_keyword(Keyword::Do).is_ok()
                && self.peek_next_next_token(TokenType::OpenBrace).is_ok()
            || self.peek_next_token(TokenType::Identifier).is_ok()
                && self.peek_next_next_token(TokenType::Colon).is_ok()
                && self.peek_next_next_next_token(TokenType::OpenBrace).is_ok()
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
    pub fn eat_block(&mut self) -> ParseResult<NodeId<Block>> {
        let start = self.mark();

        // `do` prefix
        if self.peek_keyword(Keyword::Do).is_ok() {
            self.bump(); // eat keyword
        }

        // label
        let label = if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let label = self.eat_identifier().for_node_type(NodeType::Block)?;
            self.eat_colon()?;
            Some(label)
        } else {
            None
        };

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
                label: None,
                expressions,
            },
            self.get_span_from(start),
        );
        let block = self.tree.get_mut(block_id);
        block.label = label;
        self.tree.set_span(block_id, self.get_span_from(start));
        Ok(block_id)
    }

    /// Eat a block of expressions (without the label, `{`, and `}`)
    pub fn eat_block_body(&mut self, format: BlockFormat) -> ParseResult<Vec<NodeId<Expression>>> {
        let mut expressions: Vec<NodeId<Expression>> = Vec::new();

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

        Ok(expressions)
    }

    /// Eat a break expression.
    ///
    /// Examples:
    /// ```
    /// break
    /// break :label
    /// break :label 17
    /// break 15
    /// ```
    pub fn eat_break(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Break)?;
        // label
        let label = if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            Some(self.eat_identifier()?)
        } else {
            None
        };
        // value (if not at a expression stop)
        let value_id = if self.peek().is_ok() && self.peek_statement_stop().is_err() {
            let value_id = self.eat_expression()?;
            Some(value_id)
        } else {
            None
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
    /// continue :label
    /// ```
    pub fn eat_continue(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Continue)?;
        // label
        let label = if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            let label = self.eat_identifier()?;
            Some(label)
        } else {
            None
        };
        // continue
        let continue_id = self
            .tree
            .insert(Expression::Continue { label }, self.get_span_from(start));
        Ok(continue_id)
    }

    /// Eat a defer expression.
    ///
    /// Examples:
    /// ```
    /// defer someFunction()
    ///
    /// defer {
    ///     someFunction()
    ///     someOtherFunction()
    /// }
    ///
    /// defer label: {
    ///     someOtherFunction()
    /// }
    /// ```
    pub fn eat_defer(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Defer)?;

        // block
        if self.peek_block().is_ok() {
            let block_id = self.eat_block()?;
            let block_id = self
                .tree
                .insert(Expression::Block(block_id), self.get_span_from(start));
            let defer_id = self.tree.insert(
                Expression::Defer {
                    expression: Some(block_id),
                },
                self.get_span_from(start),
            );
            Ok(defer_id)
        }
        // expression
        else {
            let expression_id = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            let defer_id = self.tree.insert(
                Expression::Defer {
                    expression: Some(expression_id),
                },
                self.get_span_from(start),
            );
            Ok(defer_id)
        }
    }

    /// Eat an await expression.
    ///
    /// Examples:
    /// ```
    /// await someFunction()
    /// ```
    pub fn eat_await(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Await)?;

        // expression
        let expression_id = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_expression()
        })?;

        // await
        let await_id = self.tree.insert(
            Expression::Await {
                expression: expression_id,
            },
            self.get_span_from(start),
        );
        Ok(await_id)
    }

    /// Eat a yield expression.
    ///
    /// Examples:
    /// ```
    /// yield someValue
    /// yield* someIterator
    /// ```
    pub fn eat_yield(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Yield)?;

        // cardinality
        let cardinality = if self.peek_token(TokenType::Multiply).is_ok() {
            self.bump(); // eat *
            YieldCardinality::Generator
        } else {
            YieldCardinality::Scalar
        };

        // value
        let value_id = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_expression()
        })?;

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
    /// Examples:
    /// ```
    /// throw someError
    /// throw anyOldExpression()
    /// ```
    pub fn eat_throw(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Throw)?;
        // value
        let value_id = if self.peek().is_ok() && self.peek_statement_stop().is_err() {
            let value_id = self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression()
            })?;
            Some(value_id)
        } else {
            None
        };
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
    pub fn eat_return(&mut self) -> ParseResult<NodeId<Expression>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Return)?;
        // value
        let value_id = if self.peek().is_ok() && self.peek_statement_stop().is_err() {
            let value_id = self.eat_expression().for_node_type(NodeType::Expression)?;
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
    use dyst_ast::{Expression, ScalarLiteral, YieldCardinality};

    use crate::parse::tests::TestParser;
    use crate::{assert_expr_path, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_empty_block() {
        let mut test = TestParser::new("{}");
        let mut parser = test.prepare();
        let block_id = parser.eat_block().unwrap();
        let block = parser.tree.get(block_id);
        assert_eq!(block.label, None);
        assert!(block.expressions.is_empty());
    }

    #[test]
    fn test_parse_labeled_empty_block() {
        let mut test = TestParser::new("label: {}");
        let mut parser = test.prepare();
        let block_id = parser.eat_block().unwrap();
        let block = parser.tree.get(block_id);
        assert_string!(parser, block.label.unwrap(), "label");
        assert!(block.expressions.is_empty());
    }

    #[test]
    fn test_break_no_label_no_value() {
        let mut test = TestParser::new("break");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert!(label.is_none());
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
        assert_node!(parser.tree, break_id, Expression::Break { label, value } => {
            assert!(label.is_none());
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(15)));
        });
    }

    #[test]
    fn test_continue_no_label() {
        let mut test = TestParser::new("continue");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Expression::Continue { label } => {
            assert!(label.is_none());
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
    fn test_defer_expression() {
        let mut test = TestParser::new("defer someFunction()");
        let mut parser = test.prepare();
        let defer_id = parser.eat_defer().unwrap();
        assert_node!(parser.tree, defer_id, Expression::Defer { expression } => {
            assert_node!(parser.tree, expression.unwrap(), Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expr_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
            });
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
                assert_expr_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
            });
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
            assert_node!(parser.tree, *value, Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expr_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
            });
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
            assert_node!(parser.tree, *value, Expression::Call { position: _, left, static_arguments: None, dynamic_arguments } => {
                assert_expr_path!(parser, parser.tree.get(*left), "someFunction");
                assert!(dynamic_arguments.is_empty());
            });
        });
    }

    #[test]
    fn test_throw_expression_no_value() {
        let mut test = TestParser::new("throw");
        let mut parser = test.prepare();
        let throw_id = parser.eat_throw().unwrap();
        assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_throw_expression_with_value() {
        let mut test = TestParser::new("throw 17");
        let mut parser = test.prepare();
        let throw_id = parser.eat_throw().unwrap();
        assert_node!(parser.tree, throw_id, Expression::Throw { value } => {
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(17)));
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
