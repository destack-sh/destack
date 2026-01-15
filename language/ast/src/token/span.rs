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
