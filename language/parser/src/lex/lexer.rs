use std::fmt::Debug;
use std::sync::Arc;

use destack_ast::{Keyword, LiteralType, Token, TokenSpan, TokenType};
use destack_source::{File, FileId, LanguageType, Span};

use super::trivia::{Trivia, TriviaMark};

use memchr::memchr;

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
    /// Create one stack with preallocated entry capacity.
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            head: None,
        }
    }

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

    /// Clear the active stack and drop retained entries.
    #[inline]
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.head = None;
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
    /// The source file.
    pub file: Arc<File>,
    /// The source ID.
    pub file_id: FileId,
    /// The current head ("next") byte position in the string.
    pub(super) pos: usize,
    /// The options for the lexer.
    pub(super) options: LexerOptions,
    /// The byte position where the current token started.
    token_start: usize,
    /// The previous character.
    prev: char,
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
    /// Dense metadata for semantic token indexes.
    pub(super) token_data: Vec<SemanticTokenData>,
    /// The stack of open parenthesis token indexes.
    pub(super) paren_stack: SnapshotStack<usize>,
    /// The stack of open brace token indexes.
    pub(super) brace_stack: SnapshotStack<usize>,
    /// The stack of open bracket token indexes.
    pub(super) bracket_stack: SnapshotStack<usize>,
    /// Whether side trivia since the previous semantic token had a line terminator.
    pub(super) pending_line_terminator_before_next: bool,
    /// Whether side trivia since the previous semantic token had a comment token.
    pub(super) pending_comment_before_next: bool,
    /// The number of semantic newline tokens.
    pub(super) semantic_newline_token_count: u32,
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
            "<Lexer {{ file_id: {:?}, pos: {} }}>",
            self.file_id, self.pos
        )
    }
}

pub const EOF_CHAR: char = '\0';

/// Dense metadata for one materialized semantic token.
#[derive(Debug, Copy, Clone)]
pub(super) struct SemanticTokenData {
    /// The matching close token index for an opening delimiter.
    pub(super) matching_pair: u32,
    /// The side token count before this semantic token.
    pub(super) side_tokens_len_before: usize,
    /// The contextual keyword classification for identifier tokens.
    pub(super) keyword: Option<Keyword>,
    /// Whether trivia before this token contains a line terminator.
    pub(super) has_line_terminator_before: bool,
    /// Whether trivia before this token contains a comment token.
    pub(super) has_comment_before: bool,
}

impl SemanticTokenData {
    /// Return an empty semantic token metadata record.
    #[inline]
    pub(super) const fn new(
        side_tokens_len_before: usize,
        keyword: Option<Keyword>,
        has_line_terminator_before: bool,
    ) -> Self {
        Self {
            matching_pair: u32::MAX,
            side_tokens_len_before,
            keyword,
            has_line_terminator_before,
            has_comment_before: false,
        }
    }
}

/// Snapshot of lexer state for speculative parsing.
#[derive(Debug, Clone)]
pub struct LexerSnapshot {
    /// The byte position of the lexer head.
    pub(super) pos: usize,
    /// The byte position where the current token started.
    pub(super) token_start: usize,
    /// The most recently consumed character.
    pub(super) prev: char,
    /// The trivia restore mark at snapshot time.
    pub(super) trivia_mark: TriviaMark,
    /// Whether an `@` token has been observed.
    pub(super) has_at: bool,
    /// Whether the most recent side token at snapshot time had a line terminator.
    pub(super) last_side_token_had_line_terminator: bool,
    /// The semantic token count captured in the snapshot.
    pub(super) tokens_len: usize,
    /// The side token count captured in the snapshot.
    pub(super) side_tokens_len: usize,
    /// The open parenthesis stack at snapshot time.
    pub(super) paren_stack: SnapshotStackState,
    /// The open brace stack at snapshot time.
    pub(super) brace_stack: SnapshotStackState,
    /// The open bracket stack at snapshot time.
    pub(super) bracket_stack: SnapshotStackState,
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
    /// Whether side trivia since the last semantic token had a comment token.
    pub(super) pending_comment_before_next: bool,
    /// The semantic newline token count at snapshot time.
    pub(super) semantic_newline_token_count: u32,
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
        let file_id = file.id;
        let source_len = file.text().len();
        let estimated_tokens = source_len / 6;
        let semantic_token_capacity = estimated_tokens;
        let side_token_capacity = estimated_tokens / 2;

