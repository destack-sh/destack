//! Low-level general purpose DS lexer (adapted from rustc).

use crate::{TokenSpan, is_identifier_continue, is_identifier_start, is_whitespace};

use super::token::{NumberBase, RawLiteralType, RawStringError, Token, TokenType};
use super::tokenizer::{EOF_CHAR, Tokenizer};

use destack_library_unicode::UnicodeEmoji;
use dyst_language_source::{SourceId, Span};

pub const TRIVIA_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Whitespace,
    TokenType::LineComment,
    TokenType::BlockComment,
    TokenType::DocLineComment,
    TokenType::DocBlockComment,
];

#[inline]
pub fn is_semantic(token_type: TokenType) -> bool {
    !TRIVIA_TOKEN_TYPES.contains(&token_type)
}

/// Tokenize the input string into an Iterator of semantic and non-semantic Tokens (no Spans).
pub fn tokenize(input: &str) -> impl Iterator<Item = Token> {
    let mut cursor = Tokenizer::new(input);
    let mut done = false;
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        let token = cursor.advance();
        if token.r#type == TokenType::End {
            done = true;
        }
        Some(token)
    })
}

/// Tokenize the input string into an Iterator of Tokens and Spans.
/// Returns both semantic and trivia tokens.
pub fn tokenize_with_spans(
    source_id: SourceId,
    input: &str,
    filter: impl Fn(TokenType) -> bool,
) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
    let mut cursor = Tokenizer::new(input);
    let mut semantic_tokens: Vec<TokenSpan> = Vec::new();
    let mut trivia_tokens: Vec<TokenSpan> = Vec::new();
    let mut pos = 0;

    loop {
        let token = cursor.advance();
        let token_span = TokenSpan {
            token,
            span: Span {
                source: source_id,
                start: pos,
                end: pos + token.len,
            },
        };
        if filter(token.r#type) {
            semantic_tokens.push(token_span);
        } else {
            trivia_tokens.push(token_span);
        }
        pos = pos.saturating_add(token.len);
        if token.r#type == TokenType::End {
            break;
        }
    }

    debug_assert!(!semantic_tokens.is_empty());
    debug_assert_eq!(
        semantic_tokens[semantic_tokens.len() - 1].token.r#type,
        TokenType::End
    );
    debug_assert_eq!(pos, input.len() as u32);

    (semantic_tokens, trivia_tokens)
}

