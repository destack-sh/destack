use crate::{Block, Break, Continue, Defer, NodeId, ParseResult, Parser, Return, Statement};
use destack_language_token::TokenType;

impl<'a> Parser<'a> {
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
        self.tree.set_span(block_id, self.span_from(start));
        Ok(block_id)
    }

    /// Parse a block of statements (without the label, `{`, and `}`)
    pub fn eat_block_body(&mut self) -> ParseResult<NodeId<Block>> {
        let start = self.mark();
        let mut statements: Vec<NodeId<Statement>> = Vec::new();

        loop {
            let statement_id = self.eat_statement()?;
            statements.push(statement_id);
            if self.peek_statement_stop().is_ok() {
                self.eat_statement_stop()?;
            } else {
                break;
            }
        }

        let block = Block {
            label: None,
            statements,
        };
        let block_id = self.tree.allocate(block, self.span_from(start));
        Ok(block_id)
    }

    // Eat a break statement.
    pub fn eat_break(&mut self) -> ParseResult<NodeId<Break>> {
        todo!()
    }

    // Eat a continue statement.
    pub fn eat_continue(&mut self) -> ParseResult<NodeId<Continue>> {
        todo!()
    }

    // Eat a return statement.
    pub fn eat_return(&mut self) -> ParseResult<NodeId<Return>> {
        todo!()
    }

    // Eat a defer statement.
    pub fn eat_defer(&mut self) -> ParseResult<NodeId<Defer>> {
        todo!()
    }
}
