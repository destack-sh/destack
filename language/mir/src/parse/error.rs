//! MIR parse errors.

use std::fmt;

use super::token::TokenType;

/// Parse error for MIR text format.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// The error message.
    pub message: String,
    /// Position in the source where the error occurred.
    pub position: usize,
}

impl ParseError {
    /// Create a new parse error.
    pub fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
        }
    }

    /// Create an "unexpected token" error.
    pub fn unexpected(expected: &str, got: TokenType, position: usize) -> Self {
        Self::new(format!("expected {expected}, got {got:?}"), position)
    }

    /// Create an "unexpected end of input" error.
    pub fn unexpected_end(expected: &str, position: usize) -> Self {
        Self::new(format!("expected {expected}, got end of input"), position)
    }

    /// Create an "invalid" error.
    pub fn invalid(what: &str, position: usize) -> Self {
        Self::new(format!("invalid {what}"), position)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error at {}: {}", self.position, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Result type for MIR parsing.
pub type ParseResult<T> = Result<T, ParseError>;

