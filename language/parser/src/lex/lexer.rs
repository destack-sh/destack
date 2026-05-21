use std::fmt::Debug;
use std::sync::Arc;

use destack_dir::TokenSpan;
use destack_source::{File, FileId, LanguageType, Span};

use super::scanner::Scanner;
use super::trivia::{Trivia, TriviaCheckpoint};

/// Lexer over a source string.
pub struct Lexer {
    /// The source scanner.
    scanner: Scanner,
    /// The options for the lexer.
    pub(super) options: LexerOptions,
    /// The language type for parsing behavior.
    pub(super) language: LanguageType,
    /// Live lexer trivia state.
    pub(super) trivia: Trivia,
    /// Whether the most recent side token contained a line terminator.
    pub(super) last_side_token_had_line_terminator: bool,
    /// The trivia retention mode.
    pub(super) trivia_mode: ParserTriviaMode,
    /// The semantic tokens produced so far.
    pub(super) tokens: Vec<TokenSpan>,
    /// The side tokens produced so far.
    pub(super) side_tokens: Vec<TokenSpan>,
    /// Whether side trivia since the previous semantic token had a line terminator.
    pub(super) pending_line_terminator_before_next: bool,
    /// Whether EOF has been reached.
    pub(super) is_finished: bool,
    /// The cached EOF token, when available.
    pub(super) eof_token: Option<TokenSpan>,
}

impl Debug for Lexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Lexer {{ file_id: {:?}, position: {} }}>",
            self.file_id(),
            self.position()
        )
    }
}

impl Lexer {
    /// Create a new Lexer from a file.
    pub fn new(file: Arc<File>, language: LanguageType) -> Lexer {
        let source_len = file.text().len();
        let estimated_tokens = source_len / 6;
        let semantic_token_capacity = estimated_tokens;
        let side_token_capacity = estimated_tokens / 2;

        Lexer {
            scanner: Scanner::new(file),
            options: LexerOptions::default(),
            language,
            trivia: Trivia::new(),
            last_side_token_had_line_terminator: false,
            trivia_mode: ParserTriviaMode::default(),
            tokens: Vec::with_capacity(semantic_token_capacity),
            side_tokens: Vec::with_capacity(side_token_capacity),
            pending_line_terminator_before_next: true,
            is_finished: false,
            eof_token: None,
        }
    }

    /// Create a checkpoint for speculative parser movement.
    pub(crate) fn checkpoint(&self) -> LexerCheckpoint {
        LexerCheckpoint {
            scanner: self.scanner.clone(),
            options: self.options.checkpoint(),
            trivia: self.trivia.checkpoint(),
            last_side_token_had_line_terminator: self.last_side_token_had_line_terminator,
            pending_line_terminator_before_next: self.pending_line_terminator_before_next,
            trivia_mode: self.trivia_mode,
            is_finished: self.is_finished,
            eof_token: self.eof_token,
            tokens_len: self.tokens.len(),
            side_tokens_len: self.side_tokens.len(),
        }
    }

    /// Restore the lexer to a checkpoint.
    pub(crate) fn restore(&mut self, checkpoint: LexerCheckpoint) {
        self.scanner = checkpoint.scanner;
        self.options.restore(checkpoint.options);
        self.trivia.restore(checkpoint.trivia);
        self.last_side_token_had_line_terminator = checkpoint.last_side_token_had_line_terminator;
        self.pending_line_terminator_before_next = checkpoint.pending_line_terminator_before_next;
        self.trivia_mode = checkpoint.trivia_mode;
        self.is_finished = checkpoint.is_finished;
        self.eof_token = checkpoint.eof_token;
        self.tokens.truncate(checkpoint.tokens_len);
        self.side_tokens.truncate(checkpoint.side_tokens_len);
    }

    /// Return the remaining source text.
    #[inline]
    pub fn remaining_text(&self) -> &str {
        self.scanner.remaining()
    }

