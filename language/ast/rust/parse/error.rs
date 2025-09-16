use core::fmt;

use dyst_language_diagnostic::{Diagnostic, DiagnosticKind, Severity};
use dyst_language_source::{LabeledSpan, Span};
use dyst_language_token::TokenType;

/// Error when parsing the AST.
#[derive(Debug, Clone)]
pub struct ParseError {
    span: Span,
    expected: Option<TokenType>,
    source: Option<Box<ParseError>>,
}

/// A result of a parse operation.
pub type ParseResult<T> = Result<T, ParseError>;

impl ParseError {
    /// Create a ParseError leaf without an expected alternative.
    #[inline]
    pub fn unexpected(span: Span) -> Self {
        ParseError {
            span,
            expected: None,
            source: None,
        }
    }

    /// Create a ParseError leaf with an expected alternative.
    #[inline]
    pub fn expected(span: Span, expected: TokenType) -> Self {
        ParseError {
            span,
            expected: Some(expected),
            source: None,
        }
    }

    /// Create a ParseError leaf from a source error with a new span.
    #[inline]
    pub fn from_source(span: Span, source: ParseError) -> Self {
        ParseError {
            span,
            expected: None,
            source: Some(Box::new(source)),
        }
    }

    /// Create a ParseError leaf from a source error with a new span.
    #[inline]
    pub fn from_source_maybe(span: Span, source: Option<ParseError>) -> Self {
        ParseError {
            span,
            expected: None,
            source: source.map(Box::new),
        }
    }

    /// Compare content.
    pub fn eq_content(&self, other: &Self) -> bool {
        let (self_span, self_expected) = self.leaf_content();
        let (other_span, other_expected) = other.leaf_content();
        self_span == other_span && self_expected == other_expected
    }

    /// Get the leaf error.
    pub fn leaf(&self) -> &ParseError {
        if let Some(source) = &self.source {
            source.leaf()
        } else {
            self
        }
    }

    /// Get the leaf content.
    pub fn leaf_content(&self) -> (Span, Option<TokenType>) {
        let leaf = self.leaf();
        (leaf.span, leaf.expected)
    }

    /// Get the leaf span.
    pub fn leaf_span(&self) -> Span {
        let leaf = self.leaf();
        leaf.span
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let span = self.span;
        match self.expected {
            Some(tok) => write!(f, "expected {tok:?} at {span:?}")?,
            None => write!(f, "unexpected token at {span:?}")?,
        }
        Ok(())
    }
}

// Optional: implement std::error::Error so callers can use `source()` if they like.
impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Some(source) = &self.source {
            Some(&**source)
        } else {
            None
        }
    }
}

impl From<&ParseError> for Diagnostic {
    fn from(error: &ParseError) -> Self {
        let (span, expected) = error.leaf_content();
        Diagnostic {
            kind: DiagnosticKind::Parse,
            code: "E001".to_string(),
            severity: Severity::Error,
            message: "parse error".to_string(),
            source: span.source,
            primary_span: LabeledSpan {
                span,
                label: match expected {
                    Some(token_type) => format!("expected {token_type:?}"),
                    None => "unexpected".to_string(),
                },
            },
            secondary_spans: None,
            suggestions: None,
        }
    }
}
