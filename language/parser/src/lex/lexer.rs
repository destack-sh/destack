use std::fmt::Debug;
use std::str::Chars;

use destack_ast::TokenSpan;
use destack_source::{FileId, LanguageType, Span};

use super::memchr::find_byte;

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
    /// Stack of entries tracking where tree expression containers started.
    /// When `}` is seen at the matching depth and tree level, we return to TreeState::Content.
    pub(super) tree_expression_stack: Vec<TreeExpressionEntry>,
    /// The last non-whitespace token for fast context lookups (includes newlines).
    /// Updated incrementally to avoid O(n) reverse scans.
    pub(super) last_non_whitespace_token: Option<TokenSpan>,
    /// The last semantic token (excludes both whitespace and newlines).
    /// Used for tree literal context detection.
    pub(super) last_semantic_token: Option<TokenSpan>,
}

/// Lexer over a source string.
pub struct Lexer<'a> {
    /// The source ID.
    pub file_id: FileId,
    /// The string to tokenize.
    pub source: &'a str,
    /// The character iterator over the string.
    chars: Chars<'a>, // Chars is faster than a &str (according to rustc)

    /// The current head ("next") byte position in the string.
    pub(super) pos: usize,
    /// The options for the lexer.
    pub(super) options: LexerOptions,
    /// The number of bytes remaining in the current token.
    len_remaining_in_token: usize,
    /// The previous character.
    prev: char,
    /// The tokens seen so far.
    pub(super) tokens: Vec<TokenSpan>,

    /// The language type for parsing behavior.
    #[allow(unused)]
    pub(super) language: LanguageType,
}

impl Debug for Lexer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Lexer {{ source: {}, pos: {} }}>",
            self.source, self.pos
        )
    }
}

pub const EOF_CHAR: char = '\0';

impl<'a> Lexer<'a> {
    /// Create a new Lexer from a string.
    pub fn new(file_id: FileId, source: &'a str, language: LanguageType) -> Lexer<'a> {
        // estimate ~8 bytes per token on average for capacity hint
        let estimated_tokens = source.len() / 8;
        Lexer {
            file_id,
            source,
            pos: 0,
            options: LexerOptions::default(),
            len_remaining_in_token: source.len(),
            chars: source.chars(),
            prev: EOF_CHAR,
            tokens: Vec::with_capacity(estimated_tokens),
            language,
        }
    }

    /// Gets the underlying string.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    /// Gets the string content of a span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'a str {
        &self.source[span.start as usize..span.end as usize]
    }

    /// Gets the last eaten symbol (or `'\0'` in release builds).
    #[inline]
    pub fn prev(&self) -> char {
        self.prev
    }

    /// Peeks the next symbol from the input stream without consuming it.
    #[inline]
    pub fn peek(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    #[inline]
    pub fn peek_next(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the third symbol from the input stream without consuming it.
    #[inline]
    pub fn peek_next_next(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Checks if there is nothing more to consume.
    #[inline]
    pub fn is_end(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Gets the amount of already consumed symbols.
    #[inline]
    pub fn get_pos_within_token(&self) -> u32 {
        (self.len_remaining_in_token - self.chars.as_str().len()) as u32
    }

    /// Resets the number of bytes consumed to 0.
    #[inline]
    pub fn reset_pos_within_token(&mut self) {
        self.len_remaining_in_token = self.chars.as_str().len();
    }

    /// Moves to the next character.
    pub fn eat(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.pos = self.source.len() - self.chars.as_str().len();
        self.prev = c;
        Some(c)
    }

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // NOTE: #Performance: rustc tried making optimized version of this for
        //  e.g., line comments, but apparently LLVM inlines all this to fast iteration over bytes.
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
        let bytes = s.as_bytes();
        match find_byte(bytes, byte) {
            Some(idx) => {
                // idx is at a UTF-8 boundary because we only search ASCII bytes
                self.chars = s[idx..].chars();
                self.pos = self.source.len() - self.chars.as_str().len();
            }
            None => {
                self.chars = "".chars();
                self.pos = self.source.len();
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
        self.options.tree_state_stack.push(state);
    }

    /// Pops the current tree literal state from the stack.
    #[inline]
    pub(super) fn pop_tree_state(&mut self) -> TreeState {
        self.options
            .tree_state_stack
            .pop()
            .unwrap_or(TreeState::None)
    }

    /// Checks if we're currently inside tree literal content.
    /// Returns false if we're inside a tree expression container (after `{`).
    #[inline]
    pub(super) fn in_tree_content(&self) -> bool {
        // not in content mode at all
        if self.tree_state() != TreeState::Content {
            return false;
        }
        // check if we're inside a tree expression container
        // if so, we're not in "true" content mode (we're lexing code)
        if let Some(entry) = self.options.tree_expression_stack.last() {
            // If we're at a deeper tree level than when the expression container started,
            // we're in a nested tree and should lex tree content
            let current_tree_depth = self.options.tree_state_stack.len();
            if current_tree_depth > entry.tree_depth {
                return true;
            }
            // We're at the same tree level, so check if we're inside the expression
            if self.options.parentheses_depth > entry.parentheses_depth {
                return false;
            }
        }
        true
    }
}
