use serde::{Deserialize, Serialize};

use crate::Token;
use destack_source::Span;

/// A Token with a Span.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenSpan {
    /// The Token.
    pub token: Token,
    /// The Span of the Token in its FileFile.
    pub span: Span,
}

impl TokenSpan {
    /// Create a token span from a source-file token.
    #[inline]
    pub fn new(token: Token, file: destack_source::FileId) -> Self {
        Self {
            token,
            span: token.span(file),
        }
    }
}
