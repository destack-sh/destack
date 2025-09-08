use dyst_language_diagnostic::{Diagnostic, LabeledSpan, Severity};
use dyst_language_source::Span;

/// An error that can occur during parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    SyntaxError(Span),
    UnexpectedToken(Span),
}

/// A result of a parse operation.
pub type ParseResult<T> = Result<T, ParseError>;

// todo!: Diagnostics
impl From<ParseError> for Diagnostic {
    fn from(error: ParseError) -> Self {
        Diagnostic {
            id: 0,
            primary_span: match error {
                ParseError::SyntaxError(span) => Some(LabeledSpan {
                    span,
                    message: "syntax error".to_string(),
                }),
                ParseError::UnexpectedToken(span) => Some(LabeledSpan {
                    span,
                    message: "unexpected token".to_string(),
                }),
            },
            secondary_spans: None,
            suggestions: None,
            severity: Severity::Error,
            message: "parse error".to_string(),
        }
    }
}
