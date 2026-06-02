use std::str::FromStr;

use super::lexer::Lexer;
use destack_dir::{Keyword, TokenLiteral, TokenType, is_identifier_continue, is_identifier_start};
use destack_unicode::UnicodeEmoji;

/// Return true when a byte can continue an ascii identifier.
#[inline]
fn is_ascii_identifier_continue_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

impl Lexer {
    /// Eat ascii identifier continuation bytes.
    #[inline]
    fn eat_ascii_identifier_continue(&mut self) {
        let bytes = self.remaining_text().as_bytes();
        let mut index = 0usize;
        while index < bytes.len() && is_ascii_identifier_continue_byte(bytes[index]) {
            index += 1;
        }

        if index > 0 {
            self.advance_ascii_bytes(index, bytes[index - 1]);
        }
    }

    /// Parse an identifier-like token after its first character.
    ///
    /// Returns the token kind and optional literal when the identifier is a
    /// built-in literal.
    pub(super) fn eat_identifier_like(
        &mut self,
        first_char: char,
    ) -> (TokenType, Option<TokenLiteral>) {
        debug_assert!(is_identifier_start(first_char));
        let start_position = self.position();

        // fast path: ascii identifier tails dominate script sources
        if first_char.is_ascii() {
            // consume mixed ascii and unicode identifier tails without char by char ascii scans
            loop {
                self.eat_ascii_identifier_continue();

                if self.is_end() {
                    break;
                }

                let current = self.peek();
                if !current.is_ascii() && is_identifier_continue(current) {
                    self.eat();
                    continue;
                }

                break;
            }
        } else {
            // unicode continuation tail
            self.eat_while(is_identifier_continue);
        }

        // check for unicode escapes mid-identifier (e.g., `AB\u{43}`)
        // only consume escapes that decode to identifier continuations
        if self.peek() == '\\'
            && self.peek_next() == 'u'
            && self.next_unicode_escape_continues_identifier()
        {
            self.eat_identifier_with_unicode_escapes();
            return (TokenType::Identifier, None);
        }
        // known prefixes must have been handled earlier
        match self.peek() {
            '#' => return (TokenType::UnknownLiteralPrefix, None),
            c if !c.is_ascii() && c.is_emoji_char() => {
                return (self.eat_invalid_identifier(), None);
            }
            _ => {}
        }
        // boolean
        let source = self.source_text();
        if first_char == 't' && source[start_position - 1..self.position()].eq("true") {
            (
                TokenType::Literal,
                Some(TokenLiteral::Boolean { value: true }),
            )
        }
        // false
        else if first_char == 'f' && source[start_position - 1..self.position()].eq("false") {
            (
                TokenType::Literal,
                Some(TokenLiteral::Boolean { value: false }),
            )
        }
        // just an identifier
        else {
            (TokenType::Identifier, None)
        }
    }

    /// Parse an invalid identifier after its first character.
    pub(super) fn eat_invalid_identifier(&mut self) -> TokenType {
        // start is already eaten, eat the rest of identifier
        self.eat_while(|c| {
            const ZERO_WIDTH_JOINER: char = '\u{200d}';
            is_identifier_continue(c)
                || (!c.is_ascii() && c.is_emoji_char())
                || c == ZERO_WIDTH_JOINER
        });
        TokenType::InvalidIdentifier
    }

    /// Try to parse an identifier that starts with a unicode escape.
    ///
    /// The leading `\` has already been consumed.
    pub(super) fn try_eat_unicode_escape_identifier(
        &mut self,
    ) -> Option<(TokenType, Option<TokenLiteral>)> {
        // check for \u
        if self.peek() != 'u' {
            return None;
        }
        self.eat(); // eat 'u'

        // parse and validate the escaped code point for identifier-start
        let ch = self.eat_unicode_escape_char()?;
        if !is_identifier_start(ch) {
            return None;
        }

        // continue eating identifier (including more unicode escapes or regular chars)
        self.eat_identifier_with_unicode_escapes();

        Some((TokenType::Identifier, None))
    }

