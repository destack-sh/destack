use std::fmt::Debug;
use std::sync::Arc;

use destack_dir::{Token, TokenLiteral, TokenSpan, TokenType};
use destack_source::{File, FileId, LanguageType, Span};

use super::scanner::{Scanner, ScannerSnapshot};
use super::trivia::Trivia;

/// The options for the lexer.
#[derive(Debug, Default)]
pub(super) struct LexerOptions {
    /// The nested template strings starting parentheses depth stack.
    pub(super) template_string_stack: SnapshotStack<i32>,
    /// The depth of nested template string parentheses.
    pub(super) parentheses_depth: i32 = 0,
    /// Whether tree literal lexing is allowed in the current context.
    pub(super) allow_tree_literals: bool,
    /// Whether the next quoted string token is lexed as a tree attribute value.
    pub(super) in_tree_attribute_value: bool,
}

/// One entry in a snapshot-restorable stack.
#[derive(Debug, Copy, Clone)]
struct SnapshotStackEntry<T: Copy> {
    /// The stored stack value.
    value: T,
    /// The previous stack head.
    prev: Option<usize>,
}

/// One snapshot-restorable stack.
#[derive(Debug, Default)]
pub(super) struct SnapshotStack<T: Copy> {
    /// The append-only entry arena.
    entries: Vec<SnapshotStackEntry<T>>,
    /// The current stack head.
    head: Option<usize>,
}

/// One snapshot of a snapshot-restorable stack.
#[derive(Debug, Copy, Clone)]
pub(super) struct SnapshotStackState {
    /// The current stack head.
    head: Option<usize>,
    /// The number of entries alive at snapshot time.
    entries_len: usize,
}

impl<T: Copy> SnapshotStack<T> {
    /// Push one value onto the stack.
    #[inline]
    pub(super) fn push(&mut self, value: T) {
        let prev = self.head;
        self.entries.push(SnapshotStackEntry { value, prev });
        self.head = Some(self.entries.len() - 1);
    }

    /// Pop one value from the stack.
    #[inline]
    pub(super) fn pop(&mut self) -> Option<T> {
        let head = self.head?;
        let entry = self.entries[head];
        self.head = entry.prev;
        Some(entry.value)
    }

    /// Peek the current stack head.
    #[inline]
    pub(super) fn peek(&self) -> Option<T> {
        self.head.map(|head| self.entries[head].value)
    }

    /// Return one snapshot of the current stack state.
    #[inline]
    pub(super) fn snapshot(&self) -> SnapshotStackState {
        SnapshotStackState {
            head: self.head,
            entries_len: self.entries.len(),
        }
    }

    /// Restore one previously captured stack state.
    #[inline]
    pub(super) fn restore(&mut self, snapshot: SnapshotStackState) {
        self.entries.truncate(snapshot.entries_len);
        self.head = snapshot.head;
    }
}

/// Lexer over a source string.
pub struct Lexer {
    /// The source scanner.
    scanner: Scanner,
    /// The options for the lexer.
    pub(super) options: LexerOptions,
    /// The language type for parsing behavior.
    #[allow(unused)]
    pub(super) language: LanguageType,
    /// Live lexer trivia state.
    pub(super) trivia: Trivia,
    /// Whether an `@` token was seen.
    pub(super) has_at: bool,
    /// Whether the most recent side token contained a line terminator.
    pub(super) last_side_token_had_line_terminator: bool,
    /// Whether side trivia buffers should be retained.
    pub(super) retain_trivia_tokens: bool,
    /// The semantic tokens produced so far.
    pub(super) tokens: Vec<TokenSpan>,
    /// The side tokens produced so far.
    pub(super) side_tokens: Vec<TokenSpan>,
    /// Whether side trivia since the previous semantic token had a line terminator.
    pub(super) pending_line_terminator_before_next: bool,
    /// The number of non-newline semantic tokens.
    pub(super) attachable_semantic_token_count: u32,
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

/// Snapshot of lexer state for speculative parsing.
#[derive(Debug, Clone)]
pub struct LexerSnapshot {
    /// The scanner state.
    pub(super) scanner: ScannerSnapshot,
    /// Whether an `@` token has been observed.
    pub(super) has_at: bool,
    /// Whether the most recent side token at snapshot time had a line terminator.
    pub(super) last_side_token_had_line_terminator: bool,
    /// The semantic token count captured in the snapshot.
    pub(super) tokens_len: usize,
    /// The side token count captured in the snapshot.
    pub(super) side_tokens_len: usize,
    /// The template stack state at snapshot time.
    pub(super) template_string_stack: SnapshotStackState,
    /// The parentheses depth at snapshot time.
    pub(super) parentheses_depth: i32,
    /// Whether tree literal lexing was enabled at snapshot time.
    pub(super) allow_tree_literals: bool,
    /// Whether tree attribute value lexing was enabled at snapshot time.
    pub(super) in_tree_attribute_value: bool,
    /// Whether side trivia since the last semantic token had a line terminator.
    pub(super) pending_line_terminator_before_next: bool,
    /// The non-newline semantic token count at snapshot time.
    pub(super) attachable_semantic_token_count: u32,
    /// Whether EOF had been reached at snapshot time.
    pub(super) is_finished: bool,
    /// The cached EOF token at snapshot time.
    pub(super) eof_token: Option<TokenSpan>,
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
            options: LexerOptions {
                allow_tree_literals: language.supports_jsx(),
                ..LexerOptions::default()
            },
            language,
            trivia: Trivia::new(),
            has_at: false,
            last_side_token_had_line_terminator: false,
            retain_trivia_tokens: true,
            tokens: Vec::with_capacity(semantic_token_capacity),
            side_tokens: Vec::with_capacity(side_token_capacity),
            pending_line_terminator_before_next: true,
            attachable_semantic_token_count: 0,
            is_finished: false,
            eof_token: None,
        }
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

