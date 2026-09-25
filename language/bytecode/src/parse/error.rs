use std::fmt;

use tspp_source::Span;

/// One bytecode text parse error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable failure message.
    pub message: String,
    /// Exact failing source span.
    pub span: Span,
}

impl ParseError {
    /// Create one parse error.
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for ParseError {
    /// Format this parse error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// Result of parsing bytecode text.
pub type ParseResult<T> = std::result::Result<T, ParseError>;
