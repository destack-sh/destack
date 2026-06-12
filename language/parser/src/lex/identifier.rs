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
        let bytes = self.scanner.remaining_bytes();
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
        // consume ascii identifier tails in bulk
        if first_char.is_ascii() {
            // continue through mixed ascii and unicode identifier tails
            loop {
                self.eat_ascii_identifier_continue();

                if self.is_end() {
                    break;
                }

                let byte = self.scanner.byte();
                if byte.is_ascii() {
                    break;
                }

                let current = self.scanner.peek_char();
                if is_identifier_continue(current) {
                    self.scanner.eat_char();
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
        if self.scanner.byte() == b'\\'
            && self.scanner.byte_at(1) == b'u'
            && self.next_unicode_escape_continues_identifier()
        {
            self.eat_identifier_with_unicode_escapes();
            return (TokenType::Identifier, None);
        }
        // known prefixes must have been handled earlier
        match self.scanner.byte() {
            b'#' => return (TokenType::UnknownLiteralPrefix, None),
            byte if !byte.is_ascii() && self.scanner.peek_char().is_emoji_char() => {
                return (self.eat_invalid_identifier(), None);
            }
            _ => {}
        }
        // boolean
        let bytes = self.token_bytes();
        if first_char == 't' && bytes == b"true" {
            (
                TokenType::Literal,
                Some(TokenLiteral::Boolean { value: true }),
            )
        }
        // false
        else if first_char == 'f' && bytes == b"false" {
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
        if self.scanner.byte() != b'u' {
            return None;
        }
        self.scanner.advance_ascii_byte();

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
            self.eat_ascii_identifier_continue();

            let byte = self.scanner.byte();
            if byte == b'\\' && self.scanner.byte_at(1) == b'u' {
                // validate the unicode escape before consuming it
                if !self.next_unicode_escape_continues_identifier() {
                    break;
                }
                self.scanner.advance_ascii_byte();
                self.scanner.advance_ascii_byte();
                if self.eat_unicode_escape_char().is_none() {
                    break;
                }
            } else if byte.is_ascii() {
                break;
            } else {
                let c = self.scanner.peek_char();
                if !is_identifier_continue(c) {
                    break;
                }
                self.scanner.eat_char();
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

                let digit = if byte.is_ascii_digit() {
                    u32::from(byte - b'0')
                } else if (b'a'..=b'f').contains(&byte) {
                    u32::from(byte - b'a') + 10
                } else {
                    u32::from(byte - b'A') + 10
                };
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
                let digit = if byte.is_ascii_digit() {
                    u32::from(byte - b'0')
                } else if (b'a'..=b'f').contains(&byte) {
                    u32::from(byte - b'a') + 10
                } else {
                    u32::from(byte - b'A') + 10
                };
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
        let bytes = self.scanner.remaining_bytes();
        let Some((decoded, consumed)) = Self::decode_identifier_unicode_escape_body(bytes) else {
            self.eat_invalid_unicode_escape_body_prefix();
            return None;
        };

        if consumed > 0 {
            self.scanner
                .advance_ascii_bytes(consumed, bytes[consumed - 1]);
        }

        Some(decoded)
    }

    /// Consume the maximal invalid identifier unicode escape prefix after `\u`.
    ///
    /// This preserves legacy lexer behavior where malformed escapes are grouped
    /// into a single unknown token prefix like `\u11`.
    pub(super) fn eat_invalid_unicode_escape_body_prefix(&mut self) {
        if self.scanner.byte() == b'{' {
            self.scanner.advance_ascii_byte();
            let mut digit_count = 0usize;
            while self.scanner.byte().is_ascii_hexdigit() {
                self.scanner.advance_ascii_byte();
                digit_count += 1;
                if digit_count > 6 {
                    break;
                }
            }
            return;
        }

        for _ in 0..4 {
            if !self.scanner.byte().is_ascii_hexdigit() {
                break;
            }
            self.scanner.advance_ascii_byte();
        }
    }

    /// Decode a unicode escape sequence at the current position without consuming it.
    fn peek_unicode_escape_ahead_char(&self) -> Option<char> {
        let bytes = self.scanner.remaining_bytes();
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
    let bytes = identifier.as_bytes();
    keyword_from_identifier_bytes(bytes)
}

/// Return a keyword for identifier bytes when they can match keyword shape.
#[inline]
pub(crate) fn keyword_from_identifier_bytes(bytes: &[u8]) -> Option<Keyword> {
    let length = bytes.len();
    if !(2..=11).contains(&length) {
        return None;
    }

    let first = bytes[0];
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

    match bytes {
        b"abstract" => Some(Keyword::Abstract),
        b"accessor" => Some(Keyword::Accessor),
        b"any" => Some(Keyword::Any),
        b"as" => Some(Keyword::As),
        b"async" => Some(Keyword::Async),
        b"await" => Some(Keyword::Await),
        b"break" => Some(Keyword::Break),
        b"case" => Some(Keyword::Case),
        b"catch" => Some(Keyword::Catch),
        b"class" => Some(Keyword::Class),
        b"comptime" => Some(Keyword::Comptime),
        b"const" => Some(Keyword::Const),
        b"constructor" => Some(Keyword::Constructor),
        b"continue" => Some(Keyword::Continue),
        b"debugger" => Some(Keyword::Debugger),
        b"declare" => Some(Keyword::Declare),
        b"default" => Some(Keyword::Default),
        b"do" => Some(Keyword::Do),
        b"else" => Some(Keyword::Else),
        b"enum" => Some(Keyword::Enum),
        b"exclusive" => Some(Keyword::Exclusive),
        b"export" => Some(Keyword::Export),
        b"extends" => Some(Keyword::Extends),
        b"extension" => Some(Keyword::Extension),
        b"false" => None,
        b"final" => Some(Keyword::Final),
        b"finally" => Some(Keyword::Finally),
        b"for" => Some(Keyword::For),
        b"from" => Some(Keyword::From),
        b"function" => Some(Keyword::Function),
        b"get" => Some(Keyword::Get),
        b"if" => Some(Keyword::If),
        b"implements" => Some(Keyword::Implements),
        b"import" => Some(Keyword::Import),
        b"in" => Some(Keyword::In),
        b"infer" => Some(Keyword::Infer),
        b"interface" => Some(Keyword::Interface),
        b"instanceof" => Some(Keyword::InstanceOf),
        b"is" => Some(Keyword::Is),
        b"keyof" => Some(Keyword::Keyof),
        b"let" => Some(Keyword::Let),
        b"local" => Some(Keyword::Local),
        b"loop" => Some(Keyword::Loop),
        b"match" => Some(Keyword::Match),
        b"move" => Some(Keyword::Move),
        b"never" => Some(Keyword::Never),
        b"new" => Some(Keyword::New),
        b"newtype" => Some(Keyword::Newtype),
        b"null" => Some(Keyword::Null),
        b"of" => Some(Keyword::Of),
        b"override" => Some(Keyword::Override),
        b"package" => Some(Keyword::Package),
        b"private" => Some(Keyword::Private),
        b"protected" => Some(Keyword::Protected),
        b"public" => Some(Keyword::Public),
        b"readonly" => Some(Keyword::Readonly),
        b"return" => Some(Keyword::Return),
        b"satisfies" => Some(Keyword::Satisfies),
        b"set" => Some(Keyword::Set),
        b"shared" => Some(Keyword::Shared),
        b"static" => Some(Keyword::Static),
        b"struct" => Some(Keyword::Struct),
        b"super" => Some(Keyword::Super),
        b"switch" => Some(Keyword::Switch),
        b"this" => Some(Keyword::This),
        b"throw" => Some(Keyword::Throw),
        b"true" => None,
        b"try" => Some(Keyword::Try),
        b"type" => Some(Keyword::Type),
        b"typeof" => Some(Keyword::Typeof),
        b"undefined" => Some(Keyword::Undefined),
        b"using" => Some(Keyword::Using),
        b"virtual" => Some(Keyword::Virtual),
        b"void" => Some(Keyword::Void),
        b"where" => Some(Keyword::Where),
        b"while" => Some(Keyword::While),
        b"with" => Some(Keyword::With),
        b"yield" => Some(Keyword::Yield),
        _ => None,
    }
}
