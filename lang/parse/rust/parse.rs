use destack_lang_lex::SemanticToken;

/// A parser for the Destack Language.
///
/// The Parser works on semantic undifferentiated Tokens (keywords are contextual).
/// Whitespace and regular line comments are ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct Parser {
    /// The tokens to parse.
    tokens: Vec<SemanticToken>,
    /// The current position in the tokens.
    pos: u32,
    /// Previous position to restore to for speculative parsing.
    last_good_pos: u32,
}

impl Parser {
    pub fn new(tokens: impl IntoIterator<Item = SemanticToken>) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            pos: 0,
            last_good_pos: 0,
        }
    }

    pub fn from_vec(tokens: Vec<SemanticToken>) -> Self {
        Self {
            tokens,
            pos: 0,
            last_good_pos: 0,
        }
    }
}
