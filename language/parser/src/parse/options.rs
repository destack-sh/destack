use crate::ParserTriviaMode;

/// Parser semantic token retention mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ParserTokenHistory {
    /// Do not retain consumed semantic tokens during parsing.
    Stream,
    /// Retain consumed semantic tokens for diagnostics and tests.
    Record,
}

impl ParserTokenHistory {
    /// Return whether consumed semantic tokens are retained.
    #[inline]
    pub const fn records_tokens(self) -> bool {
        matches!(self, Self::Record)
    }
}

/// Parser options that can be configured externally.
#[derive(Debug, Copy, Clone)]
pub struct ParserOptions {
    /// Whether ambiguous tree literal syntax is disallowed.
    pub disallow_ambiguous_tree_literal: bool,
    /// The parser trivia retention mode.
    pub trivia_mode: ParserTriviaMode,
    /// Whether transparent parenthesized wrappers should be preserved in the tree.
    pub preserve_parenthesized_wrappers: bool,
    /// Whether the parser retains consumed semantic tokens.
    pub token_history: ParserTokenHistory,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            disallow_ambiguous_tree_literal: false,
            trivia_mode: ParserTriviaMode::Documentation,
            preserve_parenthesized_wrappers: false,
            token_history: ParserTokenHistory::Stream,
        }
    }
}
