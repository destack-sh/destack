use dyst_language_diagnostic::{Diagnostic, DiagnosticKind, Severity};
use dyst_language_source::{LabeledSpan, Span};
use dyst_language_token::TokenType;

/// Error when parsing the AST.
#[must_use]
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct ParseError {
    pub span: Span,
    pub expected_token: Option<TokenType>,
}

impl ParseError {
    /// Create a ParseError for an unexpected token.
    pub(crate) fn unexpected(span: Span) -> Self {
        Self {
            span,
            expected_token: None,
        }
    }

    /// Create a ParseError for an expected token.
    pub(crate) fn expected_token(span: Span, expected_token: TokenType) -> Self {
        Self {
            span,
            expected_token: Some(expected_token),
        }
    }
}

/// The result of a parse operation.
pub type ParseResult<T> = Result<T, ParseError>;

impl From<ParseError> for Diagnostic {
    /// Convert to a Diagnostic.
    fn from(error: ParseError) -> Self {
        Diagnostic {
            id: "E001".to_string(),
            kind: DiagnosticKind::Parse,
            severity: Severity::Error,
            message: "parse error".to_string(),
            primary_span: Some(LabeledSpan {
                span: error.span,
                label: match error.expected_token {
                    Some(token_type) => format!("expected {token_type:?}"),
                    None => "unexpected".to_string(),
                },
            }),
            secondary_spans: None,
            suggestions: None,
        }
    }
}
