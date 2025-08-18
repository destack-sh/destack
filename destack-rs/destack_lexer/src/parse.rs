//! Low-level Destack lexer adapted from rustc.

use crate::DocStyle;
use crate::cursor::{Cursor, EOF_CHAR};
use crate::token::{Base, LiteralKind, RawStrError, Token, TokenKind};
use unicode_properties::UnicodeEmoji;
pub use unicode_xid::UNICODE_VERSION as UNICODE_XID_VERSION;

/// `destackc` allows files to have a shebang, e.g. "#!/usr/bin/destackrun",
/// but shebang isn't a part of destack syntax.
pub fn strip_shebang(input: &str) -> Option<usize> {
    // shebang must start with `#!` literally, without any preceding whitespace
    // for simplicity we consider any line starting with `#!` a shebang,
    // regardless of restrictions put on shebangs by specific platforms
    if let Some(input_tail) = input.strip_prefix("#!") {
        // ok, this is a shebang but if the next non-whitespace token is `[`,
        // then it may be valid Destack code, so consider it Destack code
        let next_non_whitespace_token = tokenize(input_tail).map(|tok| tok.kind).find(|tok| {
            !matches!(
                tok,
                TokenKind::Whitespace | TokenKind::LineComment { doc_style: None }
            )
        });
        if next_non_whitespace_token != Some(TokenKind::OpenBracket) {
            // no other choice than to consider this a shebang
            return Some(2 + input_tail.lines().next().unwrap_or_default().len());
        }
    }
    None
}

/// Validates a raw string literal. Used for getting more information about a
/// problem with a `RawStr`/`RawByteStr` with a `None` field.
#[inline]
pub fn validate_raw_str(input: &str, prefix_len: u32) -> Result<(), RawStrError> {
    debug_assert!(!input.is_empty());
    let mut cursor = Cursor::new(input);
    // move past the leading `r` or `br`
    for _ in 0..prefix_len {
        cursor.bump().unwrap();
    }
    cursor.raw_double_quoted_string(prefix_len).map(|_| ())
}

/// Creates an iterator that produces tokens from the input string.
pub fn tokenize(input: &str) -> impl Iterator<Item = Token> {
    let mut cursor = Cursor::new(input);
    std::iter::from_fn(move || {
        let token = cursor.advance_token();
        if token.kind != TokenKind::Eof {
            Some(token)
        } else {
            None
        }
    })
}

