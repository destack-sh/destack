use tspp_dir::{Keyword, Token, TokenType};
use tspp_source::File;

use super::cursor::TokenCursor;

/// A disposable parser token cursor for lookahead.
pub(crate) struct TokenProbe<'source> {
    /// The source file covered by the cursor.
    file: &'source File,
    /// The authoritative parser token cursor.
    cursor: &'source TokenCursor,
    /// The parser-visible offset from the authoritative cursor.
    offset: usize,
    /// The current lookahead token.
    current: Token,
}

impl<'source> TokenProbe<'source> {
    /// Create a probe at the current parser token.
    pub(super) fn new(file: &'source File, cursor: &'source TokenCursor) -> Self {
        Self {
            file,
            cursor,
            offset: 0,
            current: cursor.peek(),
        }
    }

    /// Return the current probed token.
    #[inline(always)]
    pub(crate) const fn peek_token(&self) -> Token {
        self.current
    }

    /// Return the current probed token type.
    #[inline(always)]
    pub(crate) fn peek_token_type(&self) -> TokenType {
        self.current.ty()
    }

    /// Return the current probed token as a keyword.
    #[inline]
    pub(crate) fn peek_keyword(&self) -> Option<Keyword> {
        self.cursor.classify_keyword(self.file, self.current)
    }

    /// Return whether the current token is one identifier text.
    #[inline]
    pub(crate) fn peek_identifier_is(&self, expected: &str) -> bool {
        self.cursor.identifier_is(self.file, self.current, expected)
    }

    /// Advance to the next parser-visible token.
    #[inline(always)]
    pub(crate) fn bump(&mut self) {
        self.offset += 1;
        self.current = self.cursor.peek_token_at(self.offset);
    }
}
