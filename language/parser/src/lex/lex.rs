use std::sync::Arc;

use super::html_entities::HTML_NAMED_ENTITIES;
use super::lexer::Lexer;
use destack_dir::{
    Comment, NumberBase, Token, TokenLiteral, TokenSpan, TokenType, is_identifier_continue,
    is_identifier_start, is_whitespace,
};

use destack_source::{File, LanguageType};
use destack_unicode::UnicodeEmoji;

/// Result of lexing one complete source file.
#[derive(Debug)]
pub struct LexResult {
    /// The semantic tokens (identifiers, keywords, literals, operators).
    pub tokens: Vec<TokenSpan>,
    /// The non-semantic tokens (whitespace, comments).
    pub side_tokens: Vec<TokenSpan>,
    /// The structured comments collected during lexing.
    pub comments: Vec<Comment>,
    /// The end-of-file token.
    pub eof_token: TokenSpan,
}

pub const TRIVIA_TOKEN_TYPES: [TokenType; 6] = [
    TokenType::Newline,
    TokenType::Whitespace,
    TokenType::LineComment,
    TokenType::BlockComment,
    TokenType::DocLineComment,
    TokenType::DocBlockComment,
];

pub const EXPRESSION_START_TOKEN_TYPES: &[TokenType] = &[
    TokenType::Assign,
    TokenType::Comma,
    TokenType::Colon,
    TokenType::Semicolon,
    TokenType::Spread,
    TokenType::Range,
    TokenType::RangeInclusive,
    TokenType::Not,
    TokenType::ElementwiseNot,
    TokenType::Multiply,
    TokenType::Exponent,
    TokenType::Divide,
    TokenType::Remainder,
    TokenType::Add,
    TokenType::Subtract,
    TokenType::ShiftLeft,
    TokenType::ShiftRight,
    TokenType::UnsignedShiftRight,
    TokenType::ElementwiseAnd,
    TokenType::ElementwiseXor,
    TokenType::ElementwiseOr,
    TokenType::Equal,
    TokenType::NotEqual,
    TokenType::EqualWide,
    TokenType::NotEqualWide,
    TokenType::LessThan,
    TokenType::LessThanOrEqual,
    TokenType::GreaterThan,
    TokenType::GreaterThanOrEqual,
    TokenType::LogicalAnd,
    TokenType::LogicalOr,
    TokenType::Maybe,
    TokenType::Coalesce,
    TokenType::OpenParenthesis,
    TokenType::OpenBracket,
    TokenType::OpenBrace,
    TokenType::TemplateStringStart,
    TokenType::TemplateStringMiddle,
    TokenType::Arrow,
    TokenType::ArrowWide,
    TokenType::MultiplyAssign,
    TokenType::ExponentAssign,
    TokenType::DivideAssign,
    TokenType::RemainderAssign,
    TokenType::AddAssign,
    TokenType::SubtractAssign,
    TokenType::ShiftLeftAssign,
    TokenType::ShiftRightAssign,
    TokenType::UnsignedShiftRightAssign,
    TokenType::ElementwiseAndAssign,
    TokenType::ElementwiseXorAssign,
    TokenType::ElementwiseOrAssign,
    TokenType::LogicalAndAssign,
    TokenType::LogicalOrAssign,
    TokenType::CoalesceAssign,
];

/// Check if a token is semantic (not whitespace or comment).
/// Optimized for fast inline checking.
#[inline]
pub fn is_semantic(token_type: TokenType) -> bool {
    !matches!(
        token_type,
        TokenType::Newline
            | TokenType::Whitespace
            | TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
}

/// Return true when the character is a line terminator.
#[inline]
fn is_line_terminator_char(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}')
}

/// Return true when a byte can continue an ascii identifier.
#[inline]
fn is_ascii_identifier_continue_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

/// Return true when a byte is non-newline ascii whitespace.
#[inline]
fn is_ascii_non_newline_whitespace_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | 0x0B | 0x0C)
}

impl Lexer {
    /// Return whether `.e` or `.E` starts a decimal exponent after a dot.
    #[inline]
    fn dot_starts_decimal_exponent(&self) -> bool {
        let exponent_marker = self.peek_next();
        let exponent_head = self.peek_next_next();
        (exponent_marker == 'e' || exponent_marker == 'E')
            && (exponent_head.is_ascii_digit() || exponent_head == '+' || exponent_head == '-')
    }

