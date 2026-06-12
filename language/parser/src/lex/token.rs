use super::identifier::keyword_from_identifier_bytes;
use super::lexer::Lexer;
use super::scanner::EOF_CHAR;
use destack_dir::{Token, TokenLiteral, TokenType, is_identifier_start, is_whitespace};
use destack_unicode::UnicodeEmoji;

/// Return true when the character is a line terminator.
#[inline]
fn is_line_terminator_char(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}')
}

/// Return true when a byte is non-newline ascii whitespace.
#[inline]
fn is_ascii_non_newline_whitespace_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | 0x0B | 0x0C)
}

impl Lexer {
    /// Parse one token from the input string.
    pub(super) fn read_source_token(&mut self) -> Token {
        self.last_side_token_had_line_terminator = false;
        let start = self.position() as u32;

        if self.scanner.is_end() {
            return Token::eof(start);
        };

        let first_byte = self.scanner.byte();
        if first_byte.is_ascii() {
            return self.read_ascii_source_token(start, first_byte);
        }

        let first_char = self.scanner.eat_char().unwrap_or(EOF_CHAR);
        let (token_type, literal) = self.read_unicode_source_token(first_char);

        self.finish_source_token(start, first_byte, token_type, literal)
    }

    /// Finish one source token from scanner state.
    #[inline(always)]
    fn finish_source_token(
        &mut self,
        start: u32,
        first_byte: u8,
        token_type: TokenType,
        literal: Option<TokenLiteral>,
    ) -> Token {
        let token = if token_type == TokenType::Identifier {
            let keyword = if first_byte.is_ascii_lowercase() {
                keyword_from_identifier_bytes(self.token_bytes())
            } else {
                None
            };

            Token::identifier(start, self.token_len(), keyword)
        } else if let Some(literal) = literal {
            Token::new(token_type, start, self.token_len(), Some(literal))
        } else {
            Token::simple(token_type, start, self.token_len())
        };
        self.reset_token_start();

        token
    }

    /// Finish one simple source token with known byte length.
    #[inline(always)]
    fn finish_simple_source_token(&mut self, start: u32, token_type: TokenType, len: u32) -> Token {
        self.reset_token_start();

        Token::simple(token_type, start, len)
    }

