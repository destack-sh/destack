use super::identifier::classify_keyword_bytes;
use super::scanner::EOF_CHAR;
use super::tokenizer::Tokenizer;
use tspp_dir::{
    Token, TokenLiteral, TokenType, is_identifier_continue, is_identifier_start, is_whitespace,
};
use tspp_unicode::UnicodeEmoji;

impl Tokenizer {
    /// Return true when the character is a line terminator.
    #[inline]
    fn is_line_terminator_char(character: char) -> bool {
        matches!(
            character,
            '\n' | '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}'
        )
    }

    /// Return true when a byte is non-newline ASCII whitespace.
    #[inline]
    fn is_ascii_non_newline_whitespace_byte(byte: u8) -> bool {
        matches!(byte, b' ' | b'\t' | 0x0B | 0x0C)
    }

    /// Parse one token from the input string.
    pub(super) fn read_source_token(&mut self) -> Token {
        self.state.last_trivia_token_has_line_terminator = false;
        let start = self.position() as u32;

        if self.scanner.is_end() {
            return Token::eof(start);
        };

        let first_byte = self.scanner.peek_byte();
        if first_byte.is_ascii() {
            return self.read_ascii_source_token(start, first_byte);
        }

        let first_char = self.scanner.eat_char().unwrap_or(EOF_CHAR);
        let (token_type, literal, is_identifier_escaped) =
            self.read_unicode_source_token(first_char);

        self.finish_identifier_like_source_token(
            start,
            first_byte,
            token_type,
            literal,
            is_identifier_escaped,
        )
    }

    /// Finish one source token from scanner state.
    #[inline(always)]
    fn finish_source_token(
        &mut self,
        start: u32,
        token_type: TokenType,
        literal: Option<TokenLiteral>,
    ) -> Token {
        debug_assert_ne!(token_type, TokenType::Identifier);
        let token = if let Some(literal) = literal {
            Token::new(token_type, start, self.token_len(), Some(literal))
        } else {
            Token::simple(token_type, start, self.token_len())
        };
        self.reset_token_start();

        token
    }

