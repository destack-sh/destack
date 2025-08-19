//! Low-level general purpose Descript lexer (adapted from rustc).

use crate::DocPosition;
use crate::token::{LiteralTokenType, NumberBase, RawStringError, Token, TokenType};
use crate::tokenizer::{EOF_CHAR, Tokenizer};
use destack_unicode::UnicodeEmoji;
use destack_unicode::xid::UnicodeXID;

pub fn strip_shebang(input: &str) -> Option<usize> {
    // shebang must start with `#!` literally, without any preceding whitespace
    if let Some(input_tail) = input.strip_prefix("#!") {
        return Some(2 + input_tail.lines().next().unwrap_or_default().len());
    }
    None
}

/// Validates a raw string literal. Used for getting more information about a
/// problem with a `RawStr`/`RawByteStr` with a `None` field.
#[inline]
pub fn validate_raw_str(input: &str, prefix_len: u32) -> Result<(), RawStringError> {
    debug_assert!(!input.is_empty());
    let mut cursor = Tokenizer::new(input);
    // move past the leading `r` or `br`
    for _ in 0..prefix_len {
        cursor.bump().unwrap();
    }
    cursor.raw_double_quoted_string(prefix_len).map(|_| ())
}

/// Creates an iterator that produces tokens from the input string.
pub fn tokenize(input: &str) -> impl Iterator<Item = Token> {
    let mut cursor = Tokenizer::new(input);
    std::iter::from_fn(move || {
        let token = cursor.advance_token();
        if token.r#type != TokenType::Eof {
            Some(token)
        } else {
            None
        }
    })
}

/// Checks if `c` is considered a whitespace according to Unicode `Pattern_White_Space``.
pub fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        // usual ASCII suspects
        '\u{0009}'   // \t
        | '\u{000A}' // \n
        | '\u{000B}' // vertical tab
        | '\u{000C}' // form feed
        | '\u{000D}' // \r
        | '\u{0020}' // space
        // NEXT LINE from latin1
        | '\u{0085}'
        // bidi markers
        | '\u{200E}' // LEFT-TO-RIGHT MARK
        | '\u{200F}' // RIGHT-TO-LEFT MARK
        // dedicated whitespace characters from Unicode
        | '\u{2028}' // LINE SEPARATOR
        | '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

/// Checks if `c` is valid as a first character of an identifier.
pub fn is_id_start(c: char) -> bool {
    // this is XID_Start OR '_' (which formally is not a XID_Start)
    c == '_' || UnicodeXID::is_xid_start(c)
}

/// Checks if `c` is valid as a non-first character of an identifier.
pub fn is_id_continue(c: char) -> bool {
    UnicodeXID::is_xid_continue(c)
}

/// Checks if the passed string is lexically an identifier.
pub fn is_ident(string: &str) -> bool {
    let mut chars = string.chars();
    if let Some(start) = chars.next() {
        is_id_start(start) && chars.all(is_id_continue)
    } else {
        false
    }
}