    /// Lex the input string into semantic tokens, side tokens, and the end-of-sequence Token.
    /// Semantic tokens are identifiers, keywords, literals, operators.
    /// Side tokens are whitespace and comments.
    pub fn lex(
        file: Arc<File>,
        language: LanguageType,
    ) -> (Vec<TokenSpan>, Vec<TokenSpan>, TokenSpan) {
        let result = Self::lex_file(file, language);
        (result.tokens, result.side_tokens, result.eof_token)
    }

    /// Lex the input string and return token buffers.
    pub fn lex_file(file: Arc<File>, language: LanguageType) -> LexResult {
        let mut lexer = Lexer::new(file, language);
        lexer.lex_to_result()
    }

    /// Lex the input string with trivia retention configured.
    pub fn lex_with_options(
        file: Arc<File>,
        language: LanguageType,
        retain_trivia_tokens: bool,
    ) -> LexResult {
        let mut lexer = Lexer::new(file, language);
        lexer.set_retain_trivia_tokens(retain_trivia_tokens);
        lexer.lex_to_result()
    }

    /// Finish lexing and extract the stream buffers.
    fn lex_to_result(&mut self) -> LexResult {
        self.lex_to_end();

        let eof_token = self.eof_token();
        let comments = self.take_trivia_comments();
        let (tokens, side_tokens) = self.take_tokens();

        LexResult {
            tokens,
            side_tokens,
            comments,
            eof_token,
        }
    }

    /// Parses a token from the input string.
    pub(super) fn advance(&mut self) -> Token {
        self.last_side_token_had_line_terminator = false;

        // eat first character until nothing is left (=EOF)
        let Some(first_char) = self.eat() else {
            return Token::new(TokenType::End, 0, None);
        };

        // parse token
        let (token_type, literal) = match first_char {
            // whitespace
            c if is_whitespace(c) => {
                if is_line_terminator_char(c) {
                    // normalize CRLF and CR newlines as one newline token
                    if c == '\r' && self.peek() == '\n' {
                        self.eat();
                    }
                    (TokenType::Newline, None)
                } else {
                    (self.eat_whitespace(), None)
                }
            }

            // slash, comments, regex, or divide ops
            '/' => {
                let bytes = self.remaining_text().as_bytes();
                let next = bytes.first().copied();
                match next {
                    // //
                    Some(b'/') => {
                        // doc line comment if exactly three slashes and the fourth is not '/'
                        let third_is_slash = bytes.get(1).copied() == Some(b'/');
                        let fourth_is_slash = bytes.get(2).copied() == Some(b'/');
                        let is_doc_line = third_is_slash && !fourth_is_slash;
                        self.eat_until(b'\n');
                        self.last_side_token_had_line_terminator = true;
                        if is_doc_line {
                            (TokenType::DocLineComment, None)
                        } else {
                            (TokenType::LineComment, None)
                        }
                    }
                    // /*
                    // block comments starting with '/*'
                    Some(b'*') => {
                        // detect doc block comment for exactly '/**' (not '/***')
                        let third_is_star = bytes.get(1).copied() == Some(b'*');
                        let fourth_is_star = bytes.get(2).copied() == Some(b'*');
                        let is_doc_block = third_is_star && !fourth_is_star;
                        // consume the initial '*'
                        self.eat();
                        // doc block comments do not nest
                        let (is_terminated, has_line_terminator) = if is_doc_block {
                            self.eat_doc_block_comment()
                        } else {
                            self.eat_block_comment()
                        };
                        self.last_side_token_had_line_terminator = has_line_terminator;
                        // unterminated comment is an error
                        if !is_terminated {
                            (TokenType::Unknown, None)
                        } else if is_doc_block {
                            (TokenType::DocBlockComment, None)
                        } else {
                            (TokenType::BlockComment, None)
                        }
                    }
                    _ => {
                        if self.peek() == '=' {
                            self.eat();
                            (TokenType::DivideAssign, None)
                        } else {
                            (TokenType::Divide, None)
                        }
                    }
                }
            }

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
                // ...
                if self.peek() == '.' && self.peek_next() == '.' {
                    self.eat();
                    self.eat();
                    (TokenType::Spread, None)
                }
                // ..=
                else if self.language.is_destack()
                    && self.peek() == '.'
                    && self.peek_next() == '='
                {
                    self.eat();
                    self.eat();
                    (TokenType::RangeInclusive, None)
                }
                // ..
                else if self.language.is_destack() && self.peek() == '.' {
                    self.eat();
                    (TokenType::Range, None)
                }
                // decimal literal starting with .
                else if self.peek().is_ascii_digit() {
                    let literal = self.eat_leading_dot_number_literal();
                    (TokenType::Literal, Some(literal))
                }
                // .
                else {
                    (TokenType::Dot, None)
                }
            }
            '@' => (TokenType::At, None),
            '#' => {
                // hashbang prefix to a file
                let is_hashbang = self.position() == 1
                    && self.peek() == '!'
                    && (self.language.is_javascript() || self.language.is_typescript());
                if is_hashbang {
                    self.eat(); // eat !
                    self.eat_until(b'\n');
                    self.last_side_token_had_line_terminator = true;
                    (TokenType::LineComment, None)
                } else {
                    (TokenType::Hash, None)
                }
            }
            '~' => (TokenType::ElementwiseNot, None),
            '?' => {
                // ??
                if self.peek() == '?' {
                    self.eat();
                    // ??=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::CoalesceAssign, None)
                    }
                    // ??
                    else {
                        (TokenType::Coalesce, None)
                    }
                }
                // ?
                else {
                    (TokenType::Maybe, None)
                }
            }

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

