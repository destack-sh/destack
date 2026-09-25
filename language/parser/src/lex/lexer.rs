use std::fmt::Debug;
use std::sync::Arc;

use tspp_dir::Token;
use tspp_source::File;

use super::comment::LexerComments;
use super::tokenizer::Tokenizer;

/// Lexer over one source file.
pub struct Lexer {
    /// The raw tokenizer.
    pub(super) tokenizer: Tokenizer,
    /// Live lexer comment state.
    pub(super) comments: LexerComments,
    /// The comment retention mode.
    pub(super) comment_retention: CommentRetention,
    /// The semantic tokens produced so far.
    pub(super) tokens: Vec<Token>,
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
            comments: LexerComments::new(),
            comment_retention: CommentRetention::default(),
            tokens: Vec::new(),
            pending_line_terminator_before_next: true,
            eof_token: None,
        }
    }

    /// Resume ordinary lexing after one parser-visible token.
    pub(crate) fn resume(
        file: Arc<File>,
        previous: Token,
        comment_retention: CommentRetention,
    ) -> Self {
        // anchor retained comments to the preceding contextual token
        let mut comments = LexerComments::new();
        comments.record_token(previous.ty(), previous.start());

        // resume ordinary tokenization at the contextual token boundary
        Self {
            tokenizer: Tokenizer::at(file, previous.end()),
            comments,
            comment_retention,
            tokens: Vec::new(),
            pending_line_terminator_before_next: false,
            eof_token: None,
        }
    }

    /// Return whether the most recent trivia token contained a line terminator.
    #[inline]
    pub(super) fn trivia_token_has_line_terminator(&self) -> bool {
        self.tokenizer.trivia_token_has_line_terminator()
    }
}

/// Source comment retention during parsing.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum CommentRetention {
    /// Ignore every comment.
    Ignore,
    /// Retain documentation and legal comments only.
    #[default]
    Documentation,
    /// Retain every comment.
    All,
}

impl CommentRetention {
    /// Return whether comment records should be retained.
    #[inline]
    pub const fn retains_comments(self) -> bool {
        matches!(self, Self::Documentation | Self::All)
    }
}
