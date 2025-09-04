use crate::{
    Block, Break, Continue, Defer, Keyword, NodeId, ParseError, ParseResult, Parser, Return,
    Statement,
};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
    /// Peek a block (test with and without label).
    pub fn peek_block(&self) -> ParseResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok()
            || self.peek_token(TokenType::Identifier).is_ok()
                && self.peek_next_token(TokenType::Colon).is_ok()
                && self.peek_next_next_token(TokenType::OpenBrace).is_ok()
        {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(self.peek()?.span))
        }
    }

    /// Eat a block (including the label, `{`, and `}`).
    pub fn eat_block(&mut self) -> ParseResult<NodeId<Block>> {
        let start = self.mark();
        // label
        let label = if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let label = self.eat_identifier()?;
            self.eat_colon()?;
            Some(label)
        } else {
            None
        };
        // body
        self.eat_token(TokenType::OpenBrace)?;
        let block_id = self.eat_block_body()?;
        let block = self.tree.get_mut(block_id);
        block.label = label;
        self.eat_token(TokenType::CloseBrace)?;
        self.tree.set_span(block_id, self.get_span_from(start));
        Ok(block_id)
    }

    /// Eat a block of statements (without the label, `{`, and `}`)
    pub fn eat_block_body(&mut self) -> ParseResult<NodeId<Block>> {
        let start = self.mark();
        let mut statements: Vec<NodeId<Statement>> = Vec::new();

        loop {
            // break if we're at the end of the block
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // consume any statement stops (semicolon or newline)
            else if self.peek_statement_stop().is_ok() {
                self.eat_statement_stop()?;
            }
            // keep eating statements
            else {
                let statement_id = self.eat_statement()?;
                statements.push(statement_id);
            }
        }

        let block = Block {
            label: None,
            statements,
        };
        let block_id = self.tree.allocate(block, self.get_span_from(start));
        Ok(block_id)
    }

    /// Eat a break statement.
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
        let label = if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let label = self.eat_identifier()?;
            self.eat_colon()?;
            Some(label)
        } else {
            None
        };
        // value (if not at a statement stop)
        let value_id = if self.peek_statement_stop().is_err() {
            let value_id = self.eat_expression()?;
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

    /// Eat a continue statement.
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
        let label = if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let label = self.eat_identifier()?;
            self.eat_colon()?;
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

    /// Eat a return statement.
    ///
    /// Examples:
    /// ```
    /// return
    /// return 17
    /// ```
    pub fn eat_return(&mut self) -> ParseResult<NodeId<Return>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Return)?;
        // value
        let value_id = if self.peek_statement_stop().is_err() {
            let value_id = self.eat_expression()?;
            Some(value_id)
        } else {
            None
        };
        // return
        let return_id = self
            .tree
            .allocate(Return { value: value_id }, self.get_span_from(start));
        Ok(return_id)
    }

    /// Eat a defer statement.
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
    /// defer :label {
    ///     someOtherFunction()
    /// }
    /// ```
    pub fn eat_defer(&mut self) -> ParseResult<NodeId<Defer>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Defer)?;
        // block
        if self.peek_block().is_ok() {
            let block_id = self.eat_block()?;
            let defer_id = self
                .tree
                .allocate(Defer::Block(block_id), self.get_span_from(start));
            Ok(defer_id)
        }
        // statement
        else {
            let expression_id = self.eat_expression()?;
            let defer_id = self
                .tree
                .allocate(Defer::Expression(expression_id), self.get_span_from(start));
            Ok(defer_id)
        }
    }
}

#[cfg(test)]
mod tests {
    // todo!: test block
}