                // we're at the end of a template string interpolation
                if self.options.template_string_stack.peek() == Some(self.options.parentheses_depth)
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
                if self.peek() == '&' {
                    self.eat();
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::LogicalAndAssign, None)
                    } else {
                        (TokenType::LogicalAnd, None)
                    }
                } else if self.peek() == '=' {
                    self.eat();
                    (TokenType::ElementwiseAndAssign, None)
                } else {
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
                if self.peek() == '<' {
                    self.eat();
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::ShiftLeftAssign, None)
                    } else {
                        (TokenType::ShiftLeft, None)
                    }
                } else if self.peek() == '=' {
                    self.eat();
                    (TokenType::LessThanOrEqual, None)
                } else {
                    (TokenType::LessThan, None)
                }
            }

            // greater than or shift right
            '>' => {
                if self.peek() == '>' && self.peek_next() == '=' {
                    self.eat(); // >
                    self.eat(); // =
                    (TokenType::ShiftRightAssign, None)
                } else if self.peek() == '>'
                    && self.peek_next() == '>'
                    && self.peek_next_next() == '='
                {
                    self.eat(); // >
                    self.eat(); // >
                    self.eat(); // =
                    (TokenType::UnsignedShiftRightAssign, None)
                } else if self.peek() == '>' && self.peek_next() == '>' {
                    self.eat(); // >
                    self.eat(); // >
                    (TokenType::UnsignedShiftRight, None)
                } else if self.peek() == '>' {
                    self.eat(); // >
                    (TokenType::ShiftRight, None)
                } else if self.peek() == '=' {
                    self.eat();
                    (TokenType::GreaterThanOrEqual, None)
                } else {
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
                // +=
                if self.peek() == '=' {
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
                // ** and **=
                if self.peek() == '*' {
                    self.eat();
                    // **=
                    if self.peek() == '=' {
                        self.eat();
                        (TokenType::ExponentAssign, None)
                    }
                    // **
                    else {
                        (TokenType::Exponent, None)
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

            // string literal
            '\'' => {
                let (is_terminated, has_invalid_escape) = self.eat_quoted_string('\'');
                let kind = TokenLiteral::String {
                    is_terminated,
                    has_invalid_escape,
                };
                (TokenType::Literal, Some(kind))
            }

            // string literal
            '"' => {
                let (terminated, has_invalid_escape) = self.eat_quoted_string('"');
                let kind = TokenLiteral::String {
                    is_terminated: terminated,
                    has_invalid_escape,
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

            // backslash - could be unicode escape starting an identifier (\uXXXX or \u{...})
            '\\' => {
                if let Some(token) = self.try_eat_unicode_escape_identifier() {
                    token
                } else {
                    (TokenType::Unknown, None)
                }
            }

            _ => (TokenType::Unknown, None),
        };

        if is_semantic(token_type) {
            self.options.in_tree_attribute_value = false;
        }

        let token = Token::new(token_type, self.token_len(), literal);
        self.reset_token_start();
        token
    }

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

    /// Eat non-newline ascii whitespace bytes.
    #[inline]
    fn eat_ascii_non_newline_whitespace(&mut self) {
        let bytes = self.remaining_text().as_bytes();
        let mut index = 0usize;
        while index < bytes.len() && is_ascii_non_newline_whitespace_byte(bytes[index]) {
            index += 1;
        }

        if index > 0 {
            self.advance_ascii_bytes(index, bytes[index - 1]);
        }
    }

    /// Parses a whitespace sequence (excluding first character).
    fn eat_whitespace(&mut self) -> TokenType {
        debug_assert!(is_whitespace(self.previous()));

        // fast path: consume contiguous ascii spaces and tabs in bulk
        self.eat_ascii_non_newline_whitespace();

        // unicode whitespace tail
        self.eat_while(|c| is_whitespace(c) && !is_line_terminator_char(c));
        TokenType::Whitespace
    }

    /// Parses an identifier, unknown prefix or some literal string (excluding first character).
    /// Returns the token type and the literal type if it's a hardcoded literal.
    fn eat_identifier_or_such(&mut self, first_char: char) -> (TokenType, Option<TokenLiteral>) {
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
            && self.is_valid_identifier_continue_unicode_escape_ahead()
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

    /// Try to parse a unicode escape sequence that starts an identifier.
    /// Called after `\` has been eaten.
    /// Returns Some((TokenType::Identifier, None)) if successful, None otherwise.
    fn try_eat_unicode_escape_identifier(&mut self) -> Option<(TokenType, Option<TokenLiteral>)> {
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
                if !self.is_valid_identifier_continue_unicode_escape_ahead() {
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
            self.eat_invalid_identifier_unicode_escape_body_prefix();
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
    fn eat_invalid_identifier_unicode_escape_body_prefix(&mut self) {
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

    /// Check if there's a valid identifier-continuation unicode escape at current position.
    /// Does NOT consume any characters, just peeks ahead.
    fn is_valid_identifier_continue_unicode_escape_ahead(&self) -> bool {
        self.peek_unicode_escape_ahead_char()
            .is_some_and(is_identifier_continue)
    }

    /// Parses a number literal (excluding first digit).
    /// Returns the number literal.
    fn eat_number_literal(&mut self, first_digit: char) -> TokenLiteral {
        debug_assert!('0' <= self.previous() && self.previous() <= '9');
        let mut base = NumberBase::Decimal;
        if first_digit == '0' {
            // parse encoding base
            match self.peek() {
                // binary literal
                'b' | 'B' => {
                    base = NumberBase::Binary;
                    self.eat();
                    if !self.eat_decimal_digits() {
                        return TokenLiteral::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // octal literal
                'o' | 'O' => {
                    base = NumberBase::Octal;
                    self.eat();
                    if !self.eat_decimal_digits() {
                        return TokenLiteral::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // hexadecimal literal
                'x' | 'X' => {
                    base = NumberBase::Hexadecimal;
                    self.eat();
                    if !self.eat_hexadecimal_digits() {
                        return TokenLiteral::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // not a base prefix; consume additional digits
                '0'..='9' | '_' => {
                    self.eat_decimal_digits();
                }

                // also not a base prefix; nothing more to do here
                '.' | 'e' | 'E' | 'n' => {}

                // just a 0
                _ => {
                    return TokenLiteral::Int {
                        base,
                        is_empty: false,
                        is_bigint: false,
                    };
                }
            }
        } else {
            // no base prefix, parse number in the usual way
            self.eat_decimal_digits();
        }

        match self.peek() {
            // js and ts decimal member syntax: `123..prop` and `0..prop`
            // consume the first dot into a float literal so the second dot can start member access
            '.' if self.peek_next() == '.'
                && is_identifier_start(self.peek_next_next())
                && (self.language.is_javascript() || self.language.is_typescript()) =>
            {
                self.eat();
                TokenLiteral::Float {
                    base,
                    is_empty_exponent: false,
                }
            }

            // don't be greedy if this is actually an
            // integer literal followed by field or method access
            // (`12.foo()` and `12..toString()`)
            '.' if self.peek_next() != '.'
                && (!is_identifier_start(self.peek_next())
                    || self.dot_starts_decimal_exponent()) =>
            {
                // might have stuff after the ., and if it does, it starts with a number
                self.eat();
                let mut is_empty_exponent = false;

                if self.peek().is_ascii_digit() {
                    self.eat_decimal_digits();
                }

                // allow exponent forms without a fractional part (`1.e1`)
                if self.peek() == 'e' || self.peek() == 'E' {
                    self.eat();
                    is_empty_exponent = !self.eat_float_exponent();
                }

                TokenLiteral::Float {
                    base,
                    is_empty_exponent,
                }
            }
            'e' | 'E' => {
                self.eat();
                let is_empty_exponent = !self.eat_float_exponent();
                TokenLiteral::Float {
                    base,
                    is_empty_exponent,
                }
            }
            'n' => {
                self.eat();
                TokenLiteral::Int {
                    base,
                    is_empty: false,
                    is_bigint: true,
                }
            }
            _ => TokenLiteral::Int {
                base,
                is_empty: false,
                is_bigint: false,
            },
        }
    }

    /// Parse a decimal literal starting with a leading dot.
    fn eat_leading_dot_number_literal(&mut self) -> TokenLiteral {
        let base = NumberBase::Decimal;
        let is_empty_exponent = {
            self.eat_decimal_digits();
            if matches!(self.peek(), 'e' | 'E') {
                self.eat();
                !self.eat_float_exponent()
            } else {
                false
            }
        };
        TokenLiteral::Float {
            base,
            is_empty_exponent,
        }
    }

    /// Return true when quoted strings can span lines in tree opening tag attributes.
    #[inline]
    fn allow_line_terminator_in_tree_attribute_string(&self) -> bool {
        self.options.in_tree_attribute_value
    }

    /// Parse a quoted string literal after its opening quote.
    fn eat_quoted_string(&mut self, quote: char) -> (bool, bool) {
        debug_assert!(self.previous() == quote);

        let mut has_invalid_escape = false;

        // parse until either quotes are terminated or EOF is reached
        while !self.is_end() {
            match self.peek() {
                // quotes are terminated, finish parsing
                c if c == quote => {
                    self.eat();
                    return (true, has_invalid_escape);
                }

                // line terminators are not allowed in quoted strings
                '\n' | '\r' => {
                    // allow multiline quoted values inside tree opening tag attributes
                    if self.allow_line_terminator_in_tree_attribute_string() {
                        self.eat();
                        continue;
                    }
                    return (false, has_invalid_escape);
                }

                // escape sequence
                '\\' => {
                    self.eat(); // eat '\'

                    // consume escape sequence and track invalid escapes
                    if self.eat_string_escape_sequence() {
                        has_invalid_escape = true;
                    }
                }

                // regular character
                _ => {
                    self.eat();
                }
            }
        }

        // end of file reached
        (false, has_invalid_escape)
    }

    /// Consume a string escape sequence after `\`.
    /// Returns true when the escape sequence is invalid.
    fn eat_string_escape_sequence(&mut self) -> bool {
        if self.is_end() {
            return true;
        }

        let escaped = self.peek();
        match escaped {
            // legacy escaped digits are invalid
            '8' | '9' => {
                self.eat();
                true
            }
            // legacy octal escapes are invalid without a following digit
            '0'..='7' => {
                self.eat();
                if escaped != '0' {
                    return true;
                }
                if self.peek().is_ascii_digit() {
                    return true;
                }
                false
            }
            // \uXXXX and \u{...}
            'u' => {
                self.eat(); // eat `u`
                self.eat_unicode_escape_after_u()
            }
            // \xXX
            'x' => {
                self.eat(); // eat `x`
                self.eat_fixed_hex_escape(2)
            }
            // regular escaped character
            _ => {
                self.eat();
                false
            }
        }
    }

    /// Consume a unicode escape sequence body after `\u`.
    /// Returns true when the sequence is invalid.
    fn eat_unicode_escape_after_u(&mut self) -> bool {
        if self.peek() == '{' {
            self.eat(); // eat `{`
            let mut digits = 0usize;
            let mut value: u32 = 0;
            let mut overflowed = false;
            while self.peek().is_ascii_hexdigit() {
                if !overflowed {
                    let digit = match self.peek() {
                        '0'..='9' => self.peek() as u32 - '0' as u32,
                        'a'..='f' => self.peek() as u32 - 'a' as u32 + 10,
                        'A'..='F' => self.peek() as u32 - 'A' as u32 + 10,
                        _ => unreachable!("checked ascii hex digit"),
                    };
                    if let Some(next) = value.checked_mul(16).and_then(|v| v.checked_add(digit)) {
                        value = next;
                    } else {
                        overflowed = true;
                    }
                }
                self.eat();
                digits += 1;
            }

            if digits == 0 || self.peek() != '}' {
                return true;
            }
            self.eat(); // eat `}`
            overflowed || value > 0x10FFFF
        } else {
            self.eat_fixed_hex_escape(4)
        }
    }

    /// Consume an exact number of hexadecimal digits.
    /// Returns true when the sequence is invalid.
    fn eat_fixed_hex_escape(&mut self, width: usize) -> bool {
        for _ in 0..width {
            if !self.peek().is_ascii_hexdigit() {
                return true;
            }
            self.eat();
        }
        false
    }

    /// Parse a template string (excluding first backtick).
    /// Returns whether the template ended before `${`.
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
                '\\' => {
                    let escaped = self.peek();

                    // skip the escaped code unit so `\${` stays literal text
                    if escaped != '\0' {
                        self.eat();
                    }
                }
                _ => (),
            }
        }

        false
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
        debug_assert!(self.previous() == 'e' || self.previous() == 'E');
        if self.peek() == '-' || self.peek() == '+' {
            self.eat();
        }
        self.eat_decimal_digits()
    }

    /// Parse a doc block comment body without nesting support.
    /// Assume the initial `/*` has been seen (the `/` is already consumed and `*` consumed by caller).
    /// Return whether the comment was terminated and whether it contained a line terminator.
    pub(crate) fn eat_doc_block_comment(&mut self) -> (bool, bool) {
        let mut has_line_terminator = false;

        // scan until the first closing delimiter
        while !self.is_end() {
            let bytes = self.remaining_text().as_bytes();
            if bytes.len() >= 2 && bytes[0] == b'*' && bytes[1] == b'/' {
                // consume "*/"
                self.eat();
                self.eat();
                return (true, has_line_terminator);
            }

            let first_byte = bytes[0];
            let is_ascii_line_terminator = first_byte == b'\n' || first_byte == b'\r';
            let is_unicode_line_terminator = first_byte == 0xE2
                && bytes.len() >= 3
                && bytes[1] == 0x80
                && (bytes[2] == 0xA8 || bytes[2] == 0xA9);
            if is_ascii_line_terminator || is_unicode_line_terminator {
                has_line_terminator = true;
            }

            // consume a single character and continue
            let _ = self.eat();
        }

        // reached EOF without closing comment
        (false, has_line_terminator)
    }

    /// Parse a block comment body.
    /// Assumes the initial `/*` has been seen (the `/` is already consumed and `*` consumed by caller).
    /// Returns whether the comment was terminated and whether it contained a line terminator.
    pub(crate) fn eat_block_comment(&mut self) -> (bool, bool) {
        let mut has_line_terminator = false;

        // stop at the first closing delimiter
        while !self.is_end() {
            let bytes = self.remaining_text().as_bytes();
            if bytes.len() >= 2 && bytes[0] == b'*' && bytes[1] == b'/' {
                self.eat();
                self.eat();
                return (true, has_line_terminator);
            }

            let first_byte = bytes[0];
            let is_ascii_line_terminator = first_byte == b'\n' || first_byte == b'\r';
            let is_unicode_line_terminator = first_byte == 0xE2
                && bytes.len() >= 3
                && bytes[1] == 0x80
                && (bytes[2] == 0xA8 || bytes[2] == 0xA9);
            if is_ascii_line_terminator || is_unicode_line_terminator {
                has_line_terminator = true;
            }

            let _ = self.eat();
        }

        (false, has_line_terminator)
    }
}

/// Decodes an HTML entity into a character.
pub fn decode_html_entity(entity: &str) -> Option<char> {
    if !entity.starts_with('&') || !entity.ends_with(';') {
        return None;
    }

    let body = &entity[1..entity.len() - 1];
    if body.is_empty() {
        return None;
    }

    if let Some(codepoint) = body.strip_prefix('#') {
        return decode_numeric_entity(codepoint);
    }

    decode_named_entity(body)
}

/// Decodes a numeric entity into a character (like `&#x1234;` or `&#1234;`).
#[inline]
fn decode_numeric_entity(codepoint: &str) -> Option<char> {
    let (radix, digits) = if let Some(hex_digits) = codepoint.strip_prefix(['x', 'X']) {
        (16, hex_digits)
    } else {
        (10, codepoint)
    };

    if digits.is_empty() {
        return None;
    }

    let is_valid_digits = if radix == 16 {
        digits.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        digits.chars().all(|c| c.is_ascii_digit())
    };

    if !is_valid_digits {
        return None;
    }

    let value = u32::from_str_radix(digits, radix).ok()?;
    char::from_u32(value)
}

/// Decodes a named entity into a character (like `&lt;` or `&amp;`).
#[inline]
fn decode_named_entity(name: &str) -> Option<char> {
    HTML_NAMED_ENTITIES
        .binary_search_by(|(entity_name, _)| (*entity_name).cmp(name))
        .ok()
        .map(|idx| HTML_NAMED_ENTITIES[idx].1)
}
