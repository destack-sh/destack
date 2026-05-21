use super::lexer::Lexer;

impl Lexer {
    /// Parse a quoted string literal after its opening quote.
    pub(super) fn eat_quoted_string(&mut self, quote: char) -> (bool, bool) {
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
    /// Return true when the escape sequence is invalid.
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
    /// Return true when the sequence is invalid.
    fn eat_unicode_escape_after_u(&mut self) -> bool {
        if self.peek() == '{' {
            self.eat(); // eat `{`
            let mut digits = 0usize;
            let mut value: u32 = 0;
            let mut overflowed = false;
            while self.peek().is_ascii_hexdigit() {
                if !overflowed {
                    // convert ascii hex digit
                    let head = self.peek();
                    let digit = if head.is_ascii_digit() {
                        head as u32 - '0' as u32
                    } else if ('a'..='f').contains(&head) {
                        head as u32 - 'a' as u32 + 10
                    } else {
                        head as u32 - 'A' as u32 + 10
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
    /// Return true when the sequence is invalid.
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
    /// Return whether the template ended before `${`.
    pub(super) fn eat_template_string(&mut self) -> bool {
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
}