impl Tokenizer<'_> {
    /// Parses a token from the input string.
    pub(crate) fn advance(&mut self) -> Token {
        // eat first character until nothing is left (=EOF)
        let Some(first_char) = self.bump() else {
            return Token::new(TokenType::End, 0, None);
        };

        // parse token
        let (token_type, literal) = match first_char {
            // whitespace
            c if is_whitespace(c) => {
                if c == '\n' {
                    (TokenType::Newline, None)
                } else {
                    (self.eat_whitespace(), None)
                }
            }

            // slash, comments (line, block, doc) or divide ops
            '/' => {
                // fast-path use bytes to avoid iterator cloning and extra UTF-8 decoding
                let bytes = self.as_str().as_bytes();
                let next = bytes.first().copied();
                match next {
                    // //
                    Some(b'/') => {
                        // doc line comment if exactly three slashes and the fourth is not '/'
                        let third_is_slash = bytes.get(1).copied() == Some(b'/');
                        let fourth_is_slash = bytes.get(2).copied() == Some(b'/');
                        let is_doc_line = third_is_slash && !fourth_is_slash;
                        self.eat_until(b'\n');
                        if is_doc_line {
                            (TokenType::DocLineComment, None)
                        } else {
                            (TokenType::LineComment, None)
                        }
                    }
                    // /*
                    // block comments starting with '/*' (with nesting)
                    Some(b'*') => {
                        // detect doc block comment for exactly '/**' (not '/***')
                        let third_is_star = bytes.get(1).copied() == Some(b'*');
                        let fourth_is_star = bytes.get(2).copied() == Some(b'*');
                        let is_doc_block = third_is_star && !fourth_is_star;
                        // consume the initial '*'
                        self.bump();
                        self.eat_block_comment_body();
                        if is_doc_block {
                            (TokenType::DocBlockComment, None)
                        } else {
                            (TokenType::BlockComment, None)
                        }
                    }
                    // divide or '/='
                    _ => {
                        // /=
                        if self.peek() == '=' {
                            self.bump();
                            (TokenType::DivideAssign, None)
                        }
                        // /
                        else {
                            (TokenType::Divide, None)
                        }
                    }
                }
            }

            // raw string literal, or identifier starting with 'r'
            'r' => match (self.peek(), self.peek_next()) {
                // raw string literal
                ('#', _) | ('"', _) => {
                    let raw_dq_string = self.eat_raw_double_quoted_string(1);
                    let literal = RawLiteralType::RawString {
                        hashes: raw_dq_string.ok(),
                    };
                    (TokenType::Literal, Some(literal))
                }
                // identifier fallback
                _ => self.eat_identifier_or_such('r'),
            },

            // byte literal, byte string literal, or identifier starting with 'b'
            'b' => {
                let this = &mut *self;
                match (this.peek(), this.peek_next()) {
                    // b'
                    // single-quoted byte literal
                    ('\'', _) => {
                        this.bump();
                        let is_terminated = this.eat_single_quoted_string();
                        (
                            TokenType::Literal,
                            Some(RawLiteralType::Byte { is_terminated }),
                        )
                    }
                    // b"
                    // double-quoted byte string literal
                    ('"', _) => {
                        this.bump();
                        let is_terminated = this.eat_double_quoted_string();
                        (
                            TokenType::Literal,
                            Some(RawLiteralType::ByteString { is_terminated }),
                        )
                    }
                    // br" or br#
                    // raw double-quoted byte string literal
                    ('r', '"') | ('r', '#') => {
                        this.bump();
                        let raw_dq_string = this.eat_raw_double_quoted_string(2);
                        (
                            TokenType::Literal,
                            Some(RawLiteralType::RawByteString {
                                hashes: raw_dq_string.ok(),
                            }),
                        )
                    }
                    // identifier fallback (starting with 'b')
                    _ => this.eat_identifier_or_such('b'),
                }
            }

            // wildcard (if not followed by identifier)
            '_' if !is_identifier_continue(self.peek()) => (TokenType::Wildcard, None),

            // other identifier
            c if is_identifier_start(c) => self.eat_identifier_or_such(c),

            // numeric literal
            c @ '0'..='9' => {
                let literal = self.eat_number_literal(c);
                (TokenType::Literal, Some(literal))
            }

            // symbols
            ':' => (TokenType::Colon, None),
            ';' => (TokenType::Semicolon, None),
            ',' => (TokenType::Comma, None),
            '.' => {
                // ..
                if self.peek() == '.' {
                    // ...
                    if self.peek_next() == '.' {
                        self.bump();
                        self.bump();
                        (TokenType::RangeWide, None)
                    }
                    // ..
                    else {
                        self.bump();
                        (TokenType::Range, None)
                    }
                }
                // .
                else {
                    (TokenType::Dot, None)
                }
            }
            '@' => (TokenType::At, None),
            '~' => (TokenType::BitwiseNot, None),
            '?' => {
                // ??
                if self.peek() == '?' {
                    self.bump();
                    self.bump();
                    (TokenType::Coalesce, None)
                }
                // ?
                else {
                    (TokenType::Maybe, None)
                }
            }
            '$' => (TokenType::Virtual, None),

            // brackets
            '(' => (TokenType::OpenParenthesis, None),
            ')' => (TokenType::CloseParenthesis, None),
            '{' => (TokenType::OpenBrace, None),
            '}' => (TokenType::CloseBrace, None),
            '[' => (TokenType::OpenBracket, None),
            ']' => (TokenType::CloseBracket, None),

            // bang
            '!' => {
                // !=
                if self.peek() == '=' {
                    self.bump();
                    (TokenType::NotEqual, None)
                }
                // !
                else {
                    (TokenType::Not, None)
                }
            }

            // subtract or arrow
            '-' => {
                // ->
                if self.peek() == '>' {
                    self.bump();
                    (TokenType::ThinArrow, None)
                }
                // --
                else if self.peek() == '-' {
                    // ---
                    if self.peek_next() == '-' {
                        self.bump();
                        self.bump();
                        (TokenType::EmptyWide, None)
                    }
                    // --
                    else {
                        self.bump();
                        self.bump();
                        (TokenType::Empty, None)
                    }
                }
                // -%
                else if self.peek() == '%' {
                    self.bump();
                    // -%=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::WrappingSubtractAssign, None)
                    }
                    // -%
                    else {
                        (TokenType::WrappingSubtract, None)
                    }
                }
                // -|
                else if self.peek() == '|' {
                    self.bump();
                    // -|=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::SaturatingSubtractAssign, None)
                    }
                    // -|
                    else {
                        (TokenType::SaturatingSubtract, None)
                    }
                }
                // -=
                else if self.peek() == '=' {
                    self.bump();
                    (TokenType::SubtractAssign, None)
                }
                // -
                else {
                    (TokenType::Subtract, None)
                }
            }

            // bitwise and, logical and and their assignments
            '&' => {
                // &&
                if self.peek() == '&' {
                    self.bump();
                    // &&=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::LogicalAndAssign, None)
                    }
                    // &&
                    else {
                        (TokenType::LogicalAnd, None)
                    }
                }
                // &=
                else if self.peek() == '=' {
                    self.bump();
                    (TokenType::BitwiseAndAssign, None)
                }
                // &
                else {
                    (TokenType::BitwiseAnd, None)
                }
            }

            // bitwise or, logical or and their assignments
            '|' => {
                // ||
                if self.peek() == '|' {
                    self.bump();
                    // ||=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::LogicalOrAssign, None)
                    }
                    // ||
                    else {
                        (TokenType::LogicalOr, None)
                    }
                }
                // |=
                else if self.peek() == '=' {
                    self.bump();
                    (TokenType::BitwiseOrAssign, None)
                }
                // |
                else {
                    (TokenType::BitwiseOr, None)
                }
            }

            // equal or assign
            '=' => {
                // =>
                if self.peek() == '>' {
                    self.bump();
                    (TokenType::FatArrow, None)
                }
                // ==
                else if self.peek() == '=' {
                    self.bump();
                    (TokenType::Equal, None)
                }
                // =
                else {
                    (TokenType::Assign, None)
                }
            }

            // less than or shift left
            '<' => {
                // <<
                if self.peek() == '<' {
                    self.bump();
                    // <<|
                    if self.peek() == '|' {
                        self.bump();
                        // <<|=
                        if self.peek() == '=' {
                            self.bump();
                            (TokenType::SaturatingShiftLeftAssign, None)
                        }
                        // <<|
                        else {
                            (TokenType::SaturatingShiftLeft, None)
                        }
                    }
                    // <<=
                    else if self.peek() == '=' {
                        self.bump();
                        (TokenType::ShiftLeftAssign, None)
                    }
                    // <<
                    else {
                        (TokenType::ShiftLeft, None)
                    }
                }
                // <=
                else if self.peek() == '=' {
                    self.bump();
                    (TokenType::LessThanOrEqual, None)
                }
                // <
                else {
                    (TokenType::LessThan, None)
                }
            }

            // greater than or shift right
            '>' => {
                // >>
                if self.peek() == '>' {
                    self.bump();
                    // >>=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::ShiftRightAssign, None)
                    }
                    // >>
                    else {
                        (TokenType::ShiftRight, None)
                    }
                }
                // >=
                else if self.peek() == '=' {
                    self.bump();
                    (TokenType::GreaterThanOrEqual, None)
                }
                // >
                else {
                    (TokenType::GreaterThan, None)
                }
            }

            // xor
            '^' => {
                // ^=
                if self.peek() == '=' {
                    self.bump();
                    (TokenType::BitwiseXorAssign, None)
                }
                // ^
                else {
                    (TokenType::BitwiseXor, None)
                }
            }

            // add
            '+' => {
                // +%
                if self.peek() == '%' {
                    self.bump();
                    // +%=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::WrappingAddAssign, None)
                    }
                    // +%
                    else {
                        (TokenType::WrappingAdd, None)
                    }
                }
                // +|
                else if self.peek() == '|' {
                    self.bump();
                    // +|=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::SaturatingAddAssign, None)
                    }
                    // +|
                    else {
                        (TokenType::SaturatingAdd, None)
                    }
                }
                // +=
                else if self.peek() == '=' {
                    self.bump();
                    (TokenType::AddAssign, None)
                }
                // +
                else {
                    (TokenType::Add, None)
                }
            }

            // multiply
            '*' => {
                // *%
                if self.peek() == '%' {
                    self.bump();
                    // *%=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::WrappingMultiplyAssign, None)
                    }
                    // *%
                    else {
                        (TokenType::WrappingMultiply, None)
                    }
                }
                // *|
                else if self.peek() == '|' {
                    self.bump();
                    // *|=
                    if self.peek() == '=' {
                        self.bump();
                        (TokenType::SaturatingMultiplyAssign, None)
                    }
                    // *|
                    else {
                        (TokenType::SaturatingMultiply, None)
                    }
                }
                // *=
                else if self.peek() == '=' {
                    self.bump();
                    (TokenType::MultiplyAssign, None)
                }
                // *
                else {
                    (TokenType::Multiply, None)
                }
            }

            // remainder
            '%' => {
                // %=
                if self.peek() == '=' {
                    self.bump();
                    (TokenType::RemainderAssign, None)
                }
                // %
                else {
                    (TokenType::Remainder, None)
                }
            }

            // character literal
            '\'' => {
                let terminated = self.eat_single_quoted_string();
                let kind = RawLiteralType::Character {
                    is_terminated: terminated,
                };
                (TokenType::Literal, Some(kind))
            }

            // string literal
            '"' => {
                let terminated = self.eat_double_quoted_string();
                let kind = RawLiteralType::String {
                    is_terminated: terminated,
                };
                (TokenType::Literal, Some(kind))
            }

            // identifier starting with an emoji (for graceful error recovery)
            c if !c.is_ascii() && c.is_emoji_char() => (self.eat_invalid_identifier(), None),
            _ => (TokenType::Unknown, None),
        };

        let token = Token::new(token_type, self.get_pos_within_token(), literal);
        self.reset_pos_within_token();
        token
    }

    /// Parses a whitespace sequence (excluding first character).
    fn eat_whitespace(&mut self) -> TokenType {
        debug_assert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        TokenType::Whitespace
    }

    /// Parses an identifier, unknown prefix or some literal string (excluding first character).
    /// Returns the token type and the literal type if it's a hardcoded literal.
    fn eat_identifier_or_such(&mut self, first_char: char) -> (TokenType, Option<RawLiteralType>) {
        debug_assert!(is_identifier_start(first_char));
        let start_pos = self.pos;
        // consume continuation characters until an unknown character is met
        self.eat_while(is_identifier_continue);
        // known prefixes must have been handled earlier
        match self.peek() {
            '#' | '"' | '\'' => return (TokenType::UnknownLiteralPrefix, None),
            c if !c.is_ascii() && c.is_emoji_char() => {
                return (self.eat_invalid_identifier(), None);
            }
            _ => {}
        }
        // special case: `void`, `null`, `true`, `false`
        // void
        if first_char == 'v' && self.str[start_pos - 1..self.pos].eq("void") {
            (TokenType::Literal, Some(RawLiteralType::Void))
        }
        // null
        else if first_char == 'n' && self.str[start_pos - 1..self.pos].eq("null") {
            (TokenType::Literal, Some(RawLiteralType::Null))
        }
        // true
        else if first_char == 't' && self.str[start_pos - 1..self.pos].eq("true") {
            (
                TokenType::Literal,
                Some(RawLiteralType::Boolean { value: true }),
            )
        }
        // false
        else if first_char == 'f' && self.str[start_pos - 1..self.pos].eq("false") {
            (
                TokenType::Literal,
                Some(RawLiteralType::Boolean { value: false }),
            )
        }
        // just an identifier
        else {
            (TokenType::Identifier, None)
        }
    }

    /// Parses an invalid identifier (excluding first character).
    fn eat_invalid_identifier(&mut self) -> TokenType {
        // start is already eaten, eat the rest of identifier
        self.eat_while(|c| {
            const ZERO_WIDTH_JOINER: char = '\u{200d}';
            is_identifier_continue(c)
                || (!c.is_ascii() && c.is_emoji_char())
                || c == ZERO_WIDTH_JOINER
        });
        TokenType::InvalidIdentifier
    }

    /// Parses a number literal (excluding first digit).
    /// Returns the number literal.
    fn eat_number_literal(&mut self, first_digit: char) -> RawLiteralType {
        debug_assert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = NumberBase::Decimal;
        if first_digit == '0' {
            // parse encoding base
            match self.peek() {
                // binary literal
                'b' => {
                    base = NumberBase::Binary;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return RawLiteralType::Int {
                            base,
                            is_empty: true,
                        };
                    }
                }

                // octal literal
                'o' => {
                    base = NumberBase::Octal;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return RawLiteralType::Int {
                            base,
                            is_empty: true,
                        };
                    }
                }

                // hexadecimal literal
                'x' => {
                    base = NumberBase::Hexadecimal;
                    self.bump();
                    if !self.eat_hexadecimal_digits() {
                        return RawLiteralType::Int {
                            base,
                            is_empty: true,
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
                    return RawLiteralType::Int {
                        base,
                        is_empty: false,
                    };
                }
            }
        } else {
            // no base prefix, parse number in the usual way
            self.eat_decimal_digits();
        }

        match self.peek() {
            // don't be greedy if this is actually an
            // integer literal followed by field/method access or a range pattern
            // (`0..2` and `12.foo()`)
            '.' if self.peek_next() != '.' && !is_identifier_start(self.peek_next()) => {
                // might have stuff after the ., and if it does, it starts with a number
                self.bump();
                let mut is_empty_exponent = false;
                if self.peek().is_ascii_digit() {
                    self.eat_decimal_digits();
                    match self.peek() {
                        'e' | 'E' => {
                            self.bump();
                            is_empty_exponent = !self.eat_float_exponent();
                        }
                        _ => (),
                    }
                }
                RawLiteralType::Float {
                    base,
                    is_empty_exponent,
                }
            }
            'e' | 'E' => {
                self.bump();
                let is_empty_exponent = !self.eat_float_exponent();
                RawLiteralType::Float {
                    base,
                    is_empty_exponent,
                }
            }
            _ => RawLiteralType::Int {
                base,
                is_empty: false,
            },
        }
    }

    /// Parses a single-quoted string (excluding first `'`).
    /// Returns whether the string is terminated.
    fn eat_single_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '\'');
        // check if it's a one-symbol literal
        if self.peek_next() == '\'' && self.peek() != '\\' {
            self.bump();
            self.bump();
            return true;
        }
        // literal has more than one symbol
        // parse until either quotes are terminated or error is detected
        loop {
            match self.peek() {
                // quotes are terminated, finish parsing
                '\'' => {
                    self.bump();
                    return true;
                }
                // probably beginning of the comment, which we don't want to include
                // to the error report
                '/' => break,
                // newline without following '\'' means unclosed quote, stop parsing
                '\n' if self.peek_next() != '\'' => break,
                // end of file, stop parsing
                EOF_CHAR if self.is_end() => break,
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

    /// Parses a double-quoted string (excluding first `"`).
    /// Returns whether the string is terminated.
    fn eat_double_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '"');
        while let Some(c) = self.bump() {
            match c {
                '"' => {
                    return true;
                }
                '\\' if self.peek() == '\\' || self.peek() == '"' => {
                    // bump again to skip escaped character
                    self.bump();
                }
                _ => (),
            }
        }
        // end of file reached
        false
    }

    /// Parses a raw double-quoted string (with hashes, excluding first `b`).
    /// Returns the number of hashes.
    pub(crate) fn eat_raw_double_quoted_string(
        &mut self,
        prefix_len: u32,
    ) -> Result<u8, RawStringError> {
        // wrap the actual function to handle the error with too many hashes
        // this way, it eats the whole raw string
        // (only up to 255 `#`s are allowed in raw strings)
        let n_hashes = self.eat_raw_string_body(prefix_len)?;
        match u8::try_from(n_hashes) {
            Ok(num) => Ok(num),
            Err(_) => Err(RawStringError::TooManyDelimiters {
                found_hashes: n_hashes,
            }),
        }
    }

    /// Parses a raw string (with hashes, excluding first `r`).
    /// Returns the number of hashes.
    pub(crate) fn eat_raw_string_body(&mut self, prefix_len: u32) -> Result<u32, RawStringError> {
        debug_assert!(self.prev() == 'r');
        let start_pos = self.get_pos_within_token();
        let mut possible_terminator_offset: Option<u32> = None;
        let mut max_hashes = 0;

        // count opening '#' symbols
        let mut eaten = 0;
        while self.peek() == '#' {
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

        // skip the string contents and on each '#' character met
        // check if this is a raw string termination
        loop {
            self.eat_until(b'"');

            if self.is_end() {
                return Err(RawStringError::NoTerminator {
                    expected_hashes: n_start_hashes,
                    found_hashes: max_hashes,
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
            while self.peek() == '#' && n_end_hashes < n_start_hashes {
                n_end_hashes += 1;
                self.bump();
            }

            if n_end_hashes == n_start_hashes {
                return Ok(n_start_hashes);
            } else if n_end_hashes > max_hashes {
                // keep track of possible terminators to give a hint about
                // where there might be a missing terminator
                possible_terminator_offset =
                    Some(self.get_pos_within_token() - start_pos - n_end_hashes + prefix_len);
                max_hashes = n_end_hashes;
            }
        }
    }

    /// Parses decimal digits.
    /// Returns whether any digits were parsed.
    pub(crate) fn eat_decimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.peek() {
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
    /// Returns whether any digits were parsed.
    pub(crate) fn eat_hexadecimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.peek() {
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

    /// Parses the float exponent (excluding `e` or `E`).
    /// Returns whether the exponent is non-empty.
    pub(crate) fn eat_float_exponent(&mut self) -> bool {
        debug_assert!(self.prev() == 'e' || self.prev() == 'E');
        if self.peek() == '-' || self.peek() == '+' {
            self.bump();
        }
        self.eat_decimal_digits()
    }

    /// Parses a block comment body with nesting support.
    /// Assumes the initial `/*` has been seen (the `/` is already consumed and `*` consumed by caller).
    pub(crate) fn eat_block_comment_body(&mut self) {
        let mut depth: u32 = 1;
        while !self.is_end() {
            let bytes = self.as_str().as_bytes();
            if bytes.len() >= 2 {
                // start of nested block comment
                if bytes[0] == b'/' && bytes[1] == b'*' {
                    // consume '/*'
                    self.bump();
                    self.bump();
                    depth = depth.saturating_add(1);
                    continue;
                }
                // end of current block comment level
                if bytes[0] == b'*' && bytes[1] == b'/' {
                    // consume '*/'
                    self.bump();
                    self.bump();
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
            }
            // consume a single character and continue
            let _ = self.bump();
        }
    }
}
