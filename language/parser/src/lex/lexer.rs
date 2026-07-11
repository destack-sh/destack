use std::fmt::Debug;
use std::sync::Arc;

use destack_dir::Token;
use destack_source::File;

use super::tokenizer::Tokenizer;
use super::trivia::Trivia;

/// Lexer over one source file.
pub struct Lexer {
    /// The raw tokenizer.
    pub(super) tokenizer: Tokenizer,
    /// Live lexer trivia state.
    pub(super) trivia: Trivia,
    /// The trivia retention mode.
    pub(super) trivia_mode: ParserTriviaMode,
    /// The semantic tokens produced so far.
    pub(super) tokens: Vec<Token>,
    /// The side tokens produced so far.
    pub(super) side_tokens: Vec<Token>,
    /// Whether the next semantic token starts after a line terminator.
    pub(super) pending_line_terminator_before_next: bool,
    /// The cached EOF token.
    pub(super) eof_token: Option<Token>,
}

impl Debug for Lexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Lexer {{ file_id: {:?}, position: {} }}>",
            self.tokenizer.file_id(),
            self.tokenizer.position()
        )
    }
}

impl Lexer {
    /// Create a lexer for one source file.
    pub fn new(file: Arc<File>) -> Self {
        Self {
            tokenizer: Tokenizer::new(file),
            trivia: Trivia::new(),
            trivia_mode: ParserTriviaMode::default(),
            tokens: Vec::new(),
            side_tokens: Vec::new(),
            pending_line_terminator_before_next: true,
            eof_token: None,
        }
    }

    /// Return whether the most recent side token contained a line terminator.
    #[inline]
    pub(super) fn side_token_has_line_terminator(&self) -> bool {
        self.tokenizer.side_token_has_line_terminator()
    }
}

/// Parser trivia retention mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ParserTriviaMode {
    /// Ignore comments and whitespace side tokens.
    Ignore,
    /// Retain documentation and legal comments only.
    #[default]
    Documentation,
    /// Retain every comment and whitespace side token.
    Full,
}

impl ParserTriviaMode {
    /// Return whether comment records should be retained.
    #[inline]
    pub const fn keeps_comments(self) -> bool {
        matches!(self, Self::Documentation | Self::Full)
    }

    /// Return whether every side token should be retained.
    #[inline]
    pub const fn keeps_side_tokens(self) -> bool {
        matches!(self, Self::Full)
    }
}
