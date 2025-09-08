use std::fmt::Debug;
use std::str::Chars;

use super::memchr::find_byte;

/// Tokenizer over a source string.
pub struct Tokenizer<'a> {
    /// The string to tokenize.
    pub str: &'a str,
    /// The current head ("next") byte position in the string.
    pub pos: usize,
    /// The number of bytes remaining in the current token.
    len_remaining: usize,
    /// The character iterator over the string.
    chars: Chars<'a>, // Chars is faster than a &str (according to rustc)
    /// The previous character.
    #[cfg(debug_assertions)]
    prev: char,
}

impl Debug for Tokenizer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Tokenizer {{ str: {}, pos: {}, len_remaining: {}, chars: {:?} }}>",
            self.str, self.pos, self.len_remaining, self.chars
        )
    }
}

pub const EOF_CHAR: char = '\0';

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer from a string.
    pub fn new(str: &'a str) -> Tokenizer<'a> {
        Tokenizer {
            str,
            pos: 0,
            len_remaining: str.len(),
            chars: str.chars(),
            #[cfg(debug_assertions)]
            prev: EOF_CHAR,
        }
    }

    /// Gets the underlying string.
    pub fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    /// Gets the last eaten symbol (or `'\0'` in release builds).
    /// (For debug assertions only.)
    pub fn prev(&self) -> char {
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
    pub fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Gets the amount of already consumed symbols.
    #[inline]
    pub fn get_pos_within_token(&self) -> u32 {
        (self.len_remaining - self.chars.as_str().len()) as u32
    }

    /// Resets the number of bytes consumed to 0.
    #[inline]
    pub fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    /// Moves to the next character.
    pub fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.pos = self.str.len() - self.chars.as_str().len();
        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }
        Some(c)
    }

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // NOTE: @Performance: rustc tried making optimized version of this for
        //  e.g., line comments, but apparently LLVM inlines all this to fast iteration over bytes.
        while predicate(self.peek()) && !self.is_eof() {
            self.bump();
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
                self.pos = self.str.len() - self.chars.as_str().len();
            }
            None => {
                self.chars = "".chars();
                self.pos = self.str.len();
            }
        }
    }
}
