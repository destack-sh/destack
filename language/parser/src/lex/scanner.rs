use tspp_source::File;

use memchr::memchr;

pub(super) const EOF_CHAR: char = '\0';

/// Byte cursor over source text during lexing.
#[derive(Debug)]
pub(super) struct Scanner {
    /// The source byte pointer.
    source: *const u8,
    /// The source byte length.
    source_len: usize,
    /// The current head byte position in the source text.
    position: usize,
    /// The byte position where the current token started.
    token_start: usize,
}

impl Scanner {
    /// Create a scanner at one source byte position.
    pub(super) fn at(file: &File, position: u32) -> Self {
        let source = file.text();
        let source_pointer = source.as_ptr();
        let source_len = source.len();
        let position = position as usize;
        debug_assert!(position <= source_len, "scanner position exceeds source");
        debug_assert!(
            source.is_char_boundary(position),
            "scanner position splits UTF-8"
        );

        Self {
            source: source_pointer,
            source_len,
            position,
            token_start: position,
        }
    }

    /// Return the current byte position.
    #[inline]
    pub(super) fn position(&self) -> usize {
        self.position
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

        // the owning tokenizer keeps the cached source pointer valid
        unsafe { std::slice::from_raw_parts(self.source.add(self.position), length) }
    }

    /// Peek one source byte without consuming it.
    #[inline]
    pub(super) fn peek_byte(&self) -> u8 {
        if self.position >= self.source_len {
            return 0;
        }

        // the owning tokenizer keeps the cached source pointer valid
        unsafe { *self.source.add(self.position) }
    }

    /// Peek one source byte at an offset from the current position.
    #[inline]
    pub(super) fn peek_byte_at(&self, offset: usize) -> u8 {
        let position = self.position + offset;
        if position >= self.source_len {
            return 0;
        }

        // the owning tokenizer keeps the cached source pointer valid
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
        // the owning tokenizer keeps the cached source pointer valid
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

            return Some(byte as char);
        }

        let character = self.remaining().chars().next()?;
        self.position += character.len_utf8();

        Some(character)
    }

    /// Advance by a known run of ASCII bytes.
    #[inline]
    pub(super) fn advance_ascii_bytes(&mut self, count: usize) {
        debug_assert!(count > 0, "ASCII byte run must not be empty");
        debug_assert!(
            count <= self.source_len - self.position,
            "ASCII byte run exceeds source"
        );

        self.position += count;
    }

    /// Advance by one known ASCII byte.
    #[inline(always)]
    pub(super) fn advance_ascii_byte(&mut self) {
        debug_assert!(self.position < self.source_len, "source byte must exist");
        let byte = unsafe { *self.source.add(self.position) };
        debug_assert!(byte.is_ascii(), "advance_ascii_byte expects ASCII");
        self.position += 1;
    }

    /// Advance by a known run of bytes.
    #[inline]
    pub(super) fn advance_bytes(&mut self, count: usize) {
        debug_assert!(
            count <= self.source_len - self.position,
            "byte run exceeds source"
        );
        self.position += count;
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

        let byte = self.peek_byte();
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
