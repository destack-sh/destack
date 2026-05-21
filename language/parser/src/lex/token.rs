use super::lexer::Lexer;
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
                        let (is_terminated, has_line_terminator) = self.eat_block_comment();
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
            c if is_identifier_start(c) => self.eat_identifier_like(c),

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

            // backslash: maybe unicode escape starting an identifier
            '\\' => {
                if let Some(token) = self.try_eat_unicode_escape_identifier() {
                    token
                } else {
                    (TokenType::Unknown, None)
                }
            }

            _ => (TokenType::Unknown, None),
        };

        let token = Token::new(token_type, self.token_len(), literal);
        self.reset_token_start();
        token
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

    /// Parse a whitespace sequence after its first character.
    fn eat_whitespace(&mut self) -> TokenType {
        debug_assert!(is_whitespace(self.previous()));

        // fast path: consume contiguous ascii spaces and tabs in bulk
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
