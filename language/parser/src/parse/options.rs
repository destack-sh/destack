use crate::ParserTriviaMode;

/// Parser options that can be configured externally.
#[derive(Debug, Copy, Clone)]
pub struct ParserOptions {
    /// The parser trivia retention mode.
    pub trivia_mode: ParserTriviaMode,
    /// Whether explicit parentheses should be preserved in the tree.
    pub retain_parentheses: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            trivia_mode: ParserTriviaMode::Documentation,
            retain_parentheses: false,
        }
    }
}
