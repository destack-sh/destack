use crate::ParserTriviaMode;

/// Parser options that can be configured externally.
#[derive(Debug, Copy, Clone)]
pub struct ParserOptions {
    /// Whether ambiguous tree literal syntax is disallowed.
    pub disallow_ambiguous_tree_literal: bool,
    /// The parser trivia retention mode.
    pub trivia_mode: ParserTriviaMode,
    /// Whether transparent parenthesized wrappers should be preserved in the tree.
    pub preserve_parenthesized_wrappers: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            disallow_ambiguous_tree_literal: false,
            trivia_mode: ParserTriviaMode::Documentation,
            preserve_parenthesized_wrappers: true,
        }
    }
}
