use crate::Token;

/// A position range in a `SourceFile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// The start position of the Span in bytes (absolute, inclusive).
    pub start: u32,
    /// The end position of the Span in bytes (absolute, exclusive).
    pub end: u32,
}

/// A "semantic" Token with a Span.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenSpan {
    /// The Token.
    pub token: Token,
    /// The Span of the Token in its SourceFile.
    pub span: Span,
}
