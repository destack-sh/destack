use std::fmt::Debug;
use std::str::Chars;

use crate::memchr::find_byte;

/// Peekable iterator over a char sequence.
pub struct Tokenizer<'a> {
    len_remaining: usize,
    chars: Chars<'a>, // Chars is faster than a &str (according to rustc)
    #[cfg(debug_assertions)]
    prev: char,
}

impl Debug for Tokenizer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Tokenizer {{ len_remaining: {}, chars: {:?} }}>",
            self.len_remaining, self.chars
        )
    }
}

pub const EOF_CHAR: char = '\0';

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer from a string.
    pub(crate) fn new(input: &'a str) -> Tokenizer<'a> {
        Tokenizer {
            len_remaining: input.len(),
            chars: input.chars(),
            #[cfg(debug_assertions)]
            prev: EOF_CHAR,
        }
    }

    /// Gets the underlying string.
    pub(crate) fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    /// Gets the last eaten symbol (or `'\0'` in release builds).
    /// (For debug assertions only.)
    pub(crate) fn prev(&self) -> char {
        #[cfg(debug_assertions)]
        {
            self.prev
        }
        #[cfg(not(debug_assertions))]
        {
            EOF_CHAR
        }
    }

    /// Peeks the next symbol from the input stream without consuming it.
    pub(crate) fn peek_next(&self) -> char {
        // NOTE: @Performance: `.next()` optimizes better than `.nth(0)`
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    pub(crate) fn peek_next_next(&self) -> char {
        // NOTE: @Performance: `.next()` optimizes better than `.nth(1)`
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Checks if there is nothing more to consume.
    pub(crate) fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Gets the amount of already consumed symbols.
    #[inline]
    pub(crate) fn pos_within_token(&self) -> u32 {
        (self.len_remaining - self.chars.as_str().len()) as u32
    }

    /// Resets the number of bytes consumed to 0.
    #[inline]
    pub(crate) fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    /// Moves to the next character.
    pub(crate) fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }
        Some(c)
    }

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub(crate) fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // NOTE: @Performance: rustc tried making optimized version of this for
        //  e.g., line comments, but apparently LLVM inlines all this to fast iteration over bytes.
        while predicate(self.peek_next()) && !self.is_eof() {
            self.bump();
        }
    }

    /// Eats symbols until the first occurrence of the given byte is found.
    /// If the byte is not found, the entire string is consumed.
    #[inline]
    pub(crate) fn eat_until(&mut self, byte: u8) {
        debug_assert!(byte.is_ascii(), "eat_until requires ASCII needle: {byte}");
        let s = self.as_str();
        let bytes = s.as_bytes();
        match find_byte(bytes, byte) {
            Some(idx) => {
                // idx is at a UTF-8 boundary because we only search ASCII bytes
                self.chars = s[idx..].chars();
            }
            None => {
                self.chars = "".chars();
            }
        }
    }
}
