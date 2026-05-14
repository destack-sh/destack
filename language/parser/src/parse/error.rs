use core::fmt;

use destack_dir::{NodeType, TokenSpan, TokenType};
use destack_source::{Diagnostic, DiagnosticLabel, File, Span};

/// Error when parsing the parsed DIR.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// The span of the error.
    pub span: Span,
    /// The expected token type.
    pub expected: Option<TokenType>,
    /// The node type we tried to parse.
    pub node_type: Option<NodeType>,
    /// The source error.
    pub source: Option<Box<ParseError>>,
}

/// The result of an source parse.
pub type ParseResult<T> = Result<T, ParseError>;

pub trait ParseResultExt<T> {
    /// Set the node type of the error.
    fn for_node_type(self, node_type: NodeType) -> Result<T, ParseError>;
}

impl<T> ParseResultExt<T> for Result<T, ParseError> {
    /// Set the node type of the error (if not already set)
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

impl ParseError {
    /// Create a ParseError leaf without an expected alternative.
    #[inline]
    pub fn unexpected(span: Span) -> Self {
        ParseError {
            span,
            expected: None,
            node_type: None,
            source: None,
        }
    }

    /// Create a ParseError leaf with an unexpected token and node type.
    #[inline]
    pub fn unexpected_for(span: Span, node_type: NodeType) -> Self {
        ParseError {
            span,
            expected: None,
            node_type: Some(node_type),
            source: None,
        }
    }

    /// Create a ParseError leaf with an expected alternative.
    #[inline]
    pub fn expected(span: Span, expected: TokenType) -> Self {
        ParseError {
            span,
            expected: Some(expected),
            node_type: None,
            source: None,
        }
    }

    /// Create a ParseError leaf with an expected alternative and node type.
    #[inline]
    pub fn expected_for(span: Span, expected: TokenType, node_type: NodeType) -> Self {
        ParseError {
            span,
            expected: Some(expected),
            node_type: Some(node_type),
            source: None,
        }
    }

    /// Create a ParseError leaf from a source error with a new span.
    #[inline]
    pub fn from_source(span: Span, source: ParseError) -> Self {
        ParseError {
            span,
            expected: None,
            node_type: source.node_type,
            source: Some(Box::new(source)),
        }
    }

    /// Create a ParseError leaf from a source error with a new span.
    #[inline]
    pub fn from_source_maybe(span: Span, source: Option<ParseError>) -> Self {
        ParseError {
            span,
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
        let (self_span, self_node_type, self_expected) = self.leaf_content();
        let (other_span, other_node_type, other_expected) = other.leaf_content();
        self_span == other_span
            && self_node_type == other_node_type
            && self_expected == other_expected
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
    pub fn leaf_content(&self) -> (Span, Option<NodeType>, Option<TokenType>) {
        let leaf = self.leaf();
        (leaf.span, leaf.node_type, leaf.expected)
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
            Some(tok) => write!(f, "expected {tok} at {span:?}")?,
            None => write!(f, "unexpected token at {span:?}")?,
        }
        Ok(())
    }
}

// Optional: extension std::error::Error so callers can use `source()` if they like.
impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Some(source) = &self.source {
            Some(&**source)
        } else {
            None
        }
    }
}

impl ParseError {
    /// Convert this parse error into one source diagnostic.
    pub fn to_diagnostic(&self, source: &File, tokens: &[TokenSpan]) -> Diagnostic {
        let (span, node_type, expected) = self.leaf_content();
        let content = source.content_id();

        let token_at_primary_span = tokens
            .iter()
            .find(|token| token.span.start == span.start)
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End);
        let in_node_str = match node_type {
            Some(node_type) => format!(" in {node_type:?}"),
            None => "".to_string(),
        };
        let message = match expected {
            Some(token_type) => format!("parse error: expected {token_type}{in_node_str}"),
            None => format!("parse error: unexpected {token_at_primary_span}{in_node_str}"),
        };
        let primary = DiagnosticLabel::message(
            content,
            span,
            match expected {
                Some(token_type) => format!("expected {token_type}{in_node_str}"),
                None => format!("unexpected {token_at_primary_span}{in_node_str}"),
            },
        );

        Diagnostic::error("EP001", message, primary)
    }
}
