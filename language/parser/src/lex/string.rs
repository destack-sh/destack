use super::escape::decode_escape;
use super::tokenizer::Tokenizer;

impl Tokenizer {
    /// Parse a quoted string literal after its opening quote.
    pub(super) fn eat_quoted_string(&mut self, quote: char) -> (bool, bool) {
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

            match self.scanner.peek_byte() {
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

    /// Consume a string escape sequence after `\\`.
    /// Return true when the escape sequence is invalid.
    fn eat_string_escape_sequence(&mut self) -> bool {
        let remaining = self.scanner.remaining();
        let mut characters = remaining.chars();
        let decoded = decode_escape(&mut characters);
        let consumed = remaining.len() - characters.as_str().len();
        self.scanner.advance_bytes(consumed);

        decoded.is_err()
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

            match self.scanner.peek_byte() {
                b'`' => {
                    self.scanner.advance_ascii_byte();
                    return true;
                }
                b'$' if self.scanner.peek_byte_at(1) == b'{' => {
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
