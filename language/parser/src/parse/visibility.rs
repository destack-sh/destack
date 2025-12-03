use crate::{ParseError, ParseResult, Parser};
use destack_ast::{Keyword, TokenType, Visibility};

impl Parser {
    /// Peek a visibility.
    #[inline]
    pub fn peek_visibility(&self) -> ParseResult<Option<Visibility>> {
        if self.peek_keyword(Keyword::Public).is_ok() {
            Ok(Some(Visibility::Public))
        } else if self.peek_keyword(Keyword::Protected).is_ok() {
            Ok(Some(Visibility::Protected))
        } else if self.peek_keyword(Keyword::Private).is_ok()
            // `#field` for private fields (TypeScript syntax)
            || self.peek_token(TokenType::Hash).is_ok()
        {
            Ok(Some(Visibility::Private))
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a visibility maybe.
    #[inline]
    pub fn eat_visibility_maybe(&mut self) -> ParseResult<Option<Visibility>> {
        if self.peek_visibility().is_ok() {
            Ok(Some(self.eat_visibility()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a visibility.
    #[inline]
    pub fn eat_visibility(&mut self) -> ParseResult<Visibility> {
        if let Some(visibility) = self.peek_visibility()? {
            self.bump(); // eat visibility
            Ok(visibility)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }
}