    /// Return the string content of a span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &str {
        self.scanner.span_str(span)
    }

    /// Return the last eaten symbol.
    #[inline]
    pub fn previous(&self) -> char {
        self.scanner.previous()
    }

    /// Peek the next symbol from the input stream without consuming it.
    #[inline]
    pub fn peek(&self) -> char {
        self.scanner.peek()
    }

    /// Peek the second symbol from the input stream without consuming it.
    #[inline]
    pub fn peek_next(&self) -> char {
        self.scanner.peek_next()
    }

    /// Peek the third symbol from the input stream without consuming it.
    #[inline]
    pub fn peek_next_next(&self) -> char {
        self.scanner.peek_next_next()
    }

    /// Return whether there is nothing more to consume.
    #[inline]
    pub fn is_end(&self) -> bool {
        self.scanner.is_end()
    }

    /// Return the byte length consumed for the current token.
    #[inline]
    pub fn token_len(&self) -> u32 {
        self.scanner.token_len()
    }

    /// Reset the current token start to the current scanner position.
    #[inline]
    pub fn reset_token_start(&mut self) {
        self.scanner.reset_token_start();
    }

    /// Move to the next character.
    pub fn eat(&mut self) -> Option<char> {
        self.scanner.eat()
    }

    /// Advance by a known run of ascii bytes.
    #[inline]
    pub(super) fn advance_ascii_bytes(&mut self, count: usize, last_byte: u8) {
        self.scanner.advance_ascii_bytes(count, last_byte);
    }

    /// Return the source file ID.
    #[inline]
    pub fn file_id(&self) -> FileId {
        self.scanner.file_id()
    }

    /// Return the source file.
    #[inline]
    pub(super) fn file(&self) -> &File {
        self.scanner.file()
    }

    /// Return the source text.
    #[inline]
    pub fn source_text(&self) -> &str {
        self.scanner.text()
    }

    /// Return the current scanner byte position.
    #[inline]
    pub fn position(&self) -> usize {
        self.scanner.position()
    }

    /// Move the scanner to one byte position.
    #[inline]
    pub(crate) fn set_position(&mut self, position: usize) {
        self.scanner.set_position(position);
    }

    /// Return whether the most recent side token contained a line terminator.
    #[inline]
    pub(super) fn side_token_had_line_terminator(&self) -> bool {
        self.last_side_token_had_line_terminator
    }

    /// Eat symbols while predicate returns true or until the end of file is reached.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        self.scanner.eat_while(&mut predicate);
    }

    /// Eat symbols until the first occurrence of the given byte is found.
    /// If the byte is not found, the entire string is consumed.
    #[inline]
    pub fn eat_until(&mut self, byte: u8) {
        self.scanner.eat_until(byte);
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

/// A checkpoint for live lexer cursor rollback.
#[derive(Debug)]
pub(crate) struct LexerCheckpoint {
    /// The source scanner.
    scanner: Scanner,
    /// The lexer option checkpoint.
    options: LexerOptionsCheckpoint,
    /// The live trivia checkpoint.
    trivia: TriviaCheckpoint,
    /// Whether the most recent side token had a line terminator.
    last_side_token_had_line_terminator: bool,
    /// Whether the next semantic token is line-leading.
    pending_line_terminator_before_next: bool,
    /// The trivia retention mode.
    trivia_mode: ParserTriviaMode,
    /// Whether EOF has been reached.
    is_finished: bool,
    /// The EOF token if EOF has been reached.
    eof_token: Option<TokenSpan>,
    /// The semantic token buffer length.
    tokens_len: usize,
    /// The side token buffer length.
    side_tokens_len: usize,
}

/// The options for the lexer.
#[derive(Debug, Default)]
pub(super) struct LexerOptions {
    /// The nested template strings starting parentheses depth stack.
    pub(super) template_string_stack: LexerStack<i32>,
    /// The depth of nested template string parentheses.
    pub(super) parentheses_depth: i32 = 0,
}

/// A checkpoint for contextual lexer options.
#[derive(Debug)]
struct LexerOptionsCheckpoint {
    /// The active template string stack mark.
    template_string_stack: LexerStackMark,
    /// The nested delimiter depth.
    parentheses_depth: i32,
}

impl LexerOptions {
    /// Create a checkpoint for speculative contextual lexing.
    #[inline]
    fn checkpoint(&self) -> LexerOptionsCheckpoint {
        LexerOptionsCheckpoint {
            template_string_stack: self.template_string_stack.mark(),
            parentheses_depth: self.parentheses_depth,
        }
    }

    /// Restore one contextual lexing checkpoint.
    #[inline]
    fn restore(&mut self, checkpoint: LexerOptionsCheckpoint) {
        self.template_string_stack
            .restore(checkpoint.template_string_stack);
        self.parentheses_depth = checkpoint.parentheses_depth;
    }
}

/// One compact stack.
#[derive(Debug, Default)]
pub(super) struct LexerStack<T: Copy> {
    /// The stack entries.
    entries: Vec<LexerStackEntry<T>>,
    /// The active stack head.
    head: Option<usize>,
}

/// One linked stack entry.
#[derive(Debug, Copy, Clone)]
struct LexerStackEntry<T: Copy> {
    /// The entry value.
    value: T,
    /// The previous stack entry.
    previous: Option<usize>,
}

/// One reversible lexer stack position.
#[derive(Debug, Copy, Clone)]
struct LexerStackMark {
    /// The active stack head.
    head: Option<usize>,
    /// The stack entry count.
    entries_len: usize,
}

impl<T: Copy> LexerStack<T> {
    /// Push one value onto the stack.
    #[inline]
    pub(super) fn push(&mut self, value: T) {
        self.entries.push(LexerStackEntry {
            value,
            previous: self.head,
        });
        self.head = Some(self.entries.len() - 1);
    }

    /// Pop one value from the stack.
    #[inline]
    pub(super) fn pop(&mut self) -> Option<T> {
        let head = self.head?;
        let entry = self.entries[head];
        self.head = entry.previous;

        Some(entry.value)
    }

    /// Peek the current stack head.
    #[inline]
    pub(super) fn peek(&self) -> Option<T> {
        self.head.map(|head| self.entries[head].value)
    }

    /// Mark the current stack position.
    #[inline]
    fn mark(&self) -> LexerStackMark {
        LexerStackMark {
            head: self.head,
            entries_len: self.entries.len(),
        }
    }

    /// Restore one marked stack position.
    #[inline]
    fn restore(&mut self, mark: LexerStackMark) {
        self.head = mark.head;
        self.entries.truncate(mark.entries_len);
    }
}
