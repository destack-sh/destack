//! Low-level general purpose DS lexer (adapted from rustc).

use crate::{TokenSpan, is_identifier_continue, is_identifier_start, is_whitespace};
use std::str::FromStr;

use super::lexer::{EOF_CHAR, Lexer};
use dyst_ast::{Keyword, LiteralType, NumberBase, RawStringError, Token, TokenType};

use destack_unicode::UnicodeEmoji;
use dyst_source::{SourceId, Span};

/// Result of parsing a single-quoted literal.
enum SingleQuotedLiteral {
    Character { is_terminated: bool },
    String { is_terminated: bool },
}

pub const TRIVIA_TOKEN_TYPES: [TokenType; 5] = [
    TokenType::Whitespace,
    TokenType::LineComment,
    TokenType::BlockComment,
    TokenType::DocLineComment,
    TokenType::DocBlockComment,
];

pub const EXPRESSION_START_TOKEN_TYPES: [TokenType; 15] = [
    TokenType::Newline,
    TokenType::Assign,
    TokenType::Comma,
    TokenType::Semicolon,
    TokenType::Equal,
    TokenType::NotEqual,
    TokenType::EqualWide,
    TokenType::NotEqualWide,
    TokenType::ElementwiseAnd,
    TokenType::LogicalAnd,
    TokenType::LogicalOr,
    TokenType::LogicalAndAssign,
    TokenType::LogicalOrAssign,
    TokenType::OpenParenthesis,
    TokenType::OpenBracket,
];

#[inline]
pub fn is_semantic(token_type: TokenType) -> bool {
    !TRIVIA_TOKEN_TYPES.contains(&token_type)
}