    /// Continue eating an identifier that may contain unicode escapes.
    fn eat_identifier_with_unicode_escapes(&mut self) {
        loop {
            let c = self.peek();
            if is_identifier_continue(c) {
                self.eat();
            } else if c == '\\' && self.peek_next() == 'u' {
                // validate the unicode escape before consuming it
                if !self.next_unicode_escape_continues_identifier() {
                    break;
                }
                self.eat(); // eat '\'
                self.eat(); // eat 'u'
                if self.eat_unicode_escape_char().is_none() {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Decode an identifier unicode escape body after `u`.
    ///
    /// Returns the decoded character and consumed byte count from the body.
    fn decode_identifier_unicode_escape_body(bytes: &[u8]) -> Option<(char, usize)> {
        let (value, consumed) = if bytes.first() == Some(&b'{') {
            let mut index = 1usize;
            let mut digit_count = 0usize;
            let mut value: u32 = 0;

            while let Some(&byte) = bytes.get(index) {
                if byte == b'}' {
                    break;
                }
                if !byte.is_ascii_hexdigit() {
                    return None;
                }

                let digit = (byte as char).to_digit(16)?;
                value = value.checked_mul(16)?.checked_add(digit)?;
                digit_count += 1;
                index += 1;
                if digit_count > 6 {
                    return None;
                }
            }

            if digit_count == 0 || bytes.get(index) != Some(&b'}') {
                return None;
            }

            (value, index + 1)
        } else {
            if bytes.len() < 4 {
                return None;
            }

            let mut value: u32 = 0;
            for &byte in &bytes[..4] {
                if !byte.is_ascii_hexdigit() {
                    return None;
                }
                let digit = (byte as char).to_digit(16)?;
                value = value.checked_mul(16)?.checked_add(digit)?;
            }

            (value, 4)
        };

        Some((char::from_u32(value)?, consumed))
    }

    /// Parse a unicode escape code point at the current `\u` tail.
    ///
    /// The lexer cursor must be positioned after `'u'` when this is called.
    fn eat_unicode_escape_char(&mut self) -> Option<char> {
        let bytes = self.remaining_text().as_bytes();
        let Some((decoded, consumed)) = Self::decode_identifier_unicode_escape_body(bytes) else {
            self.eat_invalid_unicode_escape_body_prefix();
            return None;
        };

        for _ in 0..consumed {
            self.eat();
        }

        Some(decoded)
    }

    /// Consume the maximal invalid identifier unicode escape prefix after `\u`.
    ///
    /// This preserves legacy lexer behavior where malformed escapes are grouped
    /// into a single unknown token prefix like `\u11`.
    pub(super) fn eat_invalid_unicode_escape_body_prefix(&mut self) {
        if self.peek() == '{' {
            self.eat(); // eat '{'
            let mut digit_count = 0usize;
            while self.peek().is_ascii_hexdigit() {
                self.eat();
                digit_count += 1;
                if digit_count > 6 {
                    break;
                }
            }
            return;
        }

        for _ in 0..4 {
            if !self.peek().is_ascii_hexdigit() {
                break;
            }
            self.eat();
        }
    }

    /// Decode a unicode escape sequence at the current position without consuming it.
    fn peek_unicode_escape_ahead_char(&self) -> Option<char> {
        let bytes = self.remaining_text().as_bytes();
        if bytes.len() < 2 || bytes[0] != b'\\' || bytes[1] != b'u' {
            return None;
        }

        let (decoded, _) = Self::decode_identifier_unicode_escape_body(&bytes[2..])?;
        Some(decoded)
    }

    /// Return whether the next unicode escape can continue an identifier.
    fn next_unicode_escape_continues_identifier(&self) -> bool {
        self.peek_unicode_escape_ahead_char()
            .is_some_and(is_identifier_continue)
    }
}

/// Return a keyword for an identifier when it can match keyword shape.
#[inline]
pub(crate) fn keyword_from_identifier(identifier: &str) -> Option<Keyword> {
    // reject lengths outside keyword bounds
    let bytes = identifier.as_bytes();
    let length = bytes.len();
    if !(2..=11).contains(&length) {
        return None;
    }

    // reject impossible length and first byte pairs
    let first = *bytes.first()?;
    let can_match_keyword = matches!(
        (length, first),
        (2, b'a' | b'd' | b'i' | b'o' | b't')
            | (3, b'a' | b'f' | b'g' | b'l' | b'n' | b's' | b't' | b'v')
            | (
                4,
                b'c' | b'e' | b'f' | b'g' | b'l' | b'm' | b'n' | b's' | b't' | b'v' | b'w'
            )
            | (
                5,
                b'a' | b'b'
                    | b'c'
                    | b'f'
                    | b'i'
                    | b'k'
                    | b'l'
                    | b'm'
                    | b'n'
                    | b's'
                    | b't'
                    | b'u'
                    | b'w'
                    | b'y'
            )
            | (6, b'a' | b'd' | b'e' | b'i' | b'p' | b'r' | b's' | b't')
            | (7, b'a' | b'd' | b'e' | b'f' | b'n' | b'p' | b'v')
            | (8, b'a' | b'c' | b'd' | b'f' | b'o' | b'p' | b'r')
            | (9, b'e' | b'i' | b'n' | b'p' | b's' | b'u')
            | (10, b'i')
            | (11, b'c')
    );
    if !can_match_keyword {
        return None;
    }

    Keyword::from_str(identifier).ok()
}