    /// Finish one identifier-like token after reading its complete source text.
    #[inline(always)]
    fn finish_identifier_like_source_token(
        &mut self,
        start: u32,
        first_byte: u8,
        token_type: TokenType,
        literal: Option<TokenLiteral>,
        is_escaped: bool,
    ) -> Token {
        if token_type != TokenType::Identifier {
            return self.finish_source_token(start, token_type, literal);
        }

        let keyword = if first_byte.is_ascii_lowercase() {
            classify_keyword_bytes(self.token_bytes())
        } else {
            None
        };
        let token = Token::identifier(start, self.token_len(), keyword, is_escaped);
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
        self.advance_ascii_bytes(1);

        match first_byte {
            // newline trivia
            b'\n' => self.finish_simple_source_token(start, TokenType::Newline, 1),
            b'\r' => {
                if self.scanner.peek_byte() == b'\n' {
                    self.scanner.advance_ascii_byte();

                    return self.finish_simple_source_token(start, TokenType::Newline, 2);
                }

                self.finish_simple_source_token(start, TokenType::Newline, 1)
            }

            // ordinary trivia
            b' ' | b'\t' | 0x0B | 0x0C => {
                let token_type = self.eat_whitespace(first_byte as char);

                self.finish_source_token(start, token_type, None)
            }

            // identifiers and literals
            b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'$' => {
                let (token_type, literal, is_escaped) = self.eat_ascii_identifier_like(first_byte);

                self.finish_identifier_like_source_token(
                    start, first_byte, token_type, literal, is_escaped,
                )
            }
            b'0'..=b'9' => {
                let literal = self.eat_number_literal(first_byte as char);

                self.finish_source_token(start, TokenType::Literal, Some(literal))
            }

            // punctuation
            b':' => self.finish_simple_source_token(start, TokenType::Colon, 1),
            b';' => self.finish_simple_source_token(start, TokenType::Semicolon, 1),
            b',' => self.finish_simple_source_token(start, TokenType::Comma, 1),
            b'@' => self.finish_simple_source_token(start, TokenType::At, 1),
            b'~' => self.finish_simple_source_token(start, TokenType::ElementwiseNot, 1),
            b'(' => {
                self.state.enter_delimiter();

                self.finish_simple_source_token(start, TokenType::OpenParenthesis, 1)
            }
            b')' => {
                self.state.leave_delimiter();

                self.finish_simple_source_token(start, TokenType::CloseParenthesis, 1)
            }
            b'[' => {
                self.state.enter_delimiter();

                self.finish_simple_source_token(start, TokenType::OpenBracket, 1)
            }
            b']' => {
                self.state.leave_delimiter();

                self.finish_simple_source_token(start, TokenType::CloseBracket, 1)
            }
            b'{' => {
                self.state.enter_delimiter();

                self.finish_simple_source_token(start, TokenType::OpenBrace, 1)
            }
            b'}' => {
                let (token_type, literal) = self.read_close_brace_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'.' => {
                let (token_type, literal) = self.read_dot_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'#' => self.finish_source_token(start, TokenType::Hash, None),
            b'?' => {
                let (token_type, literal) = self.read_question_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'!' => {
                let (token_type, literal) = self.read_bang_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'-' => {
                let (token_type, literal) = self.read_minus_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'&' => {
                let (token_type, literal) = self.read_ampersand_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'|' => {
                let (token_type, literal) = self.read_pipe_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'=' => {
                let (token_type, literal) = self.read_equals_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'<' => {
                let (token_type, literal) = self.read_less_than_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'>' => {
                let (token_type, literal) = self.read_greater_than_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'^' => {
                let (token_type, literal) = self.read_caret_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'+' => {
                let (token_type, literal) = self.read_plus_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'*' => {
                let (token_type, literal) = self.read_star_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'%' => {
                let (token_type, literal) = self.read_percent_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'/' => {
                let (token_type, literal) = self.read_slash_token();

                self.finish_source_token(start, token_type, literal)
            }

            // strings and escape identifiers
            b'\'' => {
                let (token_type, literal) = self.read_single_quote_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'"' => {
                let (token_type, literal) = self.read_double_quote_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'`' => {
                let (token_type, literal) = self.read_template_quote_token();

                self.finish_source_token(start, token_type, literal)
            }
            b'\\' => {
                let (token_type, literal, is_escaped) = self.read_backslash_token();

                self.finish_identifier_like_source_token(
                    start, first_byte, token_type, literal, is_escaped,
                )
            }

            _ => self.finish_simple_source_token(start, TokenType::Unknown, 1),
        }
    }

    /// Parse one non-ASCII token from its first character.
    fn read_unicode_source_token(
        &mut self,
        first_char: char,
    ) -> (TokenType, Option<TokenLiteral>, bool) {
        if is_whitespace(first_char) {
            if Self::is_line_terminator_char(first_char) {
                if first_char == '\r' && self.scanner.peek_byte() == b'\n' {
                    self.scanner.advance_ascii_byte();
                }

                return (TokenType::Newline, None, false);
            }

            return (self.eat_whitespace(first_char), None, false);
        }

        if is_identifier_start(first_char) {
            return self.eat_unicode_identifier_like(first_char);
        }

        if first_char.is_emoji_char() {
            return (self.eat_invalid_identifier(), None, false);
        }

        (TokenType::Unknown, None, false)
    }

    /// Parse one slash token or comment.
    fn read_slash_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        let bytes = self.scanner.remaining_bytes();

        if bytes.first().copied() == Some(b'/') {
            let is_third_slash = bytes.get(1).copied() == Some(b'/');
            let is_fourth_slash = bytes.get(2).copied() == Some(b'/');
            let token_type = if is_third_slash && !is_fourth_slash {
                TokenType::DocLineComment
            } else {
                TokenType::LineComment
            };
            self.eat_until(b'\n');
            self.state.last_trivia_token_has_line_terminator = true;

            return (token_type, None);
        }

        if bytes.first().copied() == Some(b'*') {
            let is_third_star = bytes.get(1).copied() == Some(b'*');
            let is_fourth_star = bytes.get(2).copied() == Some(b'*');
            let is_doc_block = is_third_star && !is_fourth_star;
            self.scanner.advance_ascii_byte();
            let (is_terminated, has_line_terminator) = self.eat_block_comment();
            self.state.last_trivia_token_has_line_terminator = has_line_terminator;

            if !is_terminated {
                return (TokenType::Unknown, None);
            }

            if is_doc_block {
                return (TokenType::DocBlockComment, None);
            }

            return (TokenType::BlockComment, None);
        }

        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::DivideAssign, None);
        }

        (TokenType::Divide, None)
    }

    /// Parse one dot token or leading dot number.
    fn read_dot_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.peek_byte() == b'.' && self.scanner.peek_byte_at(1) == b'.' {
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();

            return (TokenType::Spread, None);
        }

        if self.scanner.peek_byte() == b'.' {
            self.scanner.advance_ascii_byte();

            if self.scanner.peek_byte() == b'=' {
                self.scanner.advance_ascii_byte();

                return (TokenType::RangeInclusive, None);
            }

            return (TokenType::Range, None);
        }

        if self.scanner.peek_byte().is_ascii_digit() {
            let literal = self.eat_leading_dot_number_literal();

            return (TokenType::Literal, Some(literal));
        }

        (TokenType::Dot, None)
    }

    /// Parse one question token.
    fn read_question_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.peek_byte() != b'?' {
            return (TokenType::Maybe, None);
        }

        self.scanner.advance_ascii_byte();
        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::CoalesceAssign, None);
        }

        (TokenType::Coalesce, None)
    }

    /// Parse one bang token.
    fn read_bang_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.peek_byte() != b'=' {
            return (TokenType::Not, None);
        }

        self.scanner.advance_ascii_byte();
        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::NotEqualWide, None);
        }

        (TokenType::NotEqual, None)
    }

    /// Parse one minus token.
    fn read_minus_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        match self.scanner.peek_byte() {
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
        if self.scanner.peek_byte() == b'&' {
            self.scanner.advance_ascii_byte();

            if self.scanner.peek_byte() == b'=' {
                self.scanner.advance_ascii_byte();

                return (TokenType::LogicalAndAssign, None);
            }

            return (TokenType::LogicalAnd, None);
        }

        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::ElementwiseAndAssign, None);
        }

        (TokenType::ElementwiseAnd, None)
    }

    /// Parse one pipe token.
    fn read_pipe_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.peek_byte() == b'|' {
            self.scanner.advance_ascii_byte();

            if self.scanner.peek_byte() == b'=' {
                self.scanner.advance_ascii_byte();

                return (TokenType::LogicalOrAssign, None);
            }

            return (TokenType::LogicalOr, None);
        }

        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::ElementwiseOrAssign, None);
        }

        (TokenType::ElementwiseOr, None)
    }

    /// Parse one close brace token or template continuation.
    fn read_close_brace_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.state.interpolation_depths.is_empty() {
            return (TokenType::CloseBrace, None);
        }

        self.state.delimiter_depth -= 1;

        if self.state.interpolation_depths.last().copied() != Some(self.state.delimiter_depth) {
            return (TokenType::CloseBrace, None);
        }

        self.state.interpolation_depths.pop();
        let is_complete = self.eat_template_string();
        if is_complete {
            return (TokenType::TemplateStringEnd, None);
        }

        // reopen the interpolation expression after `${`
        self.state
            .interpolation_depths
            .push(self.state.delimiter_depth);
        self.state.delimiter_depth += 1;

        (TokenType::TemplateStringMiddle, None)
    }

