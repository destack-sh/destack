//! Parse statements.

use crate::{ParseResult, Parser, Statement};

impl<'a> Parser<'a> {
    /// Parse a statement (without the `;`).
    pub fn eat_statement_body(&mut self) -> ParseResult<Statement> {
        todo!()
    }

    /// Parse a statement (with the `;` or `\n`).
    pub fn eat_statement(&mut self) -> ParseResult<Statement> {
        todo!()
    }
}