    /// Parse one ASCII token from its first byte.
    fn read_ascii_source_token(&mut self, start: u32, first_byte: u8) -> Token {
        debug_assert!(first_byte.is_ascii());
        self.scanner.advance_ascii_bytes(1, first_byte);

        match first_byte {
            // newline trivia
            b'\n' => self.finish_simple_source_token(start, TokenType::Newline, 1),
            b'\r' => {
                if self.scanner.byte() == b'\n' {
                    self.scanner.advance_ascii_byte();

                    return self.finish_simple_source_token(start, TokenType::Newline, 2);
                }

                self.finish_simple_source_token(start, TokenType::Newline, 1)
            }

            // ordinary trivia
            b' ' | b'\t' | 0x0B | 0x0C => {
                let token_type = self.eat_whitespace();

                self.finish_source_token(start, first_byte, token_type, None)
            }

            // identifiers and literals
            b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$' => {
                let (token_type, literal) = self.eat_identifier_like(first_byte as char);

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'0'..=b'9' => {
                let literal = self.eat_number_literal(first_byte as char);

                self.finish_source_token(start, first_byte, TokenType::Literal, Some(literal))
            }

            // punctuation
            b':' => self.finish_simple_source_token(start, TokenType::Colon, 1),
            b';' => self.finish_simple_source_token(start, TokenType::Semicolon, 1),
            b',' => self.finish_simple_source_token(start, TokenType::Comma, 1),
            b'@' => self.finish_simple_source_token(start, TokenType::At, 1),
            b'~' => self.finish_simple_source_token(start, TokenType::ElementwiseNot, 1),
            b'(' => {
                self.options.parentheses_depth += 1;

                self.finish_simple_source_token(start, TokenType::OpenParenthesis, 1)
            }
            b')' => {
                self.options.parentheses_depth -= 1;

                self.finish_simple_source_token(start, TokenType::CloseParenthesis, 1)
            }
            b'[' => {
                self.options.parentheses_depth += 1;

                self.finish_simple_source_token(start, TokenType::OpenBracket, 1)
            }
            b']' => {
                self.options.parentheses_depth -= 1;

                self.finish_simple_source_token(start, TokenType::CloseBracket, 1)
            }
            b'{' => {
                self.options.parentheses_depth += 1;

                self.finish_simple_source_token(start, TokenType::OpenBrace, 1)
            }
            b'}' => {
                let (token_type, literal) = self.read_close_brace_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'.' => {
                let (token_type, literal) = self.read_dot_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'#' => {
                let (token_type, literal) = self.read_hash_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'?' => {
                let (token_type, literal) = self.read_question_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'!' => {
                let (token_type, literal) = self.read_bang_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'-' => {
                let (token_type, literal) = self.read_minus_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'&' => {
                let (token_type, literal) = self.read_ampersand_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'|' => {
                let (token_type, literal) = self.read_pipe_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'=' => {
                let (token_type, literal) = self.read_equals_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'<' => {
                let (token_type, literal) = self.read_less_than_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'>' => {
                let (token_type, literal) = self.read_greater_than_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'^' => {
                let (token_type, literal) = self.read_caret_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'+' => {
                let (token_type, literal) = self.read_plus_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'*' => {
                let (token_type, literal) = self.read_star_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'%' => {
                let (token_type, literal) = self.read_percent_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'/' => {
                let (token_type, literal) = self.read_slash_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }

            // strings and escape identifiers
            b'\'' => {
                let (token_type, literal) = self.read_single_quote_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'"' => {
                let (token_type, literal) = self.read_double_quote_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'`' => {
                let (token_type, literal) = self.read_template_quote_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }
            b'\\' => {
                let (token_type, literal) = self.read_backslash_token();

                self.finish_source_token(start, first_byte, token_type, literal)
            }

            _ => self.finish_simple_source_token(start, TokenType::Unknown, 1),
        }
    }

    /// Parse one non-ASCII token from its first character.
    fn read_unicode_source_token(&mut self, first_char: char) -> (TokenType, Option<TokenLiteral>) {
        if is_whitespace(first_char) {
            if is_line_terminator_char(first_char) {
                if first_char == '\r' && self.scanner.byte() == b'\n' {
                    self.scanner.advance_ascii_byte();
                }

                return (TokenType::Newline, None);
            }

            return (self.eat_whitespace(), None);
        }

        if is_identifier_start(first_char) {
            return self.eat_identifier_like(first_char);
        }

        if first_char.is_emoji_char() {
            return (self.eat_invalid_identifier(), None);
        }

        (TokenType::Unknown, None)
    }

    /// Parse one slash token or comment.
    fn read_slash_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        let bytes = self.scanner.remaining_bytes();

        if bytes.first().copied() == Some(b'/') {
            let third_is_slash = bytes.get(1).copied() == Some(b'/');
            let fourth_is_slash = bytes.get(2).copied() == Some(b'/');
            let token_type = if third_is_slash && !fourth_is_slash {
                TokenType::DocLineComment
            } else {
                TokenType::LineComment
            };
            self.eat_until(b'\n');
            self.last_side_token_had_line_terminator = true;

            return (token_type, None);
        }

        if bytes.first().copied() == Some(b'*') {
            let third_is_star = bytes.get(1).copied() == Some(b'*');
            let fourth_is_star = bytes.get(2).copied() == Some(b'*');
            let is_doc_block = third_is_star && !fourth_is_star;
            self.scanner.advance_ascii_byte();
            let (is_terminated, has_line_terminator) = self.eat_block_comment();
            self.last_side_token_had_line_terminator = has_line_terminator;

            if !is_terminated {
                return (TokenType::Unknown, None);
            }

            if is_doc_block {
                return (TokenType::DocBlockComment, None);
            }

            return (TokenType::BlockComment, None);
        }

        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::DivideAssign, None);
        }

        (TokenType::Divide, None)
    }

    /// Parse one dot token or leading dot number.
    fn read_dot_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() == b'.' && self.scanner.byte_at(1) == b'.' {
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();

            return (TokenType::Spread, None);
        }

        if self.language.is_destack() && self.scanner.byte() == b'.' {
            self.scanner.advance_ascii_byte();

            if self.scanner.byte() == b'=' {
                self.scanner.advance_ascii_byte();

                return (TokenType::RangeInclusive, None);
            }

            return (TokenType::Range, None);
        }

        if self.scanner.byte().is_ascii_digit() {
            let literal = self.eat_leading_dot_number_literal();

            return (TokenType::Literal, Some(literal));
        }

        (TokenType::Dot, None)
    }

    /// Parse one hash token or hashbang comment.
    fn read_hash_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        let is_hashbang = self.position() == 1
            && self.scanner.byte() == b'!'
            && (self.language.is_javascript() || self.language.is_typescript());
        if !is_hashbang {
            return (TokenType::Hash, None);
        }

        self.scanner.advance_ascii_byte();
        self.eat_until(b'\n');
        self.last_side_token_had_line_terminator = true;

        (TokenType::LineComment, None)
    }

    /// Parse one question token.
    fn read_question_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() != b'?' {
            return (TokenType::Maybe, None);
        }

        self.scanner.advance_ascii_byte();
        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::CoalesceAssign, None);
        }

        (TokenType::Coalesce, None)
    }

    /// Parse one bang token.
    fn read_bang_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() != b'=' {
            return (TokenType::Not, None);
        }

        self.scanner.advance_ascii_byte();
        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::NotEqualWide, None);
        }

        (TokenType::NotEqual, None)
    }

    /// Parse one minus token.
    fn read_minus_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        match self.scanner.byte() {
            b'>' => {
                self.scanner.advance_ascii_byte();

                (TokenType::Arrow, None)
            }
            b'=' => {
                self.scanner.advance_ascii_byte();

                (TokenType::SubtractAssign, None)
            }
            b'-' => {
                self.scanner.advance_ascii_byte();

                (TokenType::Decrement, None)
            }
            _ => (TokenType::Subtract, None),
        }
    }

    /// Parse one ampersand token.
    fn read_ampersand_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() == b'&' {
            self.scanner.advance_ascii_byte();

            if self.scanner.byte() == b'=' {
                self.scanner.advance_ascii_byte();

                return (TokenType::LogicalAndAssign, None);
            }

            return (TokenType::LogicalAnd, None);
        }

        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::ElementwiseAndAssign, None);
        }

        (TokenType::ElementwiseAnd, None)
    }

    /// Parse one pipe token.
    fn read_pipe_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() == b'|' {
            self.scanner.advance_ascii_byte();

            if self.scanner.byte() == b'=' {
                self.scanner.advance_ascii_byte();

                return (TokenType::LogicalOrAssign, None);
            }

            return (TokenType::LogicalOr, None);
        }

        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::ElementwiseOrAssign, None);
        }

