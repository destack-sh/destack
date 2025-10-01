use crate::{Expression, NodeId, ParseResult, Parser};
use dyst_token::TokenType;

impl<'a> Parser<'a> {
    /// Eat super types maybe.
    ///
    /// Examples:
    /// ```
    /// : Foo
    /// : Foo, Bar
    /// ```
    pub fn eat_super_types_maybe(&mut self) -> ParseResult<Option<Vec<NodeId<Expression>>>> {
        if self.peek_token(TokenType::Colon).is_ok() {
            self.bump(); // eat colon
            let super_types = self.eat_super_types()?;
            return Ok(Some(super_types));
        }
        Ok(None)
    }

    /// Eat super types.
    ///
    /// Examples:
    /// ```
    /// Foo
    /// Foo, Bar<X>
    /// ```
    pub fn eat_super_types(&mut self) -> ParseResult<Vec<NodeId<Expression>>> {
        let mut super_types: Vec<NodeId<Expression>> = Vec::new();
        loop {
            // eat until open parenthesis
            if self.peek_token(TokenType::OpenBrace).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
            // keep eating super types
            else {
                let super_type = self.with_options(self.options.in_before_block(), |parser| {
                    parser.eat_expression()
                })?;
                super_types.push(super_type);
            }
        }
        Ok(super_types)
    }
}
