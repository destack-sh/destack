use crate::{ParseError, ParseResult, Parser};
use destack_dir::{Keyword, Visibility};

impl Parser {
    /// Peek a visibility.
    #[inline]
    pub fn peek_visibility(&mut self) -> ParseResult<Option<Visibility>> {
        if let Some(visibility) = self.peek_visibility_is() {
            Ok(Some(visibility))
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Return the visibility keyword when present.
    #[inline]
    pub fn peek_visibility_is(&mut self) -> Option<Visibility> {
        if self.is_keyword(Keyword::Public) {
            Some(Visibility::Public)
        } else if self.is_keyword(Keyword::Protected) {
            Some(Visibility::Protected)
        } else if self.is_keyword(Keyword::Private) {
            Some(Visibility::Private)
        } else {
            None
        }
    }

    /// Eat a visibility maybe.
    #[inline]
    pub fn eat_visibility_maybe(&mut self) -> ParseResult<Option<Visibility>> {
        if self.peek_visibility_is().is_some() {
            Ok(Some(self.eat_visibility()?))
        } else {
            Ok(None)
        }
    }

    /// Eat a visibility.
    #[inline]
    pub fn eat_visibility(&mut self) -> ParseResult<Visibility> {
        if let Some(visibility) = self.peek_visibility_is() {
            self.bump(); // eat visibility
            Ok(visibility)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }
}
