use core::fmt;

use destack_dir::{NodeType, TokenSpan, TokenType};
use destack_source::{ContentId, Diagnostic, DiagnosticLabel, Span};

/// One structural parser error used for recovery and diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct ParserError {
    /// The span of the error.
    pub span: Span,
    /// The actual token type at the error span.
    pub actual: Option<TokenType>,
    /// The expected token type.
    pub expected: Option<TokenType>,
    /// The node type we tried to parse.
    pub node_type: Option<NodeType>,
    /// The source error.
    pub source: Option<Box<ParserError>>,
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

/// The leaf payload of one parser error chain.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ParserErrorLeaf {
    /// The source span of the leaf error.
    pub span: Span,
    /// The actual token type at the leaf error.
    pub actual: Option<TokenType>,
    /// The node type the parser tried to parse at the leaf error.
    pub node_type: Option<NodeType>,
    /// The expected token type at the leaf error.
    pub expected: Option<TokenType>,
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
        if let Err(e) = &self
            && e.node_type.is_none()
        {
            Err(e.clone().for_node_type(node_type))
        } else {
            self
        }
    }
}

impl ParserError {
    /// Create a parser error leaf for an unexpected location.
    #[inline]
    pub fn unexpected(location: impl Into<ParserErrorLocation>) -> Self {
        let location = location.into();

        ParserError {
            span: location.span,
            actual: location.actual,
            expected: None,
            node_type: None,
            source: None,
        }
    }

    /// Create a parser error leaf for an unexpected location and node type.
    #[inline]
    pub fn unexpected_for(location: impl Into<ParserErrorLocation>, node_type: NodeType) -> Self {
        let location = location.into();

        ParserError {
            span: location.span,
            actual: location.actual,
            expected: None,
            node_type: Some(node_type),
            source: None,
        }
    }

    /// Create a parser error leaf with an expected alternative at a location.
    #[inline]
    pub fn expected(location: impl Into<ParserErrorLocation>, expected: TokenType) -> Self {
        let location = location.into();

        ParserError {
            span: location.span,
            actual: location.actual,
            expected: Some(expected),
            node_type: None,
            source: None,
        }
    }

    /// Create a parser error leaf with an expected alternative and node type.
    #[inline]
    pub fn expected_for(
        location: impl Into<ParserErrorLocation>,
        expected: TokenType,
        node_type: NodeType,
    ) -> Self {
        let location = location.into();

        ParserError {
            span: location.span,
            actual: location.actual,
            expected: Some(expected),
            node_type: Some(node_type),
            source: None,
        }
    }

    /// Create a parser error leaf from a source error with a new span.
    #[inline]
    pub fn from_source(span: Span, source: ParserError) -> Self {
        ParserError {
            span,
            actual: None,
            expected: None,
            node_type: source.node_type,
            source: Some(Box::new(source)),
        }
    }

    /// Create a parser error leaf from a source error with a new span.
    #[inline]
    pub fn from_source_maybe(span: Span, source: Option<ParserError>) -> Self {
        ParserError {
            span,
            actual: None,
            expected: None,
            node_type: source.as_ref().and_then(|s| s.node_type),
            source: source.map(Box::new),
        }
    }

    /// Set the node type of the error.
    #[inline]
    pub fn for_node_type(mut self, node_type: NodeType) -> Self {
        self.node_type = Some(node_type);
        self
    }

    /// Compare content.
    pub fn eq_content(&self, other: &Self) -> bool {
        let self_leaf = self.leaf_content();
        let other_leaf = other.leaf_content();

        self_leaf.span == other_leaf.span
            && self_leaf.node_type == other_leaf.node_type
            && self_leaf.expected == other_leaf.expected
    }

    /// Get the leaf error.
    pub fn leaf(&self) -> &ParserError {
        if let Some(source) = &self.source {
            source.leaf()
        } else {
            self
        }
    }

    /// Get the leaf content.
    pub fn leaf_content(&self) -> ParserErrorLeaf {
        let leaf = self.leaf();

        ParserErrorLeaf {
            span: leaf.span,
            actual: leaf.actual,
            node_type: leaf.node_type,
            expected: leaf.expected,
        }
    }

    /// Get the leaf span.
    pub fn leaf_span(&self) -> Span {
        let leaf = self.leaf();
        leaf.span
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let leaf = self.leaf_content();

        match leaf.expected {
            Some(tok) => write!(f, "expected {tok} at {:?}", leaf.span)?,
            None => match leaf.actual {
                Some(tok) => write!(f, "unexpected {tok} at {:?}", leaf.span)?,
                None => write!(f, "unexpected syntax at {:?}", leaf.span)?,
            },
        }

        Ok(())
    }
}

impl std::error::Error for ParserError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Some(source) = &self.source {
            Some(&**source)
        } else {
            None
        }
    }
}

impl ParserError {
    /// Convert this parse error into one source diagnostic.
    pub fn to_diagnostic(&self, content: ContentId) -> Diagnostic {
        let leaf = self.leaf_content();
        let in_node_str = match leaf.node_type {
            Some(node_type) => format!(" in {node_type:?}"),
            None => "".to_string(),
        };

        let (message, label) = match leaf.expected {
            Some(token_type) => (
                format!("parse error: expected {token_type}{in_node_str}"),
                format!("expected {token_type}{in_node_str}"),
            ),
            None => {
                let actual_str = match leaf.actual {
                    Some(actual) => actual.to_string(),
                    None => "syntax".to_string(),
                };

                (
                    format!("parse error: unexpected {actual_str}{in_node_str}"),
                    format!("unexpected {actual_str}{in_node_str}"),
                )
            }
        };
        let primary = DiagnosticLabel::message(content, leaf.span, label);

        Diagnostic::error("EP001", message, primary)
    }
}
