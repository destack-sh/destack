use dyst_language_source::Span;
use dyst_language_token::TokenType;

/// Error when parsing the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken {
        span: Span,
        expected_token: Option<TokenType> = None,
    },
}

impl ParseError {
    /// Create a ParseError for an unexpected token.
    pub(crate) fn unexpected(span: Span) -> Self {
        Self::UnexpectedToken {
            span,
            expected_token: None,
        }
    }

    /// Create a ParseError for an expected token.
    pub(crate) fn expected_token(span: Span, expected_token: TokenType) -> Self {
        Self::UnexpectedToken {
            span,
            expected_token: Some(expected_token),
        }
    }
}

/// The result of a parse operation.
pub type ParseResult<T> = Result<T, ParseError>;

// todo!: Diagnostics
