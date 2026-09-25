use super::tokenizer::Tokenizer;
use tspp_dir::{NumberBase, TokenLiteral, is_identifier_start};

impl Tokenizer {
    /// Return whether a byte can start an ASCII identifier.
    #[inline]
    fn is_ascii_identifier_start_byte(byte: u8) -> bool {
        byte.is_ascii() && is_identifier_start(byte as char)
    }

    /// Return whether `.e` or `.E` starts a decimal exponent after a dot.
    #[inline]
    fn peek_dot_decimal_exponent(&self) -> bool {
        let exponent_marker = self.scanner.peek_byte_at(1);
        let exponent_head = self.scanner.peek_byte_at(2);
        matches!(exponent_marker, b'e' | b'E')
            && (exponent_head.is_ascii_digit() || matches!(exponent_head, b'+' | b'-'))
    }

    /// Parse a number literal after its first digit.
    ///
    /// Returns the number literal.
    pub(super) fn eat_number_literal(&mut self, first_digit: char) -> TokenLiteral {
        debug_assert!(first_digit.is_ascii_digit());
        let mut base = NumberBase::Decimal;
        if first_digit == '0' {
            // parse encoding base
            match self.scanner.peek_byte() {
                // binary literal
                b'b' | b'B' => {
                    base = NumberBase::Binary;
                    self.scanner.advance_ascii_byte();
                    if !self.eat_decimal_digits() {
                        return TokenLiteral::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // octal literal
                b'o' | b'O' => {
                    base = NumberBase::Octal;
                    self.scanner.advance_ascii_byte();
                    if !self.eat_decimal_digits() {
                        return TokenLiteral::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // hexadecimal literal
                b'x' | b'X' => {
                    base = NumberBase::Hexadecimal;
                    self.scanner.advance_ascii_byte();
                    if !self.eat_hexadecimal_digits() {
                        return TokenLiteral::Int {
                            base,
                            is_empty: true,
                            is_bigint: false,
                        };
                    }
                }

                // not a base prefix; consume additional digits
                b'0'..=b'9' | b'_' => {
                    self.eat_decimal_digits();
                }

                // also not a base prefix; nothing more to do here
                b'.' | b'e' | b'E' | b'n' => {}

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

        match self.scanner.peek_byte() {
            // don't be greedy if this is actually an
            // integer literal followed by field or method access
            // (`12.foo()` and `12..toString()`)
            b'.' if self.scanner.peek_byte_at(1) != b'.'
                && (!Self::is_ascii_identifier_start_byte(self.scanner.peek_byte_at(1))
                    || self.peek_dot_decimal_exponent()) =>
            {
                // might have stuff after the ., and if it does, it starts with a number
                self.scanner.advance_ascii_byte();
                let mut is_empty_exponent = false;

                if self.scanner.peek_byte().is_ascii_digit() {
                    self.eat_decimal_digits();
                }

                // allow exponent forms without a fractional part (`1.e1`)
                if matches!(self.scanner.peek_byte(), b'e' | b'E') {
                    self.scanner.advance_ascii_byte();
                    is_empty_exponent = !self.eat_float_exponent();
                }

                TokenLiteral::Float {
                    base,
                    is_empty_exponent,
                }
            }
            b'e' | b'E' => {
                self.scanner.advance_ascii_byte();
                let is_empty_exponent = !self.eat_float_exponent();
                TokenLiteral::Float {
                    base,
                    is_empty_exponent,
                }
            }
            b'n' => {
                self.scanner.advance_ascii_byte();
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
    pub(super) fn eat_leading_dot_number_literal(&mut self) -> TokenLiteral {
        let base = NumberBase::Decimal;
        let is_empty_exponent = {
            self.eat_decimal_digits();
            if matches!(self.scanner.peek_byte(), b'e' | b'E') {
                self.scanner.advance_ascii_byte();
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

    /// Parse decimal digits.
    ///
    /// Returns whether any digits were parsed.
    pub(crate) fn eat_decimal_digits(&mut self) -> bool {
        let bytes = self.scanner.remaining_bytes();
        let mut index = 0usize;
        let mut has_digits = false;

        while index < bytes.len() {
            match bytes[index] {
                b'_' => {}
                b'0'..=b'9' => {
                    has_digits = true;
                }
                _ => break,
            }
            index += 1;
        }

        if index > 0 {
            self.advance_ascii_bytes(index);
        }

        has_digits
    }

    /// Parse hexadecimal digits.
    ///
    /// Returns whether any digits were parsed.
    pub(crate) fn eat_hexadecimal_digits(&mut self) -> bool {
        let bytes = self.scanner.remaining_bytes();
        let mut index = 0usize;
        let mut has_digits = false;

        while index < bytes.len() {
            match bytes[index] {
                b'_' => {}
                b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => {
                    has_digits = true;
                }
                _ => break,
            }
            index += 1;
        }

        if index > 0 {
            self.advance_ascii_bytes(index);
        }

        has_digits
    }

    /// Parse a float exponent after `e` or `E`.
    ///
    /// Returns whether the exponent is non-empty.
    pub(crate) fn eat_float_exponent(&mut self) -> bool {
        if matches!(self.scanner.peek_byte(), b'-' | b'+') {
            self.scanner.advance_ascii_byte();
        }
        self.eat_decimal_digits()
    }
}
