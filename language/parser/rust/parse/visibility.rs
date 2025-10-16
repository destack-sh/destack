use dyst_ast::TokenType;

use crate::{Keyword, Parser, ParserError, ParserResult, Visibility};

// NOTE: we support parsing `#name` as alias for `private name` for #Leniency
// (only works in compatibility mode since #name is pre-parsed as a tag)

impl<'a> Parser<'a> {
    /// Peek a visibility.
    #[inline]
    pub fn peek_visibility(&self) -> ParserResult<Option<Visibility>> {
        if self.peek_keyword(Keyword::Public).is_ok() {
            Ok(Some(Visibility::Public))
        } else if self.peek_keyword(Keyword::Protected).is_ok() {
            Ok(Some(Visibility::Protected))
        } else if self.peek_keyword(Keyword::Private).is_ok()
            || self.peek_token(TokenType::Tag).is_ok()
        {
            Ok(Some(Visibility::Private))
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a visibility maybe.
    #[inline]
    pub fn eat_visibility_maybe(&mut self) -> ParserResult<Option<Visibility>> {
        if self.peek_visibility().is_ok() {
            Ok(Some(self.eat_visibility()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a visibility.
    #[inline]
    pub fn eat_visibility(&mut self) -> ParserResult<Visibility> {
        if let Some(visibility) = self.peek_visibility()? {
            self.bump(); // eat visibility
            Ok(visibility)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }
}
