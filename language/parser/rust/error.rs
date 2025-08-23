use destack_language_lexer::SemanticToken;

/// An error that can occur during parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken(SemanticToken),
}
