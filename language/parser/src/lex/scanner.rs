use std::sync::Arc;

use destack_source::{File, FileId, Span};

use memchr::memchr;

pub(super) const EOF_CHAR: char = '\0';

/// Cursor over source text during lexing.
#[derive(Debug)]
pub(super) struct Scanner {
    /// The source file.
    file: Arc<File>,
    /// The current head byte position in the source text.
    position: usize,
    /// The byte position where the current token started.
    token_start: usize,
    /// The most recently consumed character.
    previous: char,
}

impl Scanner {
    /// Create a scanner for one file.
    pub(super) fn new(file: Arc<File>) -> Self {
        Self {
            file,
            position: 0,
            token_start: 0,
            previous: EOF_CHAR,
        }
    }

    /// Return the source file ID.
    #[inline]
    pub(super) fn file_id(&self) -> FileId {
        self.file.id
    }

    /// Return the current byte position.
    #[inline]
    pub(super) fn position(&self) -> usize {
        self.position
    }

    /// Return the source text.
    #[inline]
    pub(super) fn text(&self) -> &str {
        self.file.text()
    }

    /// Return the remaining source text.
    #[inline]
    pub(super) fn remaining(&self) -> &str {
        let source = self.text();
        if self.position >= source.len() {
            return "";
        }

        &source[self.position..]
    }

    /// Return source text covered by one span.
    #[inline]
    pub(super) fn span_str(&self, span: Span) -> &str {
        self.file.span_str(span)
    }

    /// Return the most recently consumed character.
    #[inline]
    pub(super) fn previous(&self) -> char {
        self.previous
    }

    /// Peek the next character without consuming it.
    #[inline]
    pub(super) fn peek(&self) -> char {
        self.remaining().chars().next().unwrap_or(EOF_CHAR)
    }

    /// Peek the second character without consuming it.
    #[inline]
    pub(super) fn peek_next(&self) -> char {
        let mut iter = self.remaining().chars();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peek the third character without consuming it.
    #[inline]
    pub(super) fn peek_next_next(&self) -> char {
        let mut iter = self.remaining().chars();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Return whether there is no source left to consume.
    #[inline]
    pub(super) fn is_end(&self) -> bool {
        self.position >= self.text().len()
    }

    /// Return the byte length consumed for the current token.
    #[inline]
    pub(super) fn token_len(&self) -> u32 {
        (self.position - self.token_start) as u32
    }

    /// Reset the current token start to the current position.
    #[inline]
    pub(super) fn reset_token_start(&mut self) {
        self.token_start = self.position;
    }

    /// Consume the next character.
    pub(super) fn eat(&mut self) -> Option<char> {
        let character = self.remaining().chars().next()?;
        self.position += character.len_utf8();
        self.previous = character;
        Some(character)
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
        self.position += count;
        self.previous = last_byte as char;
    }

    /// Eat characters while a predicate returns true.
    pub(super) fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // NOTE #Performance: rustc tried making optimized version of this for e.g. line comments,
        // but apparently LLVM inlines all this to fast iteration over bytes
        while predicate(self.peek()) && !self.is_end() {
            self.eat();
        }
    }

    /// Eat until an ASCII byte is found or EOF is reached.
    #[inline]
    pub(super) fn eat_until(&mut self, byte: u8) {
        debug_assert!(byte.is_ascii(), "eat_until requires ASCII needle: {byte}");
        let source = self.remaining();
        match memchr(byte, source.as_bytes()) {
            Some(index) => {
                // index is at a UTF-8 boundary because we only search ASCII bytes
                self.position += index;
            }
            None => {
                self.position = self.text().len();
            }
        }
    }
}
