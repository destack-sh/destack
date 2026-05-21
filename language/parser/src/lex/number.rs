use super::lexer::Lexer;
use destack_dir::{NumberBase, TokenLiteral, is_identifier_start};

impl Lexer {
    /// Return whether `.e` or `.E` starts a decimal exponent after a dot.
    #[inline]
    fn dot_starts_decimal_exponent(&self) -> bool {
        let exponent_marker = self.peek_next();
        let exponent_head = self.peek_next_next();
        (exponent_marker == 'e' || exponent_marker == 'E')
            && (exponent_head.is_ascii_digit() || exponent_head == '+' || exponent_head == '-')
    }

    /// Parse a number literal after its first digit.
    ///
    /// Returns the number literal.
    pub(super) fn eat_number_literal(&mut self, first_digit: char) -> TokenLiteral {
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
    pub(super) fn eat_leading_dot_number_literal(&mut self) -> TokenLiteral {
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

    /// Parse decimal digits.
    ///
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

    /// Parse hexadecimal digits.
    ///
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

    /// Parse a float exponent after `e` or `E`.
    ///
    /// Returns whether the exponent is non-empty.
    pub(crate) fn eat_float_exponent(&mut self) -> bool {
        debug_assert!(self.previous() == 'e' || self.previous() == 'E');
        if self.peek() == '-' || self.peek() == '+' {
            self.eat();
        }
        self.eat_decimal_digits()
    }
}