    /// Parse one equals token.
    fn read_equals_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        match self.scanner.peek_byte() {
            b'>' => {
                self.scanner.advance_ascii_byte();

                (TokenType::ArrowWide, None)
            }
            b'=' => {
                self.scanner.advance_ascii_byte();

                if self.scanner.peek_byte() == b'=' {
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
        match self.scanner.peek_byte() {
            b'<' => {
                self.scanner.advance_ascii_byte();

                if self.scanner.peek_byte() == b'=' {
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
        if self.scanner.peek_byte() == b'>' && self.scanner.peek_byte_at(1) == b'=' {
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();

            return (TokenType::ShiftRightAssign, None);
        }

        if self.scanner.peek_byte() == b'>'
            && self.scanner.peek_byte_at(1) == b'>'
            && self.scanner.peek_byte_at(2) == b'='
        {
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();

            return (TokenType::UnsignedShiftRightAssign, None);
        }

        if self.scanner.peek_byte() == b'>' && self.scanner.peek_byte_at(1) == b'>' {
            self.scanner.advance_ascii_byte();
            self.scanner.advance_ascii_byte();

            return (TokenType::UnsignedShiftRight, None);
        }

        if self.scanner.peek_byte() == b'>' {
            self.scanner.advance_ascii_byte();

            return (TokenType::ShiftRight, None);
        }

        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::GreaterThanOrEqual, None);
        }

        (TokenType::GreaterThan, None)
    }

    /// Parse one caret token.
    fn read_caret_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::ElementwiseXorAssign, None);
        }

        (TokenType::ElementwiseXor, None)
    }