impl Tokenizer<'_> {
    /// Parses a token from the input string.
    pub(crate) fn advance_token(&mut self) -> Token {
        let Some(first_char) = self.bump() else {
            return Token::new(TokenType::Eof, 0);
        };

        let token_kind = match first_char {
            // slash, comment
            '/' => match self.first() {
                '/' => self.line_comment(),
                _ => TokenType::Slash,
            },

            // whitespace sequence
            c if is_whitespace(c) => self.whitespace(),

            // raw identifier, raw string literal or identifier
            'r' => match (self.first(), self.second()) {
                ('#', c1) if is_id_start(c1) => self.raw_identifier(),
                ('#', _) | ('"', _) => {
                    let res = self.raw_double_quoted_string(1);
                    let suffix_start = self.pos_within_token();
                    if res.is_ok() {
                        self.eat_literal_suffix();
                    }
                    let kind = LiteralTokenType::RawString { n_hashes: res.ok() };
                    TokenType::Literal { kind, suffix_start }
                }
                _ => self.identifier_or_unknown_prefix_with_first('r'),
            },

            // byte literal, byte string literal, raw byte string literal or identifier
            'b' => self.c_or_byte_string(
                |terminated| LiteralTokenType::ByteString { terminated },
                |n_hashes| LiteralTokenType::RawByteString { n_hashes },
                Some(|terminated| LiteralTokenType::Byte { terminated }),
                'b',
            ),

            // identifier (this should be checked after other variant that can
            // start as identifier)
            c if is_id_start(c) => self.identifier_or_unknown_prefix_with_first(c),

            // numeric literal
            c @ '0'..='9' => {
                let literal_kind = self.number(c);
                let suffix_start = self.pos_within_token();
                self.eat_literal_suffix();
                TokenType::Literal {
                    kind: literal_kind,
                    suffix_start,
                }
            }

            // one or multi-symbol tokens
            ';' => TokenType::Semi,
            ',' => TokenType::Comma,
            ':' => {
                if self.first() == ':' {
                    self.bump();
                    TokenType::DoubleColon
                } else {
                    TokenType::Colon
                }
            }
            '.' => {
                if self.first() == '.' && self.second() == '.' {
                    self.bump();
                    self.bump();
                    TokenType::DotDotDot
                } else if self.first() == '.' {
                    self.bump();
                    TokenType::DotDot
                } else {
                    TokenType::Dot
                }
            }
            '(' => TokenType::OpenParenthesis,
            ')' => TokenType::CloseParenthesis,
            '{' => TokenType::OpenBrace,
            '}' => TokenType::CloseBrace,
            '[' => TokenType::OpenBracket,
            ']' => TokenType::CloseBracket,
            '@' => TokenType::At,
            '#' => TokenType::Pound,
            '~' => TokenType::Tilde,
            '?' => TokenType::Question,
            '$' => TokenType::Dollar,
            '=' => {
                if self.first() == '>' {
                    self.bump();
                    TokenType::FatArrow
                } else {
                    TokenType::Equals
                }
            }
            '!' => TokenType::Bang,
            '<' => TokenType::LessThan,
            '>' => TokenType::GreaterThan,
            '-' => {
                if self.first() == '>' {
                    self.bump();
                    TokenType::ThinArrow
                } else {
                    TokenType::Minus
                }
            }
            '&' => TokenType::And,
            '|' => TokenType::Or,
            '+' => TokenType::Plus,
            '*' => TokenType::Star,
            '^' => TokenType::Caret,
            '%' => TokenType::Percent,

            // character literal
            '\'' => {
                let terminated = self.single_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = LiteralTokenType::Character { terminated };
                TokenType::Literal { kind, suffix_start }
            }

            // string literal
            '"' => {
                let terminated = self.double_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = LiteralTokenType::String { terminated };
                TokenType::Literal { kind, suffix_start }
            }
            // identifier starting with an emoji. Only lexed for graceful error recovery
            c if !c.is_ascii() && c.is_emoji_char() => self.invalid_ident(),
            _ => TokenType::Unknown,
        };
        let res = Token::new(token_kind, self.pos_within_token());
        self.reset_pos_within_token();
        res
    }

    /// Parses a line comment.
    fn line_comment(&mut self) -> TokenType {
        debug_assert!(self.prev() == '/' && self.first() == '/');
        self.bump();

        let doc_style = match self.first() {
            // `//!` is an inner line doc comment
            '!' => Some(DocPosition::Inner),
            // `////` (more than 3 slashes) is not considered a doc comment
            '/' if self.second() != '/' => Some(DocPosition::Outer),
            _ => None,
        };

        self.eat_until(b'\n');
        TokenType::LineComment { doc_style }
    }

    /// Parses a whitespace sequence.
    fn whitespace(&mut self) -> TokenType {
        debug_assert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        TokenType::Whitespace
    }

    /// Parses a raw identifier.
    fn raw_identifier(&mut self) -> TokenType {
        debug_assert!(self.prev() == 'r' && self.first() == '#' && is_id_start(self.second()));
        // eat "#" symbol
        self.bump();
        // eat the identifier part of RawIdent
        self.eat_identifier();
        TokenType::RawIdentifier
    }

    /// Parses an identifier or an unknown prefix.
    fn identifier_or_unknown_prefix_with_first(&mut self, first_char: char) -> TokenType {
        debug_assert!(is_id_start(first_char));
        // build the identifier string while consuming continuation characters
        let mut ident = String::new();
        ident.push(first_char);
        while is_id_continue(self.first()) {
            if let Some(ch) = self.bump() {
                ident.push(ch);
            } else {
                break;
            }
        }

        // known prefixes must have been handled earlier
        match self.first() {
            '#' | '"' | '\'' => return TokenType::UnknownPrefix,
            c if !c.is_ascii() && c.is_emoji_char() => return self.invalid_ident(),
            _ => {}
        }

        TokenType::Identifier
    }

    /// Parses an invalid identifier.
    fn invalid_ident(&mut self) -> TokenType {
        // start is already eaten, eat the rest of identifier
        self.eat_while(|c| {
            const ZERO_WIDTH_JOINER: char = '\u{200d}';
            is_id_continue(c) || (!c.is_ascii() && c.is_emoji_char()) || c == ZERO_WIDTH_JOINER
        });
        // an invalid identifier followed by '#' or '"' or '\'' could be
        // interpreted as an invalid literal prefix. We don't bother doing that
        // because the treatment of invalid identifiers and invalid prefixes
        // would be the same
        TokenType::InvalidIdentifier
    }

    /// Parses a c-string literal, raw c-string literal or identifier.
    fn c_or_byte_string(
        &mut self,
        mk_kind: fn(bool) -> LiteralTokenType,
        mk_kind_raw: fn(Option<u8>) -> LiteralTokenType,
        single_quoted: Option<fn(bool) -> LiteralTokenType>,
        first_char: char,
    ) -> TokenType {
        match (self.first(), self.second(), single_quoted) {
            ('\'', _, Some(single_quoted)) => {
                self.bump();
                let terminated = self.single_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = single_quoted(terminated);
                TokenType::Literal { kind, suffix_start }
            }
            ('"', _, _) => {
                self.bump();
                let terminated = self.double_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = mk_kind(terminated);
                TokenType::Literal { kind, suffix_start }
            }
            ('r', '"', _) | ('r', '#', _) => {
                self.bump();
                let res = self.raw_double_quoted_string(2);
                let suffix_start = self.pos_within_token();
                if res.is_ok() {
                    self.eat_literal_suffix();
                }
                let kind = mk_kind_raw(res.ok());
                TokenType::Literal { kind, suffix_start }
            }
            _ => self.identifier_or_unknown_prefix_with_first(first_char),
        }
    }

    /// Parses a numeric literal.
    fn number(&mut self, first_digit: char) -> LiteralTokenType {
        debug_assert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = NumberBase::Decimal;
        if first_digit == '0' {
            // attempt to parse encoding base
            match self.first() {
                'b' => {
                    base = NumberBase::Binary;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return LiteralTokenType::Integer {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'o' => {
                    base = NumberBase::Octal;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return LiteralTokenType::Integer {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'x' => {
                    base = NumberBase::Hexadecimal;
                    self.bump();
                    if !self.eat_hexadecimal_digits() {
                        return LiteralTokenType::Integer {
                            base,
                            empty_int: true,
                        };
                    }
                }
                // not a base prefix; consume additional digits
                '0'..='9' | '_' => {
                    self.eat_decimal_digits();
                }

                // also not a base prefix; nothing more to do here
                '.' | 'e' | 'E' => {}

                // just a 0
                _ => {
                    return LiteralTokenType::Integer {
                        base,
                        empty_int: false,
                    };
                }
            }
        } else {
            // no base prefix, parse number in the usual way
            self.eat_decimal_digits();
        }

        match self.first() {
            // don't be greedy if this is actually an
            // integer literal followed by field/method access or a range pattern
            // (`0..2` and `12.foo()`)
            '.' if self.second() != '.' && !is_id_start(self.second()) => {
                // might have stuff after the ., and if it does, it needs to start
                // with a number
                self.bump();
                let mut empty_exponent = false;
                if self.first().is_ascii_digit() {
                    self.eat_decimal_digits();
                    match self.first() {
                        'e' | 'E' => {
                            self.bump();
                            empty_exponent = !self.eat_float_exponent();
                        }
                        _ => (),
                    }
                }
                LiteralTokenType::Float {
                    base,
                    empty_exponent,
                }
            }
            'e' | 'E' => {
                self.bump();
                let empty_exponent = !self.eat_float_exponent();
                LiteralTokenType::Float {
                    base,
                    empty_exponent,
                }
            }
            _ => LiteralTokenType::Integer {
                base,
                empty_int: false,
            },
        }
    }

    /// Parses a single-quoted string.
    fn single_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '\'');
        // check if it's a one-symbol literal
        if self.second() == '\'' && self.first() != '\\' {
            self.bump();
            self.bump();
            return true;
        }
        // literal has more than one symbol
        // parse until either quotes are terminated or error is detected
        loop {
            match self.first() {
                // quotes are terminated, finish parsing
                '\'' => {
                    self.bump();
                    return true;
                }
                // probably beginning of the comment, which we don't want to include
                // to the error report
                '/' => break,
                // newline without following '\'' means unclosed quote, stop parsing
                '\n' if self.second() != '\'' => break,
                // end of file, stop parsing
                EOF_CHAR if self.is_eof() => break,
                // escaped slash is considered one character, so bump twice
                '\\' => {
                    self.bump();
                    self.bump();
                }
                // skip the character
                _ => {
                    self.bump();
                }
            }
        }
        // string was not terminated
        false
    }

    /// Parses a double-quoted string.
    fn double_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '"');
        while let Some(c) = self.bump() {
            match c {
                '"' => {
                    return true;
                }
                '\\' if self.first() == '\\' || self.first() == '"' => {
                    // bump again to skip escaped character
                    self.bump();
                }
                _ => (),
            }
        }
        // end of file reached
        false
    }

    /// Parses a raw double-quoted string.
    pub(crate) fn raw_double_quoted_string(
        &mut self,
        prefix_len: u32,
    ) -> Result<u8, RawStringError> {
        // wrap the actual function to handle the error with too many hashes
        // this way, it eats the whole raw string
        let n_hashes = self.raw_string_unvalidated(prefix_len)?;
        // only up to 255 `#`s are allowed in raw strings
        match u8::try_from(n_hashes) {
            Ok(num) => Ok(num),
            Err(_) => Err(RawStringError::TooManyDelimiters { found: n_hashes }),
        }
    }

    /// Parses a raw string.
    pub(crate) fn raw_string_unvalidated(
        &mut self,
        prefix_len: u32,
    ) -> Result<u32, RawStringError> {
        debug_assert!(self.prev() == 'r');
        let start_pos = self.pos_within_token();
        let mut possible_terminator_offset: Option<u32> = None;
        let mut max_hashes = 0;

        // count opening '#' symbols
        let mut eaten = 0;
        while self.first() == '#' {
            eaten += 1;
            self.bump();
        }
        let n_start_hashes = eaten;

        // check that string is started
        match self.bump() {
            Some('"') => (),
            c => {
                let c = c.unwrap_or(EOF_CHAR);
                return Err(RawStringError::InvalidStarter { bad_char: c });
            }
        }

        // skip the string contents and on each '#' character met, check if this is
        // a raw string termination
        loop {
            self.eat_until(b'"');

            if self.is_eof() {
                return Err(RawStringError::NoTerminator {
                    expected: n_start_hashes,
                    found: max_hashes,
                    possible_terminator_offset,
                });
            }

            // eat closing double quote
            self.bump();

            // check that amount of closing '#' symbols
            // is equal to the amount of opening ones
            // note that this will not consume extra trailing `#` characters:
            // `r###"abcde"####` is lexed as a `RawStr { n_hashes: 3 }`
            // followed by a `#` token
            let mut n_end_hashes = 0;
            while self.first() == '#' && n_end_hashes < n_start_hashes {
                n_end_hashes += 1;
                self.bump();
            }

            if n_end_hashes == n_start_hashes {
                return Ok(n_start_hashes);
            } else if n_end_hashes > max_hashes {
                // keep track of possible terminators to give a hint about
                // where there might be a missing terminator
                possible_terminator_offset =
                    Some(self.pos_within_token() - start_pos - n_end_hashes + prefix_len);
                max_hashes = n_end_hashes;
            }
        }
    }

    /// Parses decimal digits.
    pub(crate) fn eat_decimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    /// Parses hexadecimal digits.
    pub(crate) fn eat_hexadecimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    /// Parses the float exponent.
    pub(crate) fn eat_float_exponent(&mut self) -> bool {
        debug_assert!(self.prev() == 'e' || self.prev() == 'E');
        if self.first() == '-' || self.first() == '+' {
            self.bump();
        }
        self.eat_decimal_digits()
    }

    /// Parses the suffix of the literal, e.g. "u8".
    pub(crate) fn eat_literal_suffix(&mut self) {
        self.eat_identifier();
    }

    /// Parses an identifier.
    ///
    /// NOTE: succeeds on `_`, which isn't a valid identifier.
    pub(crate) fn eat_identifier(&mut self) {
        if !is_id_start(self.first()) {
            return;
        }
        self.bump();
        self.eat_while(is_id_continue);
    }
}
