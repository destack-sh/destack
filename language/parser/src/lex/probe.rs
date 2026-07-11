use destack_dir::{Keyword, Token, TokenType};
use destack_source::File;

use super::identifier::classify_keyword;

/// A disposable indexed token cursor for grammar classification.
pub(crate) struct TokenProbe<'source> {
    /// The source file covered by the tokens.
    file: &'source File,
    /// The ordinary semantic tokens.
    tokens: &'source [Token],
    /// The next ordinary token index.
    index: usize,
    /// The current probed semantic token.
    current: Token,
    /// The unconsumed suffix of one split compound token.
    split: Option<Token>,
}

impl<'source> TokenProbe<'source> {
    /// Create one probe at an indexed parser cursor.
    pub(crate) fn new(
        file: &'source File,
        tokens: &'source [Token],
        index: usize,
        current: Token,
        split: Option<Token>,
    ) -> Self {
        Self {
            file,
            tokens,
            index,
            current,
            split,
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
        // reject non-identifier tokens
        if !self.current.is(TokenType::Identifier) {
            return None;
        }

        // use the packed keyword classification when available
        if let Some(keyword) = self.current.classified_keyword() {
            return keyword;
        }

        // classify contextually produced identifiers from source text
        classify_keyword(self.file.span_str(self.current.span(self.file.id)))
    }

    /// Return whether the current token is one identifier text.
    #[inline]
    pub(crate) fn peek_identifier_is(&self, expected: &str) -> bool {
        self.current.is(TokenType::Identifier)
            && self.file.span_str(self.current.span(self.file.id)) == expected
    }

    /// Advance to the next ordinary semantic token.
    #[inline(always)]
    pub(crate) fn bump(&mut self) {
        if let Some(split) = self.split.take() {
            self.current = split;

            return;
        }

        self.current = self
            .tokens
            .get(self.index)
            .copied()
            .unwrap_or_else(|| Token::eof(self.file.len));
        self.index += usize::from(self.index < self.tokens.len());
    }
}
