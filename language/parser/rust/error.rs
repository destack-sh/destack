use destack_language_lexer::Span;

/// An error that can occur during parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    SyntaxError(Span),
    UnexpectedToken(Span),
}

/// A result of a parse operation.
pub type ParseResult<T> = Result<T, ParseError>;
