use std::sync::Arc;

use destack_source::{File, FileId, Span};

use memchr::memchr;

pub(super) const EOF_CHAR: char = '\0';

/// Byte cursor over source text during lexing.
#[derive(Debug)]
pub(super) struct Scanner {
    /// The source file.
    file: Arc<File>,
    /// The source file ID.
    file_id: FileId,
    /// The source byte pointer.
    source: *const u8,
    /// The source byte length.
    source_len: usize,
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
        let file_id = file.id;
        let source = file.text();
        let source_pointer = source.as_ptr();
        let source_len = source.len();

        Self {
            file,
            file_id,
            source: source_pointer,
            source_len,
            position: 0,
            token_start: 0,
            previous: EOF_CHAR,
        }
    }

    /// Return the source file ID.
    #[inline]
    pub(super) fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Return the source file.
    #[inline]
    pub(super) fn file(&self) -> &File {
        self.file.as_ref()
    }

    /// Return the current byte position.
    #[inline]
    pub(super) fn position(&self) -> usize {
        self.position
    }

    /// Move the scanner to one byte position.
    #[inline]
    pub(super) fn set_position(&mut self, position: usize) {
        debug_assert!(
            self.text().is_char_boundary(position),
            "scanner position must be a character boundary"
        );
        self.position = position;
        self.token_start = position;
        self.previous = EOF_CHAR;
    }

    /// Return the source text.
    #[inline]
    pub(super) fn text(&self) -> &str {
        self.file.text()
    }

    /// Return the remaining source text.
    #[inline]
    pub(super) fn remaining(&self) -> &str {
        let bytes = self.remaining_bytes();

        // scanner positions are maintained on UTF-8 boundaries
        unsafe { std::str::from_utf8_unchecked(bytes) }
    }

    /// Return the remaining source bytes.
    #[inline]
    pub(super) fn remaining_bytes(&self) -> &[u8] {
        if self.position >= self.source_len {
            return &[];
        }

        let length = self.source_len - self.position;

        // scanner owns an Arc<File>, so the cached source pointer stays valid
        unsafe { std::slice::from_raw_parts(self.source.add(self.position), length) }
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

    /// Peek one source byte without consuming it.
    #[inline]
    pub(super) fn byte(&self) -> u8 {
        if self.position >= self.source_len {
            return 0;
        }

        // scanner owns an Arc<File>, so the cached source pointer stays valid
        unsafe { *self.source.add(self.position) }
    }

    /// Peek one source byte at an offset from the current position.
    #[inline]
    pub(super) fn byte_at(&self, offset: usize) -> u8 {
        let position = self.position + offset;
        if position >= self.source_len {
            return 0;
        }

        // scanner owns an Arc<File>, so the cached source pointer stays valid
        unsafe { *self.source.add(position) }
    }

    /// Return whether there is no source left to consume.
    #[inline]
    pub(super) fn is_end(&self) -> bool {
        self.position >= self.source_len
    }

    /// Return the byte length consumed for the current token.
    #[inline]
    pub(super) fn token_len(&self) -> u32 {
        (self.position - self.token_start) as u32
    }

    /// Return the source bytes covered by the current token.
    #[inline]
    pub(super) fn token_bytes(&self) -> &[u8] {
        // scanner owns an Arc<File>, so the cached source pointer stays valid
        unsafe {
            std::slice::from_raw_parts(
                self.source.add(self.token_start),
                self.position - self.token_start,
            )
        }
    }

    /// Reset the current token start to the current position.
    #[inline]
    pub(super) fn reset_token_start(&mut self) {
        self.token_start = self.position;
    }

    /// Consume one UTF-8 character.
    pub(super) fn eat_char(&mut self) -> Option<char> {
        if self.position >= self.source_len {
            return None;
        }

        let byte = unsafe { *self.source.add(self.position) };
        if byte.is_ascii() {
            self.position += 1;
            self.previous = byte as char;
            return Some(self.previous);
        }

        let character = self.remaining().chars().next()?;
        self.position += character.len_utf8();
        self.previous = character;
        Some(character)
    }

    /// Advance by a known run of ASCII bytes.
    #[inline]
    pub(super) fn advance_ascii_bytes(&mut self, count: usize, last_byte: u8) {
        if count == 0 {
            return;
        }

        debug_assert!(
            last_byte.is_ascii(),
            "advance_ascii_bytes expects ASCII last byte"
        );
        self.position += count;
        self.previous = last_byte as char;
    }

    /// Advance by one known ASCII byte.
    #[inline(always)]
    pub(super) fn advance_ascii_byte(&mut self) {
        debug_assert!(self.position < self.source_len, "source byte must exist");
        let byte = unsafe { *self.source.add(self.position) };
        debug_assert!(byte.is_ascii(), "advance_ascii_byte expects ASCII");
        self.position += 1;
        self.previous = byte as char;
    }

    /// Advance by a known run of bytes.
    #[inline]
    pub(super) fn advance_bytes(&mut self, count: usize) {
        self.position += count;
        self.previous = EOF_CHAR;
    }

    /// Eat characters while a predicate returns true.
    pub(super) fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while !self.is_end() {
            let character = self.peek_char();
            if !predicate(character) {
                break;
            }

            self.eat_char();
        }
    }

    /// Peek the next UTF-8 character without consuming it.
    pub(super) fn peek_char(&self) -> char {
        if self.is_end() {
            return EOF_CHAR;
        }

        let byte = self.byte();
        if byte.is_ascii() {
            return byte as char;
        }

        self.remaining().chars().next().unwrap_or(EOF_CHAR)
    }

    /// Eat until an ASCII byte is found or EOF is reached.
    #[inline]
    pub(super) fn eat_until(&mut self, byte: u8) {
        debug_assert!(byte.is_ascii(), "eat_until requires ASCII needle: {byte}");
        match memchr(byte, self.remaining_bytes()) {
            Some(index) => {
                self.position += index;
            }
            None => {
                self.position = self.source_len;
            }
        }
    }
}
