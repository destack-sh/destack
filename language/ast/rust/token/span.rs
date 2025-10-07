use crate::Token;
use dyst_source::Span;

/// A Token with a Span.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenSpan {
    /// The Token.
    pub token: Token,
    /// The Span of the Token in its SourceFile.
    pub span: Span,
}
