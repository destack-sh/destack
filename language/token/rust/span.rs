use crate::Token;

/// A position range in a `SourceFile` in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// The start position of the Span in bytes (absolute, inclusive).
    pub start: u32,
    /// The end position of the Span in bytes (absolute, exclusive).
    pub end: u32,
}

impl Span {
    /// Create a new Span.
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Create an empty Span.
    pub fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Merge two Spans.
    /// The resulting Span will be the smallest Span that contains both.
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Check if the Span is empty.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// A "semantic" Token with a Span.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TokenSpan {
    /// The Token.
    pub token: Token,
    /// The Span of the Token in its SourceFile.
    pub span: Span,
}
