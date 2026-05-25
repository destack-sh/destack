use serde::{Deserialize, Serialize};

use crate::Token;
use destack_source::Span;

/// A token range in one known source file.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenRange {
    /// The token.
    pub token: Token,
    /// The start byte in the source file.
    pub start: u32,
}

impl TokenRange {
    /// Create a token range from a full token span.
    pub fn from_token_span(token_span: TokenSpan) -> Self {
        Self {
            token: token_span.token,
            start: token_span.span.start,
        }
    }

    /// Return the exclusive end byte in the source file.
    #[inline]
    pub fn end(self) -> u32 {
        self.start + self.token.len()
    }

    /// Return this token range as a full token span in one source file.
    #[inline]
    pub fn with_file(self, file: destack_source::FileId) -> TokenSpan {
        TokenSpan {
            token: self.token,
            span: Span::new(file, self.start, self.end()),
        }
    }
}

/// A Token with a Span.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenSpan {
    /// The Token.
    pub token: Token,
    /// The Span of the Token in its FileFile.
    pub span: Span,
}