        Lexer {
            file,
            file_id,
            pos: 0,
            options: LexerOptions {
                allow_tree_literals: language.supports_jsx(),
                ..LexerOptions::default()
            },
            token_start: 0,
            prev: EOF_CHAR,
            language,
            trivia: Trivia::new(),
            has_at: false,
            last_side_token_had_line_terminator: false,
            retain_trivia_tokens: true,
            tokens: Vec::with_capacity(semantic_token_capacity),
            side_tokens: Vec::with_capacity(side_token_capacity),
            token_data: Vec::with_capacity(semantic_token_capacity),
            paren_stack: SnapshotStack::with_capacity(semantic_token_capacity / 64),
            brace_stack: SnapshotStack::with_capacity(semantic_token_capacity / 64),
            bracket_stack: SnapshotStack::with_capacity(semantic_token_capacity / 64),
            pending_line_terminator_before_next: false,
            pending_comment_before_next: false,
            semantic_newline_token_count: 0,
            attachable_semantic_token_count: 0,
            is_finished: false,
            eof_token: None,
        }
    }

    /// Gets the underlying string.
    #[inline]
    pub fn as_str(&self) -> &str {
        let source = self.file.text();
        if self.pos >= source.len() {
            return "";
        }
        &source[self.pos..]
    }

    /// Gets the string content of a span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &str {
        self.file.span_str(span)
    }

    /// Gets the last eaten symbol (or `'\0'` in release builds).
    #[inline]
    pub fn prev(&self) -> char {
        self.prev
    }

    /// Peeks the next symbol from the input stream without consuming it.
    #[inline]
    pub fn peek(&self) -> char {
        self.as_str().chars().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    #[inline]
    pub fn peek_next(&self) -> char {
        let mut iter = self.as_str().chars();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the third symbol from the input stream without consuming it.
    #[inline]
    pub fn peek_next_next(&self) -> char {
        let mut iter = self.as_str().chars();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Checks if there is nothing more to consume.
    #[inline]
    pub fn is_end(&self) -> bool {
        self.pos >= self.file.text().len()
    }

    /// Gets the amount of already consumed symbols.
    #[inline]
    pub fn get_pos_within_token(&self) -> u32 {
        (self.pos - self.token_start) as u32
    }

    /// Resets the number of bytes consumed to 0.
    #[inline]
    pub fn reset_pos_within_token(&mut self) {
        self.token_start = self.pos;
    }

    /// Moves to the next character.
    pub fn eat(&mut self) -> Option<char> {
        let c = self.as_str().chars().next()?;
        self.pos += c.len_utf8();
        self.prev = c;
        Some(c)
    }

    /// Advance by a known run of ascii bytes.
    #[inline]
    pub(super) fn advance_ascii_bytes(&mut self, count: usize, last_byte: u8) {
        if count == 0 {
            return;
        }

        debug_assert!(
            last_byte.is_ascii(),
            "advance_ascii_bytes expects ascii last byte"
        );
        self.pos += count;
        self.prev = last_byte as char;
    }

    /// Snapshot lexer state for speculative parsing.
    #[inline]
    pub fn snapshot(&self) -> LexerSnapshot {
        LexerSnapshot {
            pos: self.pos,
            token_start: self.token_start,
            prev: self.prev,
            trivia_mark: self.trivia.mark(),
            has_at: self.has_at,
            last_side_token_had_line_terminator: self.last_side_token_had_line_terminator,
            tokens_len: self.tokens.len(),
            side_tokens_len: self.side_tokens.len(),
            paren_stack: self.paren_stack.snapshot(),
            brace_stack: self.brace_stack.snapshot(),
            bracket_stack: self.bracket_stack.snapshot(),
            template_string_stack: self.options.template_string_stack.snapshot(),
            parentheses_depth: self.options.parentheses_depth,
            allow_tree_literals: self.options.allow_tree_literals,
            in_tree_attribute_value: self.options.in_tree_attribute_value,
            pending_line_terminator_before_next: self.pending_line_terminator_before_next,
            pending_comment_before_next: self.pending_comment_before_next,
            semantic_newline_token_count: self.semantic_newline_token_count,
            attachable_semantic_token_count: self.attachable_semantic_token_count,
            is_finished: self.is_finished,
            eof_token: self.eof_token,
        }
    }

    /// Restore lexer state from a snapshot.
    #[inline]
    pub fn restore(&mut self, snapshot: LexerSnapshot) {
        self.pos = snapshot.pos;
        self.token_start = snapshot.token_start;
        self.prev = snapshot.prev;
        self.options
            .template_string_stack
            .restore(snapshot.template_string_stack);
        self.options.parentheses_depth = snapshot.parentheses_depth;
        self.options.allow_tree_literals = snapshot.allow_tree_literals;
        self.options.in_tree_attribute_value = snapshot.in_tree_attribute_value;
        self.trivia.restore(snapshot.trivia_mark);
        self.has_at = snapshot.has_at;
        self.last_side_token_had_line_terminator = snapshot.last_side_token_had_line_terminator;
        let old_tokens_len = self.tokens.len();

        // clear delimiter matches introduced by the truncated suffix
        for removed_index in (snapshot.tokens_len..old_tokens_len).rev() {
            let removed_token = self.tokens[removed_index];
            let removed_metadata = self.token_data[removed_index];

            if matches!(
                removed_token.token.ty,
                TokenType::CloseParenthesis | TokenType::CloseBrace | TokenType::CloseBracket
            ) {
                let matching_open = removed_metadata.matching_pair as usize;
                if matching_open < snapshot.tokens_len {
                    self.token_data[matching_open].matching_pair = u32::MAX;
                }
            }
        }

        self.tokens.truncate(snapshot.tokens_len);
        self.side_tokens.truncate(snapshot.side_tokens_len);
        self.token_data.truncate(snapshot.tokens_len);
        self.paren_stack.restore(snapshot.paren_stack);
        self.brace_stack.restore(snapshot.brace_stack);
        self.bracket_stack.restore(snapshot.bracket_stack);
        self.pending_line_terminator_before_next = snapshot.pending_line_terminator_before_next;
        self.pending_comment_before_next = snapshot.pending_comment_before_next;
        self.semantic_newline_token_count = snapshot.semantic_newline_token_count;
        self.attachable_semantic_token_count = snapshot.attachable_semantic_token_count;
        self.is_finished = snapshot.is_finished;
        self.eof_token = snapshot.eof_token;
    }

    /// Reset the lexer cursor after one current-token re-lex.
    pub(crate) fn reset_cursor_after_re_lex(&mut self, end: usize) {
        self.pos = end;
        self.token_start = end;
        self.prev = self.file.text()[..end].chars().next_back().unwrap_or('\0');
    }

    /// Re-lex the current `/` or `/=` token as a regex literal.
    pub(crate) fn re_lex_as_regex(&mut self, current_token: TokenSpan) -> TokenSpan {
        let start = current_token.span.start as usize;
        self.pos = start + 1;
        self.token_start = start;
        self.prev = '/';

        let has_flags = self.eat_regex_string();
        let end = self.pos as u32;
        let token = Token::new(
            TokenType::Literal,
            end - current_token.span.start,
            Some(LiteralType::RegexString { has_flags }),
        );

        TokenSpan {
            token,
            span: Span {
                file: current_token.span.file,
                start: current_token.span.start,
                end,
            },
        }
    }

    /// Re-lex the current token as a TypeScript `<`.
    pub(crate) fn re_lex_as_typescript_l_angle(&mut self, current_token: TokenSpan) -> TokenSpan {
        let start = current_token.span.start as usize;
        self.pos = start + 1;
        self.token_start = self.pos;
        self.prev = '<';

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
        self.pos = start + 1;
        self.token_start = self.pos;
        self.prev = '>';

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

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // NOTE #Performance: rustc tried making optimized version of this for e.g. line comments,
        // but apparently LLVM inlines all this to fast iteration over bytes
        while predicate(self.peek()) && !self.is_end() {
            self.eat();
        }
    }

    /// Eats symbols until the first occurrence of the given byte is found.
    /// If the byte is not found, the entire string is consumed.
    #[inline]
    pub fn eat_until(&mut self, byte: u8) {
        debug_assert!(byte.is_ascii(), "eat_until requires ASCII needle: {byte}");
        let s = self.as_str();
        match memchr(byte, s.as_bytes()) {
            Some(idx) => {
                // idx is at a UTF-8 boundary because we only search ASCII bytes
                self.pos += idx;
            }
            None => {
                self.pos = self.file.text().len();
            }
        }
    }
}