    /// Parse one plus token.
    fn read_plus_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        match self.scanner.peek_byte() {
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
        if self.scanner.peek_byte() == b'*' {
            self.scanner.advance_ascii_byte();

            if self.scanner.peek_byte() == b'=' {
                self.scanner.advance_ascii_byte();

                return (TokenType::ExponentAssign, None);
            }

            return (TokenType::Exponent, None);
        }

        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::MultiplyAssign, None);
        }

        (TokenType::Multiply, None)
    }

    /// Parse one percent token.
    fn read_percent_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        if self.scanner.peek_byte() == b'=' {
            self.scanner.advance_ascii_byte();

            return (TokenType::RemainderAssign, None);
        }

        (TokenType::Remainder, None)
    }

    /// Parse one single-quoted string, character, or lifetime token.
    fn read_single_quote_token(&mut self) -> (TokenType, Option<TokenLiteral>) {
        // scan lifetime
        if let Some(length) = self.peek_lifetime_name() {
            self.scanner.advance_bytes(length);

            return (TokenType::Lifetime, None);
        }

        let (is_terminated, has_invalid_escape) = self.eat_quoted_string('\'');
        let literal = TokenLiteral::String {
            is_terminated,
            has_invalid_escape,
        };

        (TokenType::Literal, Some(literal))
    }

    /// Return the byte length of one tick name after the opening quote.
    fn peek_lifetime_name(&self) -> Option<usize> {
        // scan an identifier immediately after the tick
        let remaining = self.scanner.remaining();
        let mut characters = remaining.char_indices();
        let (_, first) = characters.next()?;
        if !is_identifier_start(first) {
            return None;
        }
        let mut length = first.len_utf8();
        for (index, character) in characters {
            if !is_identifier_continue(character) {
                length = index;
                break;
            }
            length = index + character.len_utf8();
        }

        // a closing quote spells a character literal, not a lifetime
        if remaining.as_bytes().get(length) == Some(&b'\'') {
            return None;
        }

        Some(length)
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
        self.state
            .interpolation_depths
            .push(self.state.delimiter_depth);
        self.state.delimiter_depth += 1;

        (TokenType::TemplateStringStart, None)
    }

    /// Parse one unicode-escape identifier or unknown token.
    fn read_backslash_token(&mut self) -> (TokenType, Option<TokenLiteral>, bool) {
        if let Some(token) = self.try_eat_unicode_escape_identifier() {
            return token;
        }

        (TokenType::Unknown, None, false)
    }

    /// Eat non-newline ascii whitespace bytes.
    #[inline]
    fn eat_ascii_non_newline_whitespace(&mut self) {
        let bytes = self.scanner.remaining_bytes();
        let mut index = 0usize;
        while index < bytes.len() && Self::is_ascii_non_newline_whitespace_byte(bytes[index]) {
            index += 1;
        }

        if index > 0 {
            self.advance_ascii_bytes(index);
        }
    }

    /// Parse a whitespace sequence after its first character.
    fn eat_whitespace(&mut self, first_char: char) -> TokenType {
        debug_assert!(is_whitespace(first_char));

        // consume contiguous ascii spaces and tabs in bulk
        self.eat_ascii_non_newline_whitespace();

        // stop before the overwhelmingly common ASCII successor
        if self.scanner.peek_byte().is_ascii() {
            return TokenType::Whitespace;
        }

        // unicode whitespace tail
        self.eat_while(|character| {
            is_whitespace(character) && !Self::is_line_terminator_char(character)
        });
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

            if self.scanner.peek_byte().is_ascii() {
                self.scanner.advance_ascii_byte();
            } else {
                let _ = self.scanner.eat_char();
            }
        }

        (false, has_line_terminator)
    }
}
