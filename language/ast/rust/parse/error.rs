use dyst_language_diagnostic::{Diagnostic, DiagnosticKind, Severity};
use dyst_language_source::{LabeledSpan, Span};
use dyst_language_token::TokenType;

/// Error when parsing the AST.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    span: Span,
    expected_token: Option<TokenType>,
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

impl ParseError {
    /// Convert to a Diagnostic.
    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic {
            id: "E001".to_string(),
            kind: DiagnosticKind::Parse,
            severity: Severity::Error,
            message: "parse error".to_string(),
            primary_span: Some(LabeledSpan {
                span: self.span,
                label: match self.expected_token {
                    Some(token_type) => format!("expected {token_type:?}"),
                    None => "unexpected".to_string(),
                },
            }),
            secondary_spans: None,
            suggestions: None,
        }
    }
}
