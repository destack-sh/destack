use super::lexer::Lexer;

impl Lexer {
    /// Parse a quoted string literal after its opening quote.
    pub(super) fn eat_quoted_string(&mut self, quote: char) -> (bool, bool) {
        debug_assert!(self.previous() == quote);
        debug_assert!(quote.is_ascii());

        let quote_byte = quote as u8;
        let mut has_invalid_escape = false;

        // parse until either quotes are terminated or EOF is reached
        while !self.is_end() {
            let bytes = self.scanner.remaining_bytes();
            let mut index = 0usize;
            while index < bytes.len() {
                match bytes[index] {
                    byte if byte == quote_byte
                        || byte == b'\\'
                        || byte == b'\n'
                        || byte == b'\r' =>
                    {
                        break;
                    }
                    _ => {
                        index += 1;
                    }
                }
            }

            if index > 0 {
                self.scanner.advance_bytes(index);
            }

            match self.scanner.byte() {
                // quotes are terminated, finish parsing
                byte if byte == quote_byte => {
                    self.scanner.advance_ascii_byte();
                    return (true, has_invalid_escape);
                }

                // line terminators are not allowed in quoted strings
                b'\n' | b'\r' => {
                    return (false, has_invalid_escape);
                }

                // escape sequence
                b'\\' => {
                    self.scanner.advance_ascii_byte();

                    // consume escape sequence and track invalid escapes
                    if self.eat_string_escape_sequence() {
                        has_invalid_escape = true;
                    }
                }

                // regular character
                _ => {
                    break;
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

        let escaped = self.scanner.byte();
        match escaped {
            // legacy escaped digits are invalid
            b'8' | b'9' => {
                self.scanner.advance_ascii_byte();
                true
            }
            // legacy octal escapes are invalid without a following digit
            b'0'..=b'7' => {
                self.scanner.advance_ascii_byte();
                if escaped != b'0' {
                    return true;
                }
                if self.scanner.byte().is_ascii_digit() {
                    return true;
                }
                false
            }
            // \uXXXX and \u{...}
            b'u' => {
                self.scanner.advance_ascii_byte();
                self.eat_unicode_escape_after_u()
            }
            // \xXX
            b'x' => {
                self.scanner.advance_ascii_byte();
                self.eat_fixed_hex_escape(2)
            }
            // regular escaped character
            _ => {
                if escaped.is_ascii() {
                    self.scanner.advance_ascii_byte();
                } else {
                    let _ = self.scanner.eat_char();
                }
                false
            }
        }
    }

    /// Consume a unicode escape sequence body after `\u`.
    /// Return true when the sequence is invalid.
    fn eat_unicode_escape_after_u(&mut self) -> bool {
        if self.scanner.byte() == b'{' {
            self.scanner.advance_ascii_byte();
            let mut digits = 0usize;
            let mut value: u32 = 0;
            let mut overflowed = false;
            while self.scanner.byte().is_ascii_hexdigit() {
                if !overflowed {
                    // convert ascii hex digit
                    let head = self.scanner.byte();
                    let digit = if head.is_ascii_digit() {
                        u32::from(head - b'0')
                    } else if (b'a'..=b'f').contains(&head) {
                        u32::from(head - b'a') + 10
                    } else {
                        u32::from(head - b'A') + 10
                    };

                    if let Some(next) = value.checked_mul(16).and_then(|v| v.checked_add(digit)) {
                        value = next;
                    } else {
                        overflowed = true;
                    }
                }
                self.scanner.advance_ascii_byte();
                digits += 1;
            }

            if digits == 0 || self.scanner.byte() != b'}' {
                return true;
            }
            self.scanner.advance_ascii_byte();
            overflowed || value > 0x10FFFF
        } else {
            self.eat_fixed_hex_escape(4)
        }
    }

    /// Consume an exact number of hexadecimal digits.
    /// Return true when the sequence is invalid.
    fn eat_fixed_hex_escape(&mut self, width: usize) -> bool {
        for _ in 0..width {
            if !self.scanner.byte().is_ascii_hexdigit() {
                return true;
            }
            self.scanner.advance_ascii_byte();
        }
        false
    }

    /// Parse a template string (excluding first backtick).
    /// Return whether the template ended before `${`.
    pub(super) fn eat_template_string(&mut self) -> bool {
        while !self.is_end() {
            let bytes = self.scanner.remaining_bytes();
            let mut index = 0usize;
            while index < bytes.len() {
                match bytes[index] {
                    b'`' | b'$' | b'\\' => break,
                    _ => {
                        index += 1;
                    }
                }
            }

            if index > 0 {
                self.scanner.advance_bytes(index);
            }

            match self.scanner.byte() {
                b'`' => {
                    self.scanner.advance_ascii_byte();
                    return true;
                }
                b'$' if self.scanner.byte_at(1) == b'{' => {
                    self.scanner.advance_ascii_byte();
                    self.scanner.advance_ascii_byte();
                    return false;
                }
                b'\\' => {
                    self.scanner.advance_ascii_byte();
                    let escaped = self.scanner.peek_char();

                    // skip the escaped code unit so `\${` stays literal text
                    if escaped != '\0' {
                        let _ = self.scanner.eat_char();
                    }
                }
                _ => {
                    let _ = self.scanner.eat_char();
                }
            }
        }

        false
    }
}