    /// Snapshot lexer state for speculative parsing.
    #[inline]
    pub fn snapshot(&self) -> LexerSnapshot {
        LexerSnapshot {
            scanner: self.scanner.snapshot(),
            has_at: self.has_at,
            last_side_token_had_line_terminator: self.last_side_token_had_line_terminator,
            tokens_len: self.tokens.len(),
            side_tokens_len: self.side_tokens.len(),
            template_string_stack: self.options.template_string_stack.snapshot(),
            parentheses_depth: self.options.parentheses_depth,
            allow_tree_literals: self.options.allow_tree_literals,
            in_tree_attribute_value: self.options.in_tree_attribute_value,
            pending_line_terminator_before_next: self.pending_line_terminator_before_next,
            attachable_semantic_token_count: self.attachable_semantic_token_count,
            is_finished: self.is_finished,
            eof_token: self.eof_token,
        }
    }

    /// Restore lexer state from a snapshot.
    #[inline]
    pub fn restore(&mut self, snapshot: LexerSnapshot) {
        self.scanner.restore(snapshot.scanner);
        self.options
            .template_string_stack
            .restore(snapshot.template_string_stack);
        self.options.parentheses_depth = snapshot.parentheses_depth;
        self.options.allow_tree_literals = snapshot.allow_tree_literals;
        self.options.in_tree_attribute_value = snapshot.in_tree_attribute_value;
        self.has_at = snapshot.has_at;
        self.last_side_token_had_line_terminator = snapshot.last_side_token_had_line_terminator;
        self.tokens.truncate(snapshot.tokens_len);
        self.side_tokens.truncate(snapshot.side_tokens_len);
        self.pending_line_terminator_before_next = snapshot.pending_line_terminator_before_next;
        self.attachable_semantic_token_count = snapshot.attachable_semantic_token_count;
        self.is_finished = snapshot.is_finished;
        self.eof_token = snapshot.eof_token;
    }

    /// Re-lex the current `/` or `/=` token as a regex literal.
    pub(crate) fn re_lex_as_regex(&mut self, current_token: TokenSpan) -> TokenSpan {
        let start = current_token.span.start as usize;
        self.scanner.start_re_lex(start, '/');

        let has_flags = self.eat_regex_string();
        let end = self.position() as u32;
        let token = Token::new(
            TokenType::Literal,
            end - current_token.span.start,
            Some(TokenLiteral::RegexString { has_flags }),
        );
        self.reset_token_start();

        TokenSpan {
            token,
            span: Span {
                file: current_token.span.file,
                start: current_token.span.start,
                end,
            },
        }
    }

    /// Re-lex the current token as a typed `<`.
    pub(crate) fn re_lex_as_typed_l_angle(&mut self, current_token: TokenSpan) -> TokenSpan {
        let start = current_token.span.start as usize;
        self.scanner.finish_one_byte_re_lex(start, '<');

        TokenSpan {
            token: Token::new(TokenType::LessThan, 1, None),
            span: Span {
                file: current_token.span.file,
                start: current_token.span.start,
                end: current_token.span.start + 1,
            },
        }
    }

    /// Re-lex the current token as one `>`.
    pub(crate) fn re_lex_as_r_angle(&mut self, current_token: TokenSpan) -> TokenSpan {
        let start = current_token.span.start as usize;
        self.scanner.finish_one_byte_re_lex(start, '>');

        TokenSpan {
            token: Token::new(TokenType::GreaterThan, 1, None),
            span: Span {
                file: current_token.span.file,
                start: current_token.span.start,
                end: current_token.span.start + 1,
            },
        }
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
