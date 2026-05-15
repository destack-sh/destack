use crate::source::TokenType;
use std::fmt;

use destack_source::{Diagnostic, DiagnosticLabel, FileContentId, FileId, Span};

use crate::source::Token;

const MIR_PARSE_DIAGNOSTIC_CODE: &str = "EMIRP001";

/// Parse error for MIR text format.
#[derive(Debug, Clone)]
pub struct ParseError {
    /// The error message.
    pub message: String,
    /// Position in the source where the error occurred.
    pub position: usize,
    /// The source length covered by this error when known.
    pub length: usize,
}

impl ParseError {
    /// Create a new parse error.
    pub fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            message: message.into(),
            position,
            length: 0,
        }
    }

    /// Create a new parse error with one known source length.
    pub fn new_with_length(message: impl Into<String>, position: usize, length: usize) -> Self {
        Self {
            message: message.into(),
            position,
            length,
        }
    }

    /// Create an "unexpected token" error.
    pub fn unexpected(expected: &str, got: TokenType, position: usize) -> Self {
        Self::new(format!("expected {expected}, got {got:?}"), position)
    }

    /// Create an "unexpected token" error for one concrete token.
    pub fn unexpected_token(expected: &str, token: &Token) -> Self {
        Self::new_with_length(
            format!("expected {expected}, got {:?}", token.ty),
            token.span.start as usize,
            token.span.end.saturating_sub(token.span.start) as usize,
        )
    }

    /// Create an "unexpected end of input" error.
    pub fn unexpected_end(expected: &str, position: usize) -> Self {
        Self::new(format!("expected {expected}, got end of input"), position)
    }

    /// Create an "invalid" error.
    pub fn invalid(what: &str, position: usize) -> Self {
        Self::new(format!("invalid {what}"), position)
    }

    /// Create an "invalid" error for one known source span.
    pub fn invalid_at_span(what: &str, position: usize, length: usize) -> Self {
        Self::new_with_length(format!("invalid {what}"), position, length)
    }

    /// Convert this parse error into one shared source diagnostic.
    pub fn to_diagnostic(&self, content: FileContentId, file_id: FileId) -> Diagnostic {
        let start = u32::try_from(self.position).unwrap_or(u32::MAX);
        let length = u32::try_from(self.length).unwrap_or(u32::MAX);
        let span = Span::at(file_id, start, length);
        let label = self.message.clone();

        Diagnostic::error(
            MIR_PARSE_DIAGNOSTIC_CODE,
            format!("parse error: {}", self.message),
            DiagnosticLabel::message(content, span, label),
        )
    }

    /// Rebuild one parse error from a shared diagnostic.
    pub fn from_diagnostic(diagnostic: &Diagnostic) -> Self {
        let primary = diagnostic.primary_label();

        Self::new_with_length(
            primary.message.clone().unwrap_or_default(),
            primary.span.start as usize,
            primary.span.end.saturating_sub(primary.span.start) as usize,
        )
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
