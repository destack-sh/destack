use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::Token;
use tspp_source::Span;

/// A Token with a Span.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TokenSpan {
    /// The Token.
    pub token: Token,
    /// The Span of the Token in its FileFile.
    pub span: Span,
}

impl TokenSpan {
    /// Create a token span from a source-file token.
    #[inline]
    pub fn new(token: Token, file: tspp_source::FileId) -> Self {
        Self {
            token,
            span: token.span(file),
        }
    }
}
