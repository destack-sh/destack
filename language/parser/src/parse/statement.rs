use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{Parser, ParserError, ParserResult};
use tspp_dir::{
    BlockContext, Expression, LocalNodeId, NodeType, Token, TokenType, YieldCardinality,
};

impl Parser {
    /// Return whether a restricted statement operand is absent.
    #[inline]
    fn peek_statement_operand_absent(&self) -> bool {
        self.peek_is_on_new_line()
            || matches!(
                self.peek_token_type(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            )
    }

    /// Return whether one token ends a bare statement label.
    #[inline]
    fn is_statement_label_end(token: Token) -> bool {
        token.is_on_new_line()
            || matches!(
                token.ty(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            )
    }

    /// Parse a break expression.
    ///
    /// Examples:
    /// ```tspp
    /// break
    /// break outer
    /// break found: value
    /// ```
    pub(crate) fn parse_break(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        self.bump();

        // break
        let (label, label_range, value) = if self.peek_statement_operand_absent() {
            (None, None, None)
        }
        // break label: value, break label, or break value
        else if self.peek_is(TokenType::Identifier) {
            let next = self.peek_next_token();

            // break label: value
            if !next.is_on_new_line() && next.is(TokenType::Colon) {
                let (label, label_range) = self.eat_identifier_with_range()?;
                self.bump();
                let value =
                    self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;

                (Some(label), Some(label_range), Some(value))
            }
            // break label
            else if Self::is_statement_label_end(next) {
                let (label, label_range) = self.eat_identifier_with_range()?;

                (Some(label), Some(label_range), None)
            }
            // break value
            else {
                let value =
                    self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;

                (None, None, Some(value))
            }
        }
        // break value
        else {
            let value =
                self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;

            (None, None, Some(value))
        };

        // record the label as the operation's main source range
        let expression =
            self.insert_node(Expression::Break { label, value }, self.range_since(&start));
        if let Some(label_range) = label_range {
            self.tree.set_main_range(expression, label_range);
        }

        Ok(expression)
    }

    /// Parse a continue expression.
    ///
    /// Examples:
    /// ```tspp
    /// continue
    /// continue outer
    /// ```
    pub(crate) fn parse_continue(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        self.bump();

        // continue
        let (label, label_range) = if self.peek_statement_operand_absent() {
            (None, None)
        }
        // continue label
        else if self.peek_is(TokenType::Identifier) {
            let next = self.peek_next_token();
            if !Self::is_statement_label_end(next) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let (label, label_range) = self.eat_identifier_with_range()?;
            (Some(label), Some(label_range))
        }
        // reject continue value
        else {
            return Err(ParserError::unexpected(self.peek_token_span()));
        };

        // record the label as the operation's main source range
        let expression = self.insert_node(Expression::Continue { label }, self.range_since(&start));
        if let Some(label_range) = label_range {
            self.tree.set_main_range(expression, label_range);
        }

        Ok(expression)
    }

    /// Parse an await expression.
    ///
    /// Examples:
    /// ```tspp
    /// await value
    /// await? value
    /// await! value
    /// ```
    pub(crate) fn parse_await(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        self.bump();

        // await?, await!, or await
        let previous_end = self.peek_previous_token_end();
        let is_adjacent = previous_end == self.peek_token().range().start;
        let is_maybe = is_adjacent && self.eat_token_if(TokenType::Maybe);
        let is_must = !is_maybe && is_adjacent && self.eat_token_if(TokenType::Not);

        // await[?!]? value
        let expression =
            self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;
        // select the parsed await operation
        let node = if is_maybe {
            Expression::AwaitMaybe { expression }
        } else if is_must {
            Expression::AwaitMust { expression }
        } else {
            Expression::Await { expression }
        };

        Ok(self.insert_node(node, self.range_since(&start)))
    }

    /// Parse a const evaluation expression.
    ///
    /// Examples:
    /// ```tspp
    /// const expression
    /// const { statements }
    /// ```
    pub(crate) fn parse_const_evaluation(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        self.bump();

        // const { body } or const expression
        let body = if self.peek_block() {
            let block = self.parse_block(BlockContext::Expression)?;
            self.insert_node(Expression::Block(block), self.tree.get_range(block))
        } else {
            self.parse_expression(ExpressionPosition::Block, ExpressionStop::default())?
        };

        Ok(self.insert_node(Expression::Const { body }, self.range_since(&start)))
    }

    /// Parse a yield expression.
    ///
    /// Examples:
    /// ```tspp
    /// yield
    /// yield value
    /// yield* values
    /// ```
    pub(crate) fn parse_yield(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        self.bump();

        // yield* or yield
        let cardinality =
            if !self.peek_statement_operand_absent() && self.eat_token_if(TokenType::Multiply) {
                YieldCardinality::Generator
            } else {
                YieldCardinality::Scalar
            };
        // classify an omitted operand before parsing its value
        let is_absent =
            self.peek_statement_operand_absent() || self.peek_expression_slot_boundary();
        let value = if is_absent && cardinality == YieldCardinality::Generator {
            Some(self.recover_missing_expression_here(NodeType::Expression))
        } else if is_absent {
            None
        } else {
            Some(self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?)
        };

        Ok(self.insert_node(
            Expression::Yield { cardinality, value },
            self.range_since(&start),
        ))
    }

    /// Parse a return expression.
    ///
    /// Examples:
    /// ```tspp
    /// return
    /// return value
    /// ```
    pub(crate) fn parse_return(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        self.bump();

        // return or return value
        let value = if self.peek_statement_operand_absent() {
            None
        } else {
            Some(self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?)
        };

        Ok(self.insert_node(Expression::Return { value }, self.range_since(&start)))
    }
}
