use core::fmt;

use destack_dir::{NodeType, TokenSpan, TokenType};
use destack_source::{ContentId, Diagnostic, DiagnosticLabel, Span};
use std::error::Error;

/// One structural parser error used for recovery and diagnostics.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub struct ParserError {
    /// The span of the error.
    pub span: Span,
    /// The actual token type at the error span.
    pub actual: Option<TokenType>,
    /// The expected token type.
    pub expected: Option<TokenType>,
    /// The node type we tried to parse.
    pub node_type: Option<NodeType>,
}

/// The source location of one parser error.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ParserErrorLocation {
    /// The source span of the error.
    span: Span,
    /// The actual token type at the error span.
    actual: Option<TokenType>,
}

impl From<Span> for ParserErrorLocation {
    /// Create a parser error location from a source span.
    #[inline]
    fn from(span: Span) -> Self {
        Self { span, actual: None }
    }
}

impl From<TokenSpan> for ParserErrorLocation {
    /// Create a parser error location from a token span.
    #[inline]
    fn from(token: TokenSpan) -> Self {
        Self {
            span: token.span,
            actual: Some(token.token.ty()),
        }
    }
}

/// One parser operation result.
pub type ParserResult<T> = Result<T, ParserError>;

/// Extension methods for parser operation results.
pub trait ParserResultExt<T> {
    /// Set the node type of the error.
    fn for_node_type(self, node_type: NodeType) -> Result<T, ParserError>;
}

impl<T> ParserResultExt<T> for Result<T, ParserError> {
    /// Set the node type of the error if not already set.
    #[inline]
    fn for_node_type(self, node_type: NodeType) -> Self {
        match self {
            Err(error) if error.node_type.is_none() => Err(error.for_node_type(node_type)),
            result => result,
        }
    }
}

impl ParserError {
    /// Create a parser error for an unexpected location.
    #[inline]
    pub fn unexpected(location: impl Into<ParserErrorLocation>) -> Self {
        let location = location.into();

        Self {
            span: location.span,
            actual: location.actual,
            expected: None,
            node_type: None,
        }
    }

    /// Create a parser error for an unexpected location and node type.
    #[inline]
    pub fn unexpected_for(location: impl Into<ParserErrorLocation>, node_type: NodeType) -> Self {
        let location = location.into();

        Self {
            span: location.span,
            actual: location.actual,
            expected: None,
            node_type: Some(node_type),
        }
    }

    /// Create a parser error with an expected alternative at a location.
    #[inline]
    pub fn expected(location: impl Into<ParserErrorLocation>, expected: TokenType) -> Self {
        let location = location.into();

        Self {
            span: location.span,
            actual: location.actual,
            expected: Some(expected),
            node_type: None,
        }
    }

    /// Create a parser error with an expected alternative and node type.
    #[inline]
    pub fn expected_for(
        location: impl Into<ParserErrorLocation>,
        expected: TokenType,
        node_type: NodeType,
    ) -> Self {
        let location = location.into();

        Self {
            span: location.span,
            actual: location.actual,
            expected: Some(expected),
            node_type: Some(node_type),
        }
    }

    /// Set the node type of the error.
    #[inline]
    pub fn for_node_type(mut self, node_type: NodeType) -> Self {
        self.node_type = Some(node_type);
        self
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expected {
            Some(token_type) => write!(f, "expected {token_type} at {:?}", self.span)?,
            None => match self.actual {
                Some(token_type) => write!(f, "unexpected {token_type} at {:?}", self.span)?,
                None => write!(f, "unexpected syntax at {:?}", self.span)?,
            },
        }

        Ok(())
    }
}

impl Error for ParserError {}

impl ParserError {
    /// Convert this parse error into one source diagnostic.
    pub fn to_diagnostic(&self, content: ContentId) -> Diagnostic {
        let in_node_str = match self.node_type {
            Some(node_type) => format!(" in {node_type:?}"),
            None => "".to_string(),
        };

        let (message, label) = match self.expected {
            Some(token_type) => (
                format!("parse error: expected {token_type}{in_node_str}"),
                format!("expected {token_type}{in_node_str}"),
            ),
            None => {
                let actual_str = match self.actual {
                    Some(actual) => actual.to_string(),
                    None => "syntax".to_string(),
                };

                (
                    format!("parse error: unexpected {actual_str}{in_node_str}"),
                    format!("unexpected {actual_str}{in_node_str}"),
                )
            }
        };
        let primary = DiagnosticLabel::message(content, self.span, label);

        Diagnostic::error("EP001", message, primary)
    }
}
