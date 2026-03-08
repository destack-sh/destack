use std::fmt::Debug;
use std::sync::Arc;

use destack_ast::TokenSpan;
use destack_source::{File, FileId, LanguageType, Span};

use memchr::memchr;

/// Tree literal lexer state for contextual parsing (TSX-compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum TreeState {
    /// Normal code context (not in a tree literal).
    #[default]
    None,
    /// Inside a tree literal opening tag (after `<Identifier`, before `>` or `/>`).
    OpeningTag,
    /// Inside a tree literal closing tag (after `</`, before `>`).
    ClosingTag,
    /// Inside tree literal content (after `>`, before `</` or `{`).
    Content,
}

/// Entry tracking where a tree expression container started.
#[derive(Debug, Clone, Copy)]
pub(super) struct TreeExpressionEntry {
    /// The parentheses depth when this expression container started.
    pub parentheses_depth: i32,
    /// The tree state stack depth when this expression container started.
    pub tree_depth: usize,
    /// Whether this expression container came from Content mode (vs OpeningTag mode).
    /// When `}` closes this container, we only restore Content state if this is true.
    pub from_content: bool,
}

/// The options for the lexer.
#[derive(Debug, Default, Clone)]
pub(super) struct LexerOptions {
    /// The nested template strings starting parentheses depth stack.
    pub(super) template_string_stack: Vec<i32>,
    /// The depth of nested template string parentheses.
    pub(super) parentheses_depth: i32 = 0,
    /// Stack of tree literal states for nested tree elements (TSX-compatible).
    pub(super) tree_state_stack: Vec<TreeState>,
    /// The angle bracket depth within the current opening tag.
    pub(super) tree_tag_angle_depth: usize,
    /// Stack of entries tracking where tree expression containers started.
    /// When `}` is seen at the matching depth and tree level, we return to TreeState::Content.
    pub(super) tree_expression_stack: Vec<TreeExpressionEntry>,
    /// The last non-whitespace token for fast context lookups (excludes comments, includes newlines).
    /// Updated incrementally to avoid O(n) reverse scans.
    pub(super) last_non_whitespace_token: Option<TokenSpan>,
    /// The last semantic token (excludes whitespace, comments, and newlines).
    /// Used for tree literal context detection.
    pub(super) last_semantic_token: Option<TokenSpan>,
    /// The second-to-last semantic token (excludes whitespace, comments, and newlines).
    pub(super) prev_semantic_token: Option<TokenSpan>,
    /// The third-to-last semantic token (excludes whitespace, comments, and newlines).
    pub(super) prev_prev_semantic_token: Option<TokenSpan>,
    /// Stack marking whether an open parenthesis started a control statement header.
    pub(super) control_header_parenthesis_stack: Vec<bool>,
    /// Whether the last semantic close parenthesis ended a control header.
    pub(super) last_close_parenthesis_ends_control_header: bool,
    /// Whether tree literal lexing is allowed in the current context.
    pub(super) allow_tree_literals: bool,
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
    /// Whether an `@` token was seen.
    pub(super) has_at: bool,
    /// Whether the most recent side token contained a line terminator.
    pub(super) last_side_token_had_line_terminator: bool,
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

/// Snapshot of lexer state for speculative parsing.
#[derive(Debug, Clone)]
pub struct LexerSnapshot {
    /// The byte position of the lexer head.
    pub(super) pos: usize,
    /// The byte position where the current token started.
    pub(super) token_start: usize,
    /// The most recently consumed character.
    pub(super) prev: char,
    /// The snapshot of lexer options and stacks.
    pub(super) options: LexerOptions,
    /// Whether an `@` token has been observed.
    pub(super) has_at: bool,
    /// Whether the most recent side token at snapshot time had a line terminator.
    pub(super) last_side_token_had_line_terminator: bool,
}

impl Lexer {
    /// Create a new Lexer from a file.
    pub fn new(file: Arc<File>, language: LanguageType) -> Lexer {
        let file_id = file.id;
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
            has_at: false,
            last_side_token_had_line_terminator: false,
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
            options: self.options.clone(),
            has_at: self.has_at,
            last_side_token_had_line_terminator: self.last_side_token_had_line_terminator,
        }
    }

    /// Restore lexer state from a snapshot.
    #[inline]
    pub fn restore(&mut self, snapshot: LexerSnapshot) {
        self.pos = snapshot.pos;
        self.token_start = snapshot.token_start;
        self.prev = snapshot.prev;
        self.options = snapshot.options;
        self.has_at = snapshot.has_at;
        self.last_side_token_had_line_terminator = snapshot.last_side_token_had_line_terminator;
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

    /// Gets the current tree literal state (top of stack or None).
    #[inline]
    pub(super) fn tree_state(&self) -> TreeState {
        self.options
            .tree_state_stack
            .last()
            .copied()
            .unwrap_or(TreeState::None)
    }

    /// Pushes a new tree literal state onto the stack.
    #[inline]
    pub(super) fn push_tree_state(&mut self, state: TreeState) {
        if state == TreeState::OpeningTag {
            self.options.tree_tag_angle_depth = 0;
        }
        self.options.tree_state_stack.push(state);
    }

    /// Pops the current tree literal state from the stack.
    #[inline]
    pub(super) fn pop_tree_state(&mut self) -> TreeState {
        let state = self
            .options
            .tree_state_stack
            .pop()
            .unwrap_or(TreeState::None);
        if state == TreeState::OpeningTag {
            self.options.tree_tag_angle_depth = 0;
        }
        state
    }

    /// Checks if we're currently inside tree literal content.
    /// Returns false if we're inside a tree expression container (after `{`).
    #[inline]
    pub(super) fn in_tree_content(&self) -> bool {
        // require content mode
        if self.tree_state() != TreeState::Content {
            return false;
        }

        // check for an active tree expression container
        if let Some(entry) = self.options.tree_expression_stack.last() {
            // treat deeper tree levels as tree content
            let current_tree_depth = self.options.tree_state_stack.len();
            if current_tree_depth > entry.tree_depth {
                return true;
            }

            // treat same-level expression containers as code
            if self.options.parentheses_depth > entry.parentheses_depth {
                return false;
            }
        }

        true
    }

    /// Checks if we're inside any tree expression container at the current tree depth.
    #[inline]
    pub(super) fn in_tree_expression_container(&self) -> bool {
        if let Some(entry) = self.options.tree_expression_stack.last() {
            entry.tree_depth == self.options.tree_state_stack.len()
        } else {
            false
        }
    }
}
