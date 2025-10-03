use crate::parse::prelude::*;
use crate::{
    Block, BlockFormat, Break, Continue, Defer, Expression, Keyword, NodeId, NodeType, ParseError,
    ParseResult, Parser,
};
use dyst_token::TokenType;

impl<'a> Parser<'a> {
    /// Peek a block (test with and without label).
    ///
    /// Examples:
    /// ```
    /// { ... }
    /// label: { ... }
    /// ```
    #[inline]
    pub fn peek_block(&self) -> ParseResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok()
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

    /// Eat a block (including the label, `{`, and `}`).
    ///
    /// Examples:
    /// ```
    /// { ... }
    /// block: { ... }
    pub fn eat_block(&mut self) -> ParseResult<NodeId<Block>> {
        let start = self.mark();

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
        self.eat_token(TokenType::OpenBrace)?;
        let expressions = self
            .eat_block_body(BlockFormat::Explicit)
            .for_node_type(NodeType::Block)?;
        self.eat_token(TokenType::CloseBrace)?;

        // block
        let block_id = self.tree.allocate(
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

        loop {
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
                    .try_eat_expression_as_statement()
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
    pub fn eat_break(&mut self) -> ParseResult<NodeId<Break>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Break)?;
        // label
        let label = if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            Some(self.eat_identifier().for_node_type(NodeType::Break)?)
        } else {
            None
        };
        // value (if not at a expression stop)
        let value_id = if self.peek().is_ok() && self.peek_statement_stop().is_err() {
            let value_id = self.eat_expression().for_node_type(NodeType::Break)?;
            Some(value_id)
        } else {
            None
        };
        // break
        let break_id = self.tree.allocate(
            Break {
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
    pub fn eat_continue(&mut self) -> ParseResult<NodeId<Continue>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Continue)?;
        // label
        let label = if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            let label = self.eat_identifier().for_node_type(NodeType::Continue)?;
            Some(label)
        } else {
            None
        };
        // continue
        let continue_id = self
            .tree
            .allocate(Continue { label }, self.get_span_from(start));
        Ok(continue_id)
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
        let return_id = self.tree.allocate(
            Expression::Return { value: value_id },
            self.get_span_from(start),
        );
        Ok(return_id)
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
    ///
    /// defer catch e {
    ///     _ => someErrorHandler(e)
    /// }
    /// ```
    pub fn eat_defer(&mut self) -> ParseResult<NodeId<Defer>> {
        let start = self.mark();

        // keyword
        self.eat_keyword(Keyword::Defer)?;

        // catch
        if self.peek_keyword(Keyword::Catch).is_ok() {
            self.bump(); // eat keyword
            let match_id = self.eat_match_body().for_node_type(NodeType::Match)?;
            let defer_id = self
                .tree
                .allocate(Defer::Catch(match_id), self.get_span_from(start));
            Ok(defer_id)
        }
        // block
        else if self.peek_block().is_ok() {
            let block_id = self.eat_block().for_node_type(NodeType::Defer)?;
            let defer_id = self
                .tree
                .allocate(Defer::Block(block_id), self.get_span_from(start));
            Ok(defer_id)
        }
        // expression
        else {
            let expression_id = self.eat_expression().for_node_type(NodeType::Defer)?;
            let defer_id = self
                .tree
                .allocate(Defer::Expression(expression_id), self.get_span_from(start));
            Ok(defer_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Break, Continue, Defer, Expression, Match, assert_expr_path, assert_int, assert_node,
        assert_path, assert_string,
    };

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
        assert_string!(parser.session, block.label.unwrap(), "label");
        assert!(block.expressions.is_empty());
    }

    #[test]
    fn test_break_no_label_no_value() {
        let mut test = TestParser::new("break");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Break { label, value } => {
            assert!(label.is_none());
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_break_with_label() {
        let mut test = TestParser::new("break :label");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Break { label, value } => {
            assert_string!(parser.session, label.unwrap(), "label");
            assert!(value.is_none());
        });
    }

    #[test]
    fn test_break_with_label_and_value() {
        let mut test = TestParser::new("break :label 17");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Break { label, value } => {
            assert_string!(parser.session, label.unwrap(), "label");
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                assert_int!(parser.tree, *literal_id, 17);
            });
        });
    }

    #[test]
    fn test_break_with_value() {
        let mut test = TestParser::new("break 15");
        let mut parser = test.prepare();
        let break_id = parser.eat_break().unwrap();
        assert_node!(parser.tree, break_id, Break { label, value } => {
            assert!(label.is_none());
            assert!(value.is_some());
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                assert_int!(parser.tree, *literal_id, 15);
            });
        });
    }

    #[test]
    fn test_continue_no_label() {
        let mut test = TestParser::new("continue");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Continue { label } => {
            assert!(label.is_none());
        });
    }

    #[test]
    fn test_continue_with_label() {
        let mut test = TestParser::new("continue :label");
        let mut parser = test.prepare();
        let continue_id = parser.eat_continue().unwrap();
        assert_node!(parser.tree, continue_id, Continue { label } => {
            assert_string!(parser.session, label.unwrap(), "label");
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
            assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(literal_id) => {
                assert_int!(parser.tree, *literal_id, 42);
            });
        });
    }

    #[test]
    fn test_defer_expression() {
        let mut test = TestParser::new("defer someFunction()");
        let mut parser = test.prepare();
        let defer_id = parser.eat_defer().unwrap();
        assert_node!(parser.tree, defer_id, Defer::Expression(expr_id) => {
            assert_node!(parser.tree, *expr_id, Expression::Call(_));
        });
    }

    #[test]
    fn test_defer_block() {
        let mut test = TestParser::new("defer {}");
        let mut parser = test.prepare();
        let defer_id = parser.eat_defer().unwrap();
        assert_node!(parser.tree, defer_id, Defer::Block(block_id) => {
            let block = parser.tree.get(*block_id);
            assert!(block.expressions.is_empty());
        });
    }

    #[test]
    fn test_defer_catch() {
        let mut test = TestParser::new("defer catch e { _ => someErrorHandler(e) }");
        let mut parser = test.prepare();
        let defer_id = parser.eat_defer().unwrap();
        assert_node!(parser.tree, defer_id, Defer::Catch(match_id) => {
            assert_node!(parser.tree, *match_id, Match { value, .. } => {
                assert_expr_path!(parser.session, parser.tree.get(*value), "e");
            });
        });
    }
}
