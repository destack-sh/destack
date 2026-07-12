use core::fmt;

use destack_dir::{NodeType, Token, TokenSpan, TokenType};
use destack_source::{ByteRange, ContentId, Diagnostic, DiagnosticLabel, FileId, Span};
use std::error::Error;

/// One structural parser error used for recovery and diagnostics.
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub struct ParserError {
    /// The source byte range of the error.
    pub range: ByteRange,
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
    /// The source byte range of the error.
    range: ByteRange,
    /// The actual token type at the error span.
    actual: Option<TokenType>,
}

impl From<ByteRange> for ParserErrorLocation {
    /// Create a parser error location from a source byte range.
    #[inline]
    fn from(range: ByteRange) -> Self {
        Self {
            range,
            actual: None,
        }
    }
}

impl From<Token> for ParserErrorLocation {
    /// Create a parser error location from a compact source token.
    #[inline]
    fn from(token: Token) -> Self {
        Self {
            range: token.range(),
            actual: Some(token.ty()),
        }
    }
}

impl From<TokenSpan> for ParserErrorLocation {
    /// Create a parser error location from a token span.
    #[inline]
    fn from(token: TokenSpan) -> Self {
        Self {
            range: token.span.range(),
            actual: Some(token.token.ty()),
        }
    }
}

/// One parser operation result.
pub type ParserResult<T> = Result<T, ParserError>;

/// Extension methods for parser operation results.
pub trait ParserResultExt<T> {
    /// Attach the grammar node containing the error.
    fn in_node(self, node_type: NodeType) -> Result<T, ParserError>;
}

impl<T> ParserResultExt<T> for Result<T, ParserError> {
    /// Attach the grammar node when the error has no existing owner.
    #[inline]
    fn in_node(self, node_type: NodeType) -> Self {
        match self {
            Err(error) if error.node_type.is_none() => Err(error.in_node(node_type)),
            result => result,
        }
    }
}

impl ParserError {
    /// Return this error range as a span in the parsed file.
    #[inline]
    pub fn span(self, file_id: FileId) -> Span {
        Span::new(file_id, self.range.start, self.range.end)
    }

    /// Create a parser error for an unexpected location.
    #[inline]
    pub fn unexpected(location: impl Into<ParserErrorLocation>) -> Self {
        let location = location.into();

        Self {
            range: location.range,
            actual: location.actual,
            expected: None,
            node_type: None,
        }
    }

    /// Create a parser error with an expected alternative at a location.
    #[inline]
    pub fn expected(location: impl Into<ParserErrorLocation>, expected: TokenType) -> Self {
        let location = location.into();

        Self {
            range: location.range,
            actual: location.actual,
            expected: Some(expected),
            node_type: None,
        }
    }

    /// Attach the grammar node containing this error.
    #[inline]
    pub fn in_node(mut self, node_type: NodeType) -> Self {
        self.node_type = Some(node_type);

        self
    }

    /// Convert this parse error into one source diagnostic.
    pub fn to_diagnostic(&self, content: ContentId, file_id: FileId) -> Diagnostic {
        let node = self
            .node_type
            .map_or_else(String::new, |node_type| format!(" in {node_type:?}"));

        // describe the expected or actual token
        let label = if let Some(expected) = self.expected {
            format!("expected {expected}{node}")
        } else if let Some(actual) = self.actual {
            format!("unexpected {actual}{node}")
        } else {
            format!("unexpected syntax{node}")
        };

        // anchor the diagnostic at the parser error range
        let message = format!("parse error: {label}");
        let span = self.span(file_id);
        let primary = DiagnosticLabel::message(content, span, label);

        Diagnostic::error("EP001", message, primary)
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expected {
            Some(token_type) => write!(f, "expected {token_type} at {:?}", self.range)?,
            None => match self.actual {
                Some(token_type) => write!(f, "unexpected {token_type} at {:?}", self.range)?,
                None => write!(f, "unexpected syntax at {:?}", self.range)?,
            },
        }

        Ok(())
    }
}

impl Error for ParserError {}
