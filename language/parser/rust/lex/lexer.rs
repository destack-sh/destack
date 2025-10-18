use std::fmt::Debug;
use std::str::Chars;

use dyst_source::Span;

use super::memchr::find_byte;

/// Lexer over a source string.
pub struct Lexer<'a> {
    /// The string to tokenize.
    pub str: &'a str,
    /// The current head ("next") byte position in the string.
    pub pos: usize,
    /// The number of bytes remaining in the current token.
    len_remaining: usize,
    /// The character iterator over the string.
    chars: Chars<'a>, // Chars is faster than a &str (according to rustc)
    /// The previous character.
    prev: char,
}

impl Debug for Lexer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Lexer {{ str: {}, pos: {}, len_remaining: {}, chars: {:?} }}>",
            self.str, self.pos, self.len_remaining, self.chars
        )
    }
}

pub const EOF_CHAR: char = '\0';

impl<'a> Lexer<'a> {
    /// Create a new Lexer from a string.
    pub fn new(str: &'a str) -> Lexer<'a> {
        Lexer {
            str,
            pos: 0,
            len_remaining: str.len(),
            chars: str.chars(),
            prev: EOF_CHAR,
        }
    }

    /// Gets the underlying string.
    pub fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    /// Gets the string content of a span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'a str {
        &self.str[span.start as usize..span.end as usize]
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
        (self.len_remaining - self.chars.as_str().len()) as u32
    }

    /// Resets the number of bytes consumed to 0.
    #[inline]
    pub fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    /// Moves to the next character.
    pub fn eat(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.pos = self.str.len() - self.chars.as_str().len();
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
                self.pos = self.str.len() - self.chars.as_str().len();
            }
            None => {
                self.chars = "".chars();
                self.pos = self.str.len();
            }
        }
    }
}