        (TokenType::ElementwiseOr, None)
    }

    /// Parse one close brace token or template continuation.
    fn read_close_brace_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        self.options.parentheses_depth -= 1;

        if self.options.template_string_stack.peek() != Some(self.options.parentheses_depth) {
            return (TokenType::CloseBrace, None);
        }

        self.options.template_string_stack.pop();
        let is_complete = self.eat_template_string();
        if is_complete {
            return (TokenType::TemplateStringEnd, None);
        }

        // reopen the interpolation expression after `${`
        self.options
            .template_string_stack
            .push(self.options.parentheses_depth);
        self.options.parentheses_depth += 1;

        (TokenType::TemplateStringMiddle, None)
    }

    /// Parse one equals token.
    fn read_equals_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        match self.scanner.byte() {
            b'>' => {
                self.scanner.advance_ascii_byte();

                (TokenType::ArrowWide, None)
            }
            b'=' => {
                self.scanner.advance_ascii_byte();

                if self.scanner.byte() == b'=' {
                    self.scanner.advance_ascii_byte();

                    return (TokenType::EqualWide, None);
                }

                (TokenType::Equal, None)
            }
            _ => (TokenType::Assign, None),
        }
    }

    /// Parse one less-than token.
    fn read_less_than_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        match self.scanner.byte() {
            b'<' => {
                self.scanner.advance_ascii_byte();

                if self.scanner.byte() == b'=' {
                    self.scanner.advance_ascii_byte();

                    return (TokenType::ShiftLeftAssign, None);
                }

                (TokenType::ShiftLeft, None)
            }
            b'=' => {
                self.scanner.advance_ascii_byte();

                (TokenType::LessThanOrEqual, None)
            }
            _ => (TokenType::LessThan, None),
        }
    }

    /// Parse one greater-than token.
    fn read_greater_than_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() == b'>' && self.scanner.byte_at(1) == b'=' {
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();

            return (TokenType::ShiftRightAssign, None);
        }

        if self.scanner.byte() == b'>'
            && self.scanner.byte_at(1) == b'>'
            && self.scanner.byte_at(2) == b'='
        {
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();

            return (TokenType::UnsignedShiftRightAssign, None);
        }

        if self.scanner.byte() == b'>' && self.scanner.byte_at(1) == b'>' {
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();

            return (TokenType::UnsignedShiftRight, None);
        }

        if self.scanner.byte() == b'>' {
            self.scanner.advance_ascii_byte();

            return (TokenType::ShiftRight, None);
        }

        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::GreaterThanOrEqual, None);
        }

        (TokenType::GreaterThan, None)
    }

    /// Parse one caret token.
    fn read_caret_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::ElementwiseXorAssign, None);
        }

        (TokenType::ElementwiseXor, None)
    }

    /// Parse one plus token.
    fn read_plus_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        match self.scanner.byte() {
            b'=' => {
                self.scanner.advance_ascii_byte();

                (TokenType::AddAssign, None)
            }
            b'+' => {
                self.scanner.advance_ascii_byte();

                (TokenType::Increment, None)
            }
            _ => (TokenType::Add, None),
        }
    }

    /// Parse one star token.
    fn read_star_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() == b'*' {
            self.scanner.advance_ascii_byte();

            if self.scanner.byte() == b'=' {
                self.scanner.advance_ascii_byte();

                return (TokenType::ExponentAssign, None);
            }

            return (TokenType::Exponent, None);
        }

        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::MultiplyAssign, None);
        }

        (TokenType::Multiply, None)
    }

    /// Parse one percent token.
    fn read_percent_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::RemainderAssign, None);
        }

        (TokenType::Remainder, None)
    }

    /// Parse one single-quoted string token.
    fn read_single_quote_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        let (is_terminated, has_invalid_escape) = self.eat_quoted_string('\'');
        let literal = TokenLiteral::String {
            is_terminated,
            has_invalid_escape,
        };

        (TokenType::Literal, Some(literal))
    }

    /// Parse one double-quoted string token.
    fn read_double_quote_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        let (is_terminated, has_invalid_escape) = self.eat_quoted_string('"');
        let literal = TokenLiteral::String {
            is_terminated,
            has_invalid_escape,
        };

        (TokenType::Literal, Some(literal))
    }

    /// Parse one template string token.
    fn read_template_quote_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        let is_complete = self.eat_template_string();
        if is_complete {
            return (TokenType::TemplateString, None);
        }

        // open the interpolation expression after `${`
        self.options
            .template_string_stack
            .push(self.options.parentheses_depth);
        self.options.parentheses_depth += 1;

        (TokenType::TemplateStringStart, None)
    }

    /// Parse one unicode-escape identifier or unknown token.
    fn read_backslash_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if let Some(token) = self.try_eat_unicode_escape_identifier() {
            return token;
        }

        (TokenType::Unknown, None)
    }

    /// Eat non-newline ascii whitespace bytes.
    #[inline]
    fn eat_ascii_non_newline_whitespace(&mut self) {
        let bytes = self.scanner.remaining_bytes();
        let mut index = 0usize;
        while index < bytes.len() && is_ascii_non_newline_whitespace_byte(bytes[index]) {
            index += 1;
        }

        if index > 0 {
            self.advance_ascii_bytes(index, bytes[index - 1]);
        }
    }

    /// Parse a whitespace sequence after its first character.
    fn eat_whitespace(&mut self) -> TokenType {
        debug_assert!(is_whitespace(self.previous()));

        // consume contiguous ascii spaces and tabs in bulk
        self.eat_ascii_non_newline_whitespace();

        // unicode whitespace tail
        self.eat_while(|c| is_whitespace(c) && !is_line_terminator_char(c));
        TokenType::Whitespace
    }

    /// Parse a block comment body.
    ///
    /// Assumes the initial `/*` has been seen: the `/` is consumed by dispatch
    /// and the `*` is consumed by the caller.
    /// Returns whether the comment was terminated and whether it contained a
    /// line terminator.
    pub(super) fn eat_block_comment(&mut self) -> (bool, bool) {
        let mut has_line_terminator = false;

        // stop at the first closing delimiter
        while !self.is_end() {
            let bytes = self.scanner.remaining_bytes();
            let mut index = 0usize;
            while index < bytes.len() {
                match bytes[index] {
                    b'*' | b'\n' | b'\r' | 0xC2 | 0xE2 => break,
                    _ => {
                        index += 1;
                    }
                }
            }

            if index > 0 {
                self.scanner.advance_bytes(index);
            }

            if self.is_end() {
                break;
            }

            let bytes = self.scanner.remaining_bytes();
            if bytes.len() >= 2 && bytes[0] == b'*' && bytes[1] == b'/' {
                self.scanner.advance_ascii_byte();
                self.scanner.advance_ascii_byte();
                return (true, has_line_terminator);
            }

            let is_ascii_line_terminator = matches!(bytes.first(), Some(b'\n' | b'\r'));
            let is_next_line = bytes.starts_with(&[0xC2, 0x85]);
            let is_unicode_separator =
                bytes.starts_with(&[0xE2, 0x80, 0xA8]) || bytes.starts_with(&[0xE2, 0x80, 0xA9]);
            if is_ascii_line_terminator || is_next_line || is_unicode_separator {
                has_line_terminator = true;
            }

            if self.scanner.byte().is_ascii() {
                self.scanner.advance_ascii_byte();
            } else {
                let _ = self.scanner.eat_char();
            }
        }

        (false, has_line_terminator)
    }
}