/// True if `c` is considered a whitespace according to Destack language definition.
/// See [Destack language reference](https://doc.destack-lang.org/reference/whitespace.html)
/// for definitions of these classes.
pub fn is_whitespace(c: char) -> bool {
    // this is Pattern_White_Space
    //
    // note that this set is stable (ie, it doesn't change with different
    // Unicode versions), so it's ok to just hard-code the values

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

/// True if `c` is valid as a first character of an identifier.
/// See [Destack language reference](https://doc.destack-lang.org/reference/identifiers.html) for
/// a formal definition of valid identifier name.
pub fn is_id_start(c: char) -> bool {
    // this is XID_Start OR '_' (which formally is not a XID_Start)
    c == '_' || unicode_xid::UnicodeXID::is_xid_start(c)
}

/// True if `c` is valid as a non-first character of an identifier.
/// See [Destack language reference](https://doc.destack-lang.org/reference/identifiers.html) for
/// a formal definition of valid identifier name.
pub fn is_id_continue(c: char) -> bool {
    unicode_xid::UnicodeXID::is_xid_continue(c)
}

/// The passed string is lexically an identifier.
pub fn is_ident(string: &str) -> bool {
    let mut chars = string.chars();
    if let Some(start) = chars.next() {
        is_id_start(start) && chars.all(is_id_continue)
    } else {
        false
    }
}

impl Cursor<'_> {
    /// Parses a token from the input string.
    pub(crate) fn advance_token(&mut self) -> Token {
        let Some(first_char) = self.bump() else {
            return Token::new(TokenKind::Eof, 0);
        };

        let token_kind = match first_char {
            // slash, comment
            '/' => match self.first() {
                '/' => self.line_comment(),
                _ => TokenKind::Slash,
            },

            // whitespace sequence
            c if is_whitespace(c) => self.whitespace(),

            // raw identifier, raw string literal or identifier
            'r' => match (self.first(), self.second()) {
                ('#', c1) if is_id_start(c1) => self.raw_ident(),
                ('#', _) | ('"', _) => {
                    let res = self.raw_double_quoted_string(1);
                    let suffix_start = self.pos_within_token();
                    if res.is_ok() {
                        self.eat_literal_suffix();
                    }
                    let kind = LiteralKind::RawStr { n_hashes: res.ok() };
                    TokenKind::Literal { kind, suffix_start }
                }
                _ => self.ident_or_unknown_prefix(),
            },

            // byte literal, byte string literal, raw byte string literal or identifier
            'b' => self.c_or_byte_string(
                |terminated| LiteralKind::ByteStr { terminated },
                |n_hashes| LiteralKind::RawByteStr { n_hashes },
                Some(|terminated| LiteralKind::Byte { terminated }),
            ),

            // c-string literal, raw c-string literal or identifier
            'c' => self.c_or_byte_string(
                |terminated| LiteralKind::CStr { terminated },
                |n_hashes| LiteralKind::RawCStr { n_hashes },
                None,
            ),

            // identifier (this should be checked after other variant that can
            // start as identifier)
            c if is_id_start(c) => self.ident_or_unknown_prefix(),

            // numeric literal
            c @ '0'..='9' => {
                let literal_kind = self.number(c);
                let suffix_start = self.pos_within_token();
                self.eat_literal_suffix();
                TokenKind::Literal {
                    kind: literal_kind,
                    suffix_start,
                }
            }

            // one-symbol tokens
            ';' => TokenKind::Semi,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            '@' => TokenKind::At,
            '#' => TokenKind::Pound,
            '~' => TokenKind::Tilde,
            '?' => TokenKind::Question,
            ':' => TokenKind::Colon,
            '$' => TokenKind::Dollar,
            '=' => TokenKind::Eq,
            '!' => TokenKind::Bang,
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            '-' => TokenKind::Minus,
            '&' => TokenKind::And,
            '|' => TokenKind::Or,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '^' => TokenKind::Caret,
            '%' => TokenKind::Percent,

            // character literal
            '\'' => {
                let terminated = self.single_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = LiteralKind::Char { terminated };
                TokenKind::Literal { kind, suffix_start }
            }

            // string literal
            '"' => {
                let terminated = self.double_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = LiteralKind::Str { terminated };
                TokenKind::Literal { kind, suffix_start }
            }
            // identifier starting with an emoji. Only lexed for graceful error recovery
            c if !c.is_ascii() && c.is_emoji_char() => self.invalid_ident(),
            _ => TokenKind::Unknown,
        };
        let res = Token::new(token_kind, self.pos_within_token());
        self.reset_pos_within_token();
        res
    }

    fn line_comment(&mut self) -> TokenKind {
        debug_assert!(self.prev() == '/' && self.first() == '/');
        self.bump();

        let doc_style = match self.first() {
            // `//!` is an inner line doc comment
            '!' => Some(DocStyle::Inner),
            // `////` (more than 3 slashes) is not considered a doc comment
            '/' if self.second() != '/' => Some(DocStyle::Outer),
            _ => None,
        };

        self.eat_until(b'\n');
        TokenKind::LineComment { doc_style }
    }

    fn whitespace(&mut self) -> TokenKind {
        debug_assert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        TokenKind::Whitespace
    }

    fn raw_ident(&mut self) -> TokenKind {
        debug_assert!(self.prev() == 'r' && self.first() == '#' && is_id_start(self.second()));
        // eat "#" symbol
        self.bump();
        // eat the identifier part of RawIdent
        self.eat_identifier();
        TokenKind::RawIdent
    }

    fn ident_or_unknown_prefix(&mut self) -> TokenKind {
        debug_assert!(is_id_start(self.prev()));
        // start is already eaten, eat the rest of identifier
        self.eat_while(is_id_continue);
        // known prefixes must have been handled earlier. So if
        // we see a prefix here, it is definitely an unknown prefix
        match self.first() {
            '#' | '"' | '\'' => TokenKind::UnknownPrefix,
            c if !c.is_ascii() && c.is_emoji_char() => self.invalid_ident(),
            _ => TokenKind::Ident,
        }
    }

    fn invalid_ident(&mut self) -> TokenKind {
        // start is already eaten, eat the rest of identifier
        self.eat_while(|c| {
            const ZERO_WIDTH_JOINER: char = '\u{200d}';
            is_id_continue(c) || (!c.is_ascii() && c.is_emoji_char()) || c == ZERO_WIDTH_JOINER
        });
        // an invalid identifier followed by '#' or '"' or '\'' could be
        // interpreted as an invalid literal prefix. We don't bother doing that
        // because the treatment of invalid identifiers and invalid prefixes
        // would be the same
        TokenKind::InvalidIdent
    }

    fn c_or_byte_string(
        &mut self,
        mk_kind: fn(bool) -> LiteralKind,
        mk_kind_raw: fn(Option<u8>) -> LiteralKind,
        single_quoted: Option<fn(bool) -> LiteralKind>,
    ) -> TokenKind {
        match (self.first(), self.second(), single_quoted) {
            ('\'', _, Some(single_quoted)) => {
                self.bump();
                let terminated = self.single_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = single_quoted(terminated);
                TokenKind::Literal { kind, suffix_start }
            }
            ('"', _, _) => {
                self.bump();
                let terminated = self.double_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = mk_kind(terminated);
                TokenKind::Literal { kind, suffix_start }
            }
            ('r', '"', _) | ('r', '#', _) => {
                self.bump();
                let res = self.raw_double_quoted_string(2);
                let suffix_start = self.pos_within_token();
                if res.is_ok() {
                    self.eat_literal_suffix();
                }
                let kind = mk_kind_raw(res.ok());
                TokenKind::Literal { kind, suffix_start }
            }
            _ => self.ident_or_unknown_prefix(),
        }
    }

    fn number(&mut self, first_digit: char) -> LiteralKind {
        debug_assert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = Base::Decimal;
        if first_digit == '0' {
            // attempt to parse encoding base
            match self.first() {
                'b' => {
                    base = Base::Binary;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return LiteralKind::Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'o' => {
                    base = Base::Octal;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return LiteralKind::Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'x' => {
                    base = Base::Hexadecimal;
                    self.bump();
                    if !self.eat_hexadecimal_digits() {
                        return LiteralKind::Int {
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
                    return LiteralKind::Int {
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
                LiteralKind::Float {
                    base,
                    empty_exponent,
                }
            }
            'e' | 'E' => {
                self.bump();
                let empty_exponent = !self.eat_float_exponent();
                LiteralKind::Float {
                    base,
                    empty_exponent,
                }
            }
            _ => LiteralKind::Int {
                base,
                empty_int: false,
            },
        }
    }

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

    /// Eats double-quoted string and returns true
    /// if string is terminated.
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

    /// Eats the double-quoted string and returns `n_hashes` and an error if encountered.
    pub(crate) fn raw_double_quoted_string(&mut self, prefix_len: u32) -> Result<u8, RawStrError> {
        // wrap the actual function to handle the error with too many hashes
        // this way, it eats the whole raw string
        let n_hashes = self.raw_string_unvalidated(prefix_len)?;
        // only up to 255 `#`s are allowed in raw strings
        match u8::try_from(n_hashes) {
            Ok(num) => Ok(num),
            Err(_) => Err(RawStrError::TooManyDelimiters { found: n_hashes }),
        }
    }

    pub(crate) fn raw_string_unvalidated(&mut self, prefix_len: u32) -> Result<u32, RawStrError> {
        debug_assert!(self.prev() == 'r');
        let start_pos = self.pos_within_token();
        let mut possible_terminator_offset = None;
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
                return Err(RawStrError::InvalidStarter { bad_char: c });
            }
        }

        // skip the string contents and on each '#' character met, check if this is
        // a raw string termination
        loop {
            self.eat_until(b'"');

            if self.is_eof() {
                return Err(RawStrError::NoTerminator {
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

    /// Eats the float exponent. Returns true if at least one digit was met,
    /// and returns false otherwise.
    pub(crate) fn eat_float_exponent(&mut self) -> bool {
        debug_assert!(self.prev() == 'e' || self.prev() == 'E');
        if self.first() == '-' || self.first() == '+' {
            self.bump();
        }
        self.eat_decimal_digits()
    }

    // eats the suffix of the literal, e.g. "u8"
    pub(crate) fn eat_literal_suffix(&mut self) {
        self.eat_identifier();
    }

    // eats the identifier. Note: succeeds on `_`, which isn't a valid
    // identifier
    pub(crate) fn eat_identifier(&mut self) {
        if !is_id_start(self.first()) {
            return;
        }
        self.bump();

        self.eat_while(is_id_continue);
    }
}
