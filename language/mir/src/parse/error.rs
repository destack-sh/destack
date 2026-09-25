use std::fmt;

use tspp_core::Blob;
use tspp_source::{
    Diagnostic, DiagnosticDefinition, DiagnosticLabel, DiagnosticTarget, FileId, Span,
};

use crate::source::{Token, TokenType};

/// All MIR parser diagnostic definitions.
const MIR_PARSE_DIAGNOSTICS: &[DiagnosticDefinition] = &[
    ParseErrorKind::Invalid.definition(),
    ParseErrorKind::UnexpectedToken.definition(),
    ParseErrorKind::UnexpectedEnd.definition(),
];

/// One class of MIR text parse failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ParseErrorKind {
    /// Structurally invalid MIR source.
    Invalid,
    /// One token rejected by its grammar position.
    UnexpectedToken,
    /// MIR source that ended before a required construct.
    UnexpectedEnd,
}

/// One MIR text parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The failure class.
    kind: ParseErrorKind,
    /// The exact failure message.
    message: String,
    /// The source byte position.
    position: usize,
    /// The covered source length when known.
    length: usize,
}

impl ParseErrorKind {
    /// Return the static diagnostic definition.
    const fn definition(self) -> DiagnosticDefinition {
        let (id, description) = match self {
            Self::Invalid => ("invalid-mir", "MIR source is structurally invalid."),
            Self::UnexpectedToken => (
                "unexpected-mir-token",
                "MIR source contains an unexpected token.",
            ),
            Self::UnexpectedEnd => (
                "unexpected-end-of-mir",
                "MIR source ended before a required construct.",
            ),
        };

        DiagnosticDefinition::error(id, description)
    }
}

impl ParseError {
    /// All MIR parser diagnostic definitions.
    pub const ALL: &'static [DiagnosticDefinition] = MIR_PARSE_DIAGNOSTICS;

    /// Create one invalid MIR error.
    pub(super) fn new(message: impl Into<String>, position: usize) -> Self {
        Self {
            kind: ParseErrorKind::Invalid,
            message: message.into(),
            position,
            length: 0,
        }
    }

    /// Create one invalid MIR error with a known source length.
    pub(super) fn with_length(message: impl Into<String>, position: usize, length: usize) -> Self {
        Self {
            kind: ParseErrorKind::Invalid,
            message: message.into(),
            position,
            length,
        }
    }

    /// Create an unexpected MIR token error.
    pub(super) fn unexpected(expected: &str, actual: TokenType, position: usize) -> Self {
        Self {
            kind: ParseErrorKind::UnexpectedToken,
            message: format!("expected {expected}, got {actual:?}"),
            position,
            length: 0,
        }
    }

    /// Create an unexpected error for one concrete MIR token.
    pub(super) fn unexpected_token(expected: &str, token: &Token) -> Self {
        Self {
            kind: ParseErrorKind::UnexpectedToken,
            message: format!("expected {expected}, got {:?}", token.ty),
            position: token.span.start as usize,
            length: token.span.len() as usize,
        }
    }

    /// Create an unexpected end of MIR source error.
    pub(super) fn unexpected_end(expected: &str, position: usize) -> Self {
        Self {
            kind: ParseErrorKind::UnexpectedEnd,
            message: format!("expected {expected}, got end of input"),
            position,
            length: 0,
        }
    }

    /// Create an invalid MIR construct error.
    pub(super) fn invalid(description: &str, position: usize) -> Self {
        Self::new(format!("invalid {description}"), position)
    }

    /// Create an invalid MIR construct error with a known source length.
    pub(super) fn invalid_with_length(description: &str, position: usize, length: usize) -> Self {
        Self::with_length(format!("invalid {description}"), position, length)
    }

    /// Return the source byte position of this error.
    pub(super) fn position(&self) -> usize {
        self.position
    }

    /// Convert this MIR parse error into one source diagnostic.
    pub(super) fn to_diagnostic(&self, blob: Blob, file_id: FileId) -> Diagnostic {
        let definition = self.kind.definition();
        let start = self.position as u32;
        let length = self.length as u32;
        let span = Span::at(file_id, start, length);
        let primary =
            DiagnosticLabel::message(blob, DiagnosticTarget::Span(span), self.message.clone());

        Diagnostic::error(definition.id, self.message.clone(), primary)
    }
}

impl fmt::Display for ParseError {
    /// Format one MIR parse error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "MIR parse error at {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// Result type for MIR parsing.
pub type ParseResult<T> = Result<T, ParseError>;