impl Lexer<'_> {
    /// Lex the input string into TokenSpans and the end-of-sequence Token.
    pub fn lex(source_id: SourceId, input: &str) -> (Vec<TokenSpan>, TokenSpan) {
        let mut lexer = Lexer::new(source_id, input);
        let eof_token = lexer.run();
        (lexer.tokens, eof_token)
    }

    /// Runs the lexer until the end of the input string.
    /// Returns the end-of-sequence Token.
    fn run(&mut self) -> TokenSpan {
        // tokenize with spans
        loop {
            let start = self.pos as u32;
            let token = self.advance();
            let token_span = TokenSpan {
                token,
                span: Span {
                    source: self.source_id,
                    start,
                    end: (start + token.len),
                },
            };
            self.tokens.push(token_span);
            if token.ty == TokenType::End {
                break;
            }
        }

        // eof token
        *self.tokens.last().unwrap_or(&TokenSpan {
            span: Span {
                source: self.source_id,
                start: 0,
                end: 0,
            },
            token: Token::end(),
        })
    }

    /// Parses a token from the input string.
    fn advance(&mut self) -> Token {
        // eat first character until nothing is left (=EOF)
        let Some(first_char) = self.eat() else {
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

            // slash, comments, regex or divide ops
            '/' => {
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
                        self.eat();
                        self.eat_block_comment();
                        if is_doc_block {
                            (TokenType::DocBlockComment, None)
                        } else {
                            (TokenType::BlockComment, None)
                        }
                    }
                    // regex or divide
                    _ => {
                        // /regex/ if we're in a "start" context
                        // (and not at a `/>` on a line without a closing `/` to disambiguate trees)
                        let prev_non_whitespace_token = {
                            self.tokens
                                .iter()
                                .rev()
                                .find(|token| token.token.ty != TokenType::Whitespace)
                        };
                        let is_at_start = {
                            if let Some(prev_non_whitespace_token) = prev_non_whitespace_token {
                                EXPRESSION_START_TOKEN_TYPES
                                    .contains(&prev_non_whitespace_token.token.ty)
                                    || prev_non_whitespace_token.token.ty == TokenType::Identifier
                                        && Keyword::from_str(
                                            self.get_span_str(prev_non_whitespace_token.span),
                                        )
                                        .map(|k| k.is_control())
                                        .unwrap_or(false)
                            } else {
                                true
                            }
                        };
                        // tag end looks like `/>` without a closing `/` on the same line
                        let is_tag_end = {
                            if self.peek() != '>' {
                                false
                            } else {
                                // if we find a closing `/` on the same line, it's not a tag end
                                let mut found_closing_slash_on_line = false;
                                for c in self.as_str().chars() {
                                    if c == '/' {
                                        found_closing_slash_on_line = true;
                                        break;
                                    } else if c == '\n' {
                                        break;
                                    }
                                }
                                !found_closing_slash_on_line
                            }
                        };

                        if is_at_start && !is_tag_end {
                            let has_flags = self.eat_regex_string();
                            (
                                TokenType::Literal,
                                Some(LiteralType::RegexString { has_flags }),
                            )
                        }
                        // /=
                        else if self.peek() == '=' {
                            self.eat();
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
                    let literal = LiteralType::RawString {
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
                        this.eat();
                        let parsed = this.eat_single_quoted_string();
                        let is_terminated = match parsed {
                            SingleQuotedLiteral::Character { is_terminated }
                            | SingleQuotedLiteral::String { is_terminated } => is_terminated,
                        };
                        (
                            TokenType::Literal,
                            Some(LiteralType::Byte { is_terminated }),
                        )
                    }
                    // b"
                    // double-quoted byte string literal
                    ('"', _) => {
                        this.eat();
                        let is_terminated = this.eat_double_quoted_string();
                        (
                            TokenType::Literal,
                            Some(LiteralType::ByteString { is_terminated }),
                        )
                    }
                    // br" or br#
                    // raw double-quoted byte string literal
                    ('r', '"') | ('r', '#') => {
                        this.eat();
                        let raw_dq_string = this.eat_raw_double_quoted_string(2);
                        (
                            TokenType::Literal,
                            Some(LiteralType::RawByteString {
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
                        self.eat();
                        self.eat();
                        (TokenType::RangeWide, None)
                    }
                    // ..
                    else {
                        self.eat();
                        (TokenType::Range, None)
                    }
                }
                // .
                else {
                    (TokenType::Dot, None)
                }
            }
            '@' => (TokenType::At, None),
            '#' => (TokenType::Tag, None),
            '~' => (TokenType::ElementwiseNot, None),
            '?' => {
                // ??
                if self.peek() == '?' {
                    self.eat();
                    self.eat();
                    (TokenType::Coalesce, None)
                }
                // ?
                else {
                    (TokenType::Maybe, None)
                }
            }
            '$' => (TokenType::Dynamic, None),

            // brackets
            '(' => {
                self.options.parentheses_depth += 1;
                (TokenType::OpenParenthesis, None)
            }
            ')' => {
                self.options.parentheses_depth -= 1;
                (TokenType::CloseParenthesis, None)
            }
            '[' => {
                self.options.parentheses_depth += 1;
                (TokenType::OpenBracket, None)
            }
            ']' => {
                self.options.parentheses_depth -= 1;
                (TokenType::CloseBracket, None)
            }
            '{' => {
                self.options.parentheses_depth += 1;
                (TokenType::OpenBrace, None)
            }
            // closing brace or maybe start of template middle
            '}' => {
                self.options.parentheses_depth -= 1;

                // we're at the end of an interpolation, continue or end
                if self.options.template_string_stack.last()
                    == Some(&self.options.parentheses_depth)
                {
                    self.options.template_string_stack.pop();
                    let is_complete = self.eat_template_string();
                    if is_complete {
                        (TokenType::TemplateStringEnd, None)
                    } else {
                        // continue eating the template (after `${`, again)
                        self.options
                            .template_string_stack
                            .push(self.options.parentheses_depth);
                        self.options.parentheses_depth += 1; // for the opening `{` (again)
                        (TokenType::TemplateStringMiddle, None)
                    }
                } else {
                    (TokenType::CloseBrace, None)
                }
            }

            // bang
            '!' => {
                // !=
                if self.peek() == '=' {
                    self.eat();
                    // !==
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::NotEqualWide, None)
                    }
                    // !=
                    else {
                        (TokenType::NotEqual, None)
                    }
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
                    self.eat();
                    (TokenType::Arrow, None)
                }
                // -%
                else if self.peek() == '%' {
                    self.eat();
                    // -%=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::WrappingSubtractAssign, None)
                    }
                    // -%
                    else {
                        (TokenType::WrappingSubtract, None)
                    }
                }
                // -|
                else if self.peek() == '|' {
                    self.eat();
                    // -|=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::SaturatingSubtractAssign, None)
                    }
                    // -|
                    else {
                        (TokenType::SaturatingSubtract, None)
                    }
                }
                // -=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::SubtractAssign, None)
                }
                // --
                else if self.peek() == '-' {
                    self.eat();
                    (TokenType::Decrement, None)
                }
                // -
                else {
                    (TokenType::Subtract, None)
                }
            }

            // elementwise and, logical and and their assignments
            '&' => {
                // &&
                if self.peek() == '&' {
                    self.eat();
                    // &&=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::LogicalAndAssign, None)
                    }
                    // &&
                    else {
                        (TokenType::LogicalAnd, None)
                    }
                }
                // &=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::ElementwiseAndAssign, None)
                }
                // &
                else {
                    (TokenType::ElementwiseAnd, None)
                }
            }

            // elementwise or, logical or and their assignments
            '|' => {
                // ||
                if self.peek() == '|' {
                    self.eat();
                    // ||=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::LogicalOrAssign, None)
                    }
                    // ||
                    else {
                        (TokenType::LogicalOr, None)
                    }
                }
                // |=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::ElementwiseOrAssign, None)
                }
                // |
                else {
                    (TokenType::ElementwiseOr, None)
                }
            }

            // equal or assign
            '=' => {
                // =>
                if self.peek() == '>' {
                    self.eat();
                    (TokenType::ArrowWide, None)
                }
                // ==
                else if self.peek() == '=' {
                    self.eat();
                    // ===
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::EqualWide, None)
                    }
                    // ==
                    else {
                        (TokenType::Equal, None)
                    }
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
                    self.eat();
                    // <<|
                    if self.peek() == '|' {
                        self.eat();
                        // <<|=
                        if self.peek() == '=' {
                            self.eat();
                            (TokenType::SaturatingShiftLeftAssign, None)
                        }
                        // <<|
                        else {
                            (TokenType::SaturatingShiftLeft, None)
                        }
                    }
                    // <<=
                    else if self.peek() == '=' {
                        self.eat();
                        (TokenType::ShiftLeftAssign, None)
                    }
                    // <<
                    else {
                        (TokenType::ShiftLeft, None)
                    }
                }
                // <=
                else if self.peek() == '=' {
                    self.eat();
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
                if self.peek() == '>' && self.peek_next() == '=' {
                    self.eat(); // >
                    self.eat(); // =
                    (TokenType::ShiftRightAssign, None)
                }
                // >=
                else if self.peek() == '=' {
                    self.eat();
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
                    self.eat();
                    (TokenType::ElementwiseXorAssign, None)
                }
                // ^
                else {
                    (TokenType::ElementwiseXor, None)
                }
            }

            // add
            '+' => {
                // +%
                if self.peek() == '%' {
                    self.eat();
                    // +%=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::WrappingAddAssign, None)
                    }
                    // +%
                    else {
                        (TokenType::WrappingAdd, None)
                    }
                }
                // +|
                else if self.peek() == '|' {
                    self.eat();
                    // +|=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::SaturatingAddAssign, None)
                    }
                    // +|
                    else {
                        (TokenType::SaturatingAdd, None)
                    }
                }
                // +=
                else if self.peek() == '=' {
                    self.eat();
                    (TokenType::AddAssign, None)
                }
                // ++
                else if self.peek() == '+' {
                    self.eat();
                    (TokenType::Increment, None)
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
                    self.eat();
                    // *%=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::WrappingMultiplyAssign, None)
                    }
                    // *%
                    else {
                        (TokenType::WrappingMultiply, None)
                    }
                }
                // *|
                else if self.peek() == '|' {
                    self.eat();
                    // *|=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::SaturatingMultiplyAssign, None)
                    }
                    // *|
                    else {
                        (TokenType::SaturatingMultiply, None)
                    }
                }
                // *=
                else if self.peek() == '=' {
                    self.eat();
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
                    self.eat();
                    (TokenType::RemainderAssign, None)
                }
                // %
                else {
                    (TokenType::Remainder, None)
                }
            }

            // character literal (with fallback to string literal for #Leniency)
            '\'' => match self.eat_single_quoted_string() {
                SingleQuotedLiteral::Character { is_terminated } => {
                    let kind = LiteralType::Character { is_terminated };
                    (TokenType::Literal, Some(kind))
                }
                SingleQuotedLiteral::String { is_terminated } => {
                    let kind = LiteralType::String { is_terminated };
                    (TokenType::Literal, Some(kind))
                }
            },

            // string literal
            '"' => {
                let terminated = self.eat_double_quoted_string();
                let kind = LiteralType::String {
                    is_terminated: terminated,
                };
                (TokenType::Literal, Some(kind))
            }

            // template string literal
            '`' => {
                let is_complete = self.eat_template_string();
                if is_complete {
                    (TokenType::TemplateString, None)
                } else {
                    self.options
                        .template_string_stack
                        .push(self.options.parentheses_depth);
                    self.options.parentheses_depth += 1; // for the opening `${`
                    (TokenType::TemplateStringStart, None)
                }
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
    fn eat_identifier_or_such(&mut self, first_char: char) -> (TokenType, Option<LiteralType>) {
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
        // boolean
        if first_char == 't' && self.source[start_pos - 1..self.pos].eq("true") {
            (
                TokenType::Literal,
                Some(LiteralType::Boolean { value: true }),
            )
        }
        // false
        else if first_char == 'f' && self.source[start_pos - 1..self.pos].eq("false") {
            (
                TokenType::Literal,
                Some(LiteralType::Boolean { value: false }),
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
    fn eat_number_literal(&mut self, first_digit: char) -> LiteralType {
        debug_assert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = NumberBase::Decimal;
        if first_digit == '0' {
            // parse encoding base
            match self.peek() {
                // binary literal
                'b' => {
                    base = NumberBase::Binary;
                    self.eat();
                    if !self.eat_decimal_digits() {
                        return LiteralType::Int {
                            base,
                            is_empty: true,
                        };
                    }
                }

                // octal literal
                'o' => {
                    base = NumberBase::Octal;
                    self.eat();
                    if !self.eat_decimal_digits() {
                        return LiteralType::Int {
                            base,
                            is_empty: true,
                        };
                    }
                }

                // hexadecimal literal
                'x' => {
                    base = NumberBase::Hexadecimal;
                    self.eat();
                    if !self.eat_hexadecimal_digits() {
                        return LiteralType::Int {
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
                    return LiteralType::Int {
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
                self.eat();
                let mut is_empty_exponent = false;
                if self.peek().is_ascii_digit() {
                    self.eat_decimal_digits();
                    match self.peek() {
                        'e' | 'E' => {
                            self.eat();
                            is_empty_exponent = !self.eat_float_exponent();
                        }
                        _ => (),
                    }
                }
                LiteralType::Float {
                    base,
                    is_empty_exponent,
                }
            }
            'e' | 'E' => {
                self.eat();
                let is_empty_exponent = !self.eat_float_exponent();
                LiteralType::Float {
                    base,
                    is_empty_exponent,
                }
            }
            _ => LiteralType::Int {
                base,
                is_empty: false,
            },
        }
    }

    /// Parse a single-quoted literal (excluding the initial `'`).
    /// Might be a character if single-quoted length is 1 or a string otherwise.
    fn eat_single_quoted_string(&mut self) -> SingleQuotedLiteral {
        debug_assert!(self.prev() == '\'');

        let mut logical_len = 0_u32;

        // parse until either quotes are terminated or error is detected
        loop {
            match self.peek() {
                // quotes are terminated, finish parsing
                '\'' => {
                    self.eat();
                    return Self::finish_single_quoted_literal(logical_len, true);
                }
                // escaped slash is considered one character, so bump twice
                '\\' => {
                    self.eat();
                    if self.is_end() {
                        return Self::finish_single_quoted_literal(logical_len, false);
                    }
                    self.eat();
                    logical_len = logical_len.saturating_add(1);
                }
                // skip the character
                _ => {
                    self.eat();
                    logical_len = logical_len.saturating_add(1);
                }
            }
        }
    }

    #[inline]
    fn finish_single_quoted_literal(logical_len: u32, is_terminated: bool) -> SingleQuotedLiteral {
        if logical_len == 1 {
            SingleQuotedLiteral::Character { is_terminated }
        } else {
            SingleQuotedLiteral::String { is_terminated }
        }
    }

    /// Parses a double-quoted string (excluding first `"`).
    /// Returns whether the string is complete (i.e. not a true interpolation).
    fn eat_double_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '"');
        while let Some(c) = self.eat() {
            match c {
                '"' => {
                    return true;
                }
                '\\' if self.peek() == '\\' || self.peek() == '"' => {
                    // bump again to skip escaped character
                    self.eat();
                }
                _ => (),
            }
        }
        // end of file reached
        false
    }

    /// Parses a regex string (excluding first `/`, including any flags after `/`).
    /// Works exactly like JS/TS regex literals.
    fn eat_regex_string(&mut self) -> bool {
        debug_assert!(self.prev() == '/');
        // match until next '/'
        while let Some(c) = self.eat() {
            match c {
                '/' => {
                    break;
                }
                '\\' if self.peek() == '\\' || self.peek() == '/' => {
                    // bump again to skip escaped character
                    self.eat();
                }
                _ => (), // keep eating
            }
        }
        // flags are are any alpha characters immediately after the last '/'
        let mut has_flags = false;
        loop {
            if self.peek().is_ascii_alphabetic() {
                has_flags = true;
                self.eat();
            } else {
                break;
            }
        }
        has_flags
    }

    /// Parses a template string (excluding first ``).
    /// Returns whether it's the start or the full string.
    fn eat_template_string(&mut self) -> bool {
        while let Some(c) = self.eat() {
            match c {
                '`' => {
                    return true;
                }
                '$' if self.peek() == '{' => {
                    self.eat();
                    return false;
                }
                '\\' if self.peek() == '\\' || self.peek() == '`' => {
                    // bump again to skip escaped character
                    self.eat();
                }
                _ => (), // keep eating
            }
        }
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
        let n_hashes = self.eat_raw_string(prefix_len)?;
        match u8::try_from(n_hashes) {
            Ok(num) => Ok(num),
            Err(_) => Err(RawStringError::TooManyDelimiters {
                found_hashes: n_hashes,
            }),
        }
    }

    /// Parses a raw string (with hashes, excluding first `r`).
    /// Returns the number of hashes.
    pub(crate) fn eat_raw_string(&mut self, prefix_len: u32) -> Result<u32, RawStringError> {
        debug_assert!(self.prev() == 'r');
        let start_pos = self.get_pos_within_token();
        let mut possible_terminator_offset: Option<u32> = None;
        let mut max_hashes = 0;

        // count opening '#' symbols
        let mut eaten = 0;
        while self.peek() == '#' {
            eaten += 1;
            self.eat();
        }
        let n_start_hashes = eaten;

        // check that string is started
        match self.eat() {
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
            self.eat();

            // check that amount of closing '#' symbols
            // is equal to the amount of opening ones
            // note that this will not consume extra trailing `#` characters:
            // `r###"abcde"####` is lexed as a `RawStr { n_hashes: 3 }`
            // followed by a `#` token
            let mut n_end_hashes = 0;
            while self.peek() == '#' && n_end_hashes < n_start_hashes {
                n_end_hashes += 1;
                self.eat();
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
                    self.eat();
                }
                '0'..='9' => {
                    has_digits = true;
                    self.eat();
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
                    self.eat();
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    has_digits = true;
                    self.eat();
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
            self.eat();
        }
        self.eat_decimal_digits()
    }

    /// Parses a block comment body with nesting support.
    /// Assumes the initial `/*` has been seen (the `/` is already consumed and `*` consumed by caller).
    pub(crate) fn eat_block_comment(&mut self) {
        let mut depth: u32 = 1;
        while !self.is_end() {
            let bytes = self.as_str().as_bytes();
            if bytes.len() >= 2 {
                // start of nested block comment
                if bytes[0] == b'/' && bytes[1] == b'*' {
                    // consume '/*'
                    self.eat();
                    self.eat();
                    depth = depth.saturating_add(1);
                    continue;
                }
                // end of current block comment level
                if bytes[0] == b'*' && bytes[1] == b'/' {
                    // consume '*/'
                    self.eat();
                    self.eat();
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
            }
            // consume a single character and continue
            let _ = self.eat();
        }
    }
}
