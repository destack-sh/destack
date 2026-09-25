use std::sync::Arc;

use tspp_source::{File, FileId, Span};

use super::scanner::Scanner;

/// Raw token recognition over one source file.
pub(crate) struct Tokenizer {
    /// The source file keeping scanner pointers alive.
    file: Arc<File>,
    /// The source byte cursor.
    pub(super) scanner: Scanner,
    /// The tokenization state.
    pub(super) state: TokenizerState,
}

impl Tokenizer {
    /// Create a tokenizer at the beginning of one file.
    pub(crate) fn new(file: Arc<File>) -> Self {
        Self::at(file, 0)
    }

    /// Create a tokenizer at one source byte position.
    pub(crate) fn at(file: Arc<File>, position: u32) -> Self {
        let scanner = Scanner::at(&file, position);

        Self {
            file,
            scanner,
            state: TokenizerState::default(),
        }
    }

    /// Return source text covered by one span.
    #[inline]
    pub(crate) fn span_str(&self, span: Span) -> &str {
        self.file.span_str(span)
    }

    /// Return whether no source bytes remain.
    #[inline]
    pub(crate) fn is_end(&self) -> bool {
        self.scanner.is_end()
    }

    /// Return the current raw token byte length.
    #[inline]
    pub(crate) fn token_len(&self) -> u32 {
        self.scanner.token_len()
    }

    /// Return the current raw token bytes.
    #[inline]
    pub(super) fn token_bytes(&self) -> &[u8] {
        self.scanner.token_bytes()
    }

    /// Reset the raw token start to the current byte position.
    #[inline]
    pub(crate) fn reset_token_start(&mut self) {
        self.scanner.reset_token_start();
    }

    /// Advance by a known run of ASCII bytes.
    #[inline]
    pub(crate) fn advance_ascii_bytes(&mut self, count: usize) {
        self.scanner.advance_ascii_bytes(count);
    }

    /// Return the source file ID.
    #[inline]
    pub(crate) fn file_id(&self) -> FileId {
        self.file.id
    }

    /// Return the current byte position.
    #[inline]
    pub(crate) fn position(&self) -> usize {
        self.scanner.position()
    }

    /// Return whether the most recent trivia token contained a line terminator.
    #[inline]
    pub(crate) fn trivia_token_has_line_terminator(&self) -> bool {
        self.state.last_trivia_token_has_line_terminator
    }

    /// Eat characters while one predicate accepts them.
    pub(crate) fn eat_while(&mut self, predicate: impl FnMut(char) -> bool) {
        self.scanner.eat_while(predicate);
    }

    /// Eat source bytes until one byte or EOF.
    #[inline]
    pub(crate) fn eat_until(&mut self, byte: u8) {
        self.scanner.eat_until(byte);
    }
}

/// Mutable raw tokenization state.
#[derive(Debug, Default)]
pub(super) struct TokenizerState {
    /// The delimiter depths of active template interpolations.
    pub(super) interpolation_depths: Vec<i32>,
    /// The current grouping delimiter depth.
    pub(super) delimiter_depth: i32,
    /// Whether the most recent trivia token contained a line terminator.
    pub(super) last_trivia_token_has_line_terminator: bool,
}

impl TokenizerState {
    /// Enter one grouping delimiter inside an active template interpolation.
    #[inline]
    pub(super) fn enter_delimiter(&mut self) {
        if !self.interpolation_depths.is_empty() {
            self.delimiter_depth += 1;
        }
    }

    /// Leave one grouping delimiter inside an active template interpolation.
    #[inline]
    pub(super) fn leave_delimiter(&mut self) {
        if !self.interpolation_depths.is_empty() {
            self.delimiter_depth -= 1;
        }
    }
}
