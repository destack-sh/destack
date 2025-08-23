use destack_language_lexer::SemanticToken;

use crate::ParseError;

/// A parser for the Destack Language.
///
/// The Parser works on semantic undifferentiated Tokens (keywords are contextual).
/// Whitespace and regular line comments are ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct Parser<'a> {
    /// The tokens to parse.
    pub(crate) tokens: &'a [SemanticToken],
    /// The current position in the tokens.
    pub(crate) pos: usize,
}

/// A result of a parse operation.
pub type ParseResult<'a, T> = Result<T, ParseError>;

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [SemanticToken]) -> Self {
        Self { tokens, pos: 0 }
    }
}
