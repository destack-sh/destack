use tspp_source::{FileId, Span};

/// A token in JSON/JSONC source.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonToken {
    /// The token type.
    pub kind: JsonTokenKind,
    /// The span of the token.
    pub span: Span,
}

/// The type of a JSON token.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonTokenKind {
    // literals
    /// The `null` keyword.
    Null,
    /// The `true` keyword.
    True,
    /// The `false` keyword.
    False,
    /// A numeric literal.
    Number(String),
    /// A string literal (unescaped content).
    String(String),

    // punctuation
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `[`
    LeftBracket,
    /// `]`
    RightBracket,
    /// `:`
    Colon,
    /// `,`
    Comma,

    // trivia
    /// Whitespace (spaces, tabs).
    Whitespace,
    /// A newline character.
    Newline,
    /// A line comment (`// ...`).
    LineComment(String),
    /// A block comment (`/* ... */`).
    BlockComment(String),

    // special
    /// End of file.
    Eof,
    /// Invalid/unexpected character.
    Error(String),
}

/// A JSON/JSONC lexer.
#[derive(Debug)]
pub struct JsonLexer<'a> {
    /// The source text.
    source: &'a str,
    /// The file id for spans.
    file_id: FileId,
    /// Current byte position.
    position: u32,
}

impl<'a> JsonLexer<'a> {
    /// Create a new lexer.
    pub fn new(source: &'a str, file_id: FileId) -> Self {
        Self {
            source,
            file_id,
            position: 0,
        }
    }

    /// Tokenize the entire source into a list of tokens.
    pub fn tokenize(&mut self) -> Vec<JsonToken> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            let is_eof = token.kind == JsonTokenKind::Eof;
            tokens.push(token);

            if is_eof {
                break;
            }
        }

        tokens
    }

    /// Get the next token.
    pub fn next_token(&mut self) -> JsonToken {
        let start = self.position;

        // check for eof
        let Some(ch) = self.peek() else {
            return self.make_token(start, JsonTokenKind::Eof);
        };

        match ch {
            // whitespace
            ' ' | '\t' | '\r' => {
                self.advance();
                while matches!(self.peek(), Some(' ' | '\t' | '\r')) {
                    self.advance();
                }
                self.make_token(start, JsonTokenKind::Whitespace)
            }

            // newline
            '\n' => {
                self.advance();
                self.make_token(start, JsonTokenKind::Newline)
            }

            // punctuation
            '{' => {
                self.advance();
                self.make_token(start, JsonTokenKind::LeftBrace)
            }
            '}' => {
                self.advance();
                self.make_token(start, JsonTokenKind::RightBrace)
            }
            '[' => {
                self.advance();
                self.make_token(start, JsonTokenKind::LeftBracket)
            }
            ']' => {
                self.advance();
                self.make_token(start, JsonTokenKind::RightBracket)
            }
            ':' => {
                self.advance();
                self.make_token(start, JsonTokenKind::Colon)
            }
            ',' => {
                self.advance();
                self.make_token(start, JsonTokenKind::Comma)
            }

            // string
            '"' => self.scan_string(start),

            // comment or error
            '/' => self.scan_comment_or_error(start),

            // number
            '-' | '0'..='9' => self.scan_number(start),

            // keywords (null, true, false)
            'n' => self.scan_keyword(start, "null", JsonTokenKind::Null),
            't' => self.scan_keyword(start, "true", JsonTokenKind::True),
            'f' => self.scan_keyword(start, "false", JsonTokenKind::False),

            // error
            _ => {
                self.advance();
                self.make_token(
                    start,
                    JsonTokenKind::Error(format!("unexpected character: {ch}")),
                )
            }
        }
    }

    // scanning helpers

    /// Scan a string literal starting at the given position.
    fn scan_string(&mut self, start: u32) -> JsonToken {
        // consume opening quote
        self.advance();
        let mut value = String::new();

        loop {
            match self.peek() {
                None => {
                    return self
                        .make_token(start, JsonTokenKind::Error("unterminated string".into()));
                }
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('"') => {
                            value.push('"');
                            self.advance();
                        }
                        Some('\\') => {
                            value.push('\\');
                            self.advance();
                        }
                        Some('/') => {
                            value.push('/');
                            self.advance();
                        }
                        Some('b') => {
                            value.push('\x08');
                            self.advance();
                        }
                        Some('f') => {
                            value.push('\x0c');
                            self.advance();
                        }
                        Some('n') => {
                            value.push('\n');
                            self.advance();
                        }
                        Some('r') => {
                            value.push('\r');
                            self.advance();
                        }
                        Some('t') => {
                            value.push('\t');
                            self.advance();
                        }
                        Some('u') => {
                            self.advance();
                            match self.scan_unicode_escape() {
                                Ok(c) => value.push(c),
                                Err(e) => {
                                    return self.make_token(start, JsonTokenKind::Error(e));
                                }
                            }
                        }
                        Some(c) => {
                            return self.make_token(
                                start,
                                JsonTokenKind::Error(format!("invalid escape sequence: \\{c}")),
                            );
                        }
                        None => {
                            return self.make_token(
                                start,
                                JsonTokenKind::Error("unterminated string".into()),
                            );
                        }
                    }
                }
                Some('\n') => {
                    return self.make_token(
                        start,
                        JsonTokenKind::Error("newline in string literal".into()),
                    );
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
            }
        }

        self.make_token(start, JsonTokenKind::String(value))
    }

    /// Scan a unicode escape sequence (\uXXXX).
    fn scan_unicode_escape(&mut self) -> Result<char, String> {
        let mut hex = String::with_capacity(4);

        for _ in 0..4 {
            match self.peek() {
                Some(c) if c.is_ascii_hexdigit() => {
                    hex.push(c);
                    self.advance();
                }
                _ => return Err("invalid unicode escape sequence".into()),
            }
        }

        let code_point =
            u32::from_str_radix(&hex, 16).map_err(|_| "invalid unicode escape sequence")?;

        char::from_u32(code_point).ok_or_else(|| "invalid unicode code point".into())
    }

    /// Scan a comment (line or block) or return an error for bare slash.
    fn scan_comment_or_error(&mut self, start: u32) -> JsonToken {
        self.advance(); // consume first /

        match self.peek() {
            // line comment
            Some('/') => {
                self.advance();
                let content_start = self.position as usize;

                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }

                let content = self.source[content_start..self.position as usize].to_string();
                self.make_token(start, JsonTokenKind::LineComment(content))
            }

            // block comment
            Some('*') => {
                self.advance();
                let content_start = self.position as usize;

                loop {
                    match self.peek() {
                        None => {
                            return self.make_token(
                                start,
                                JsonTokenKind::Error("unterminated block comment".into()),
                            );
                        }
                        Some('*') => {
                            self.advance();
                            if self.peek() == Some('/') {
                                let content = self.source
                                    [content_start..(self.position as usize - 1)]
                                    .to_string();
                                self.advance();
                                return self
                                    .make_token(start, JsonTokenKind::BlockComment(content));
                            }
                        }
                        Some(_) => {
                            self.advance();
                        }
                    }
                }
            }

            // not a comment, just a slash (error in JSON)
            _ => self.make_token(start, JsonTokenKind::Error("unexpected '/'".into())),
        }
    }

    /// Scan a number literal (integer, decimal, or exponential).
    fn scan_number(&mut self, start: u32) -> JsonToken {
        // optional minus
        if self.peek() == Some('-') {
            self.advance();
        }

        // integer part
        match self.peek() {
            Some('0') => {
                self.advance();
            }
            Some('1'..='9') => {
                self.advance();
                while matches!(self.peek(), Some('0'..='9')) {
                    self.advance();
                }
            }
            _ => {
                return self.make_token(start, JsonTokenKind::Error("invalid number".into()));
            }
        }

        // fractional part
        if self.peek() == Some('.') {
            self.advance();
            if !matches!(self.peek(), Some('0'..='9')) {
                return self.make_token(start, JsonTokenKind::Error("invalid number".into()));
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.advance();
            }
        }

        // exponent part
        if matches!(self.peek(), Some('e' | 'E')) {
            self.advance();
            if matches!(self.peek(), Some('+' | '-')) {
                self.advance();
            }
            if !matches!(self.peek(), Some('0'..='9')) {
                return self.make_token(start, JsonTokenKind::Error("invalid number".into()));
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.advance();
            }
        }

        let raw = self.source[start as usize..self.position as usize].to_string();
        self.make_token(start, JsonTokenKind::Number(raw))
    }

    /// Scan a keyword (null, true, false) or return an error.
    fn scan_keyword(&mut self, start: u32, expected: &str, kind: JsonTokenKind) -> JsonToken {
        let end = start as usize + expected.len();

        if end <= self.source.len() && &self.source[start as usize..end] == expected {
            self.position = end as u32;
            self.make_token(start, kind)
        } else {
            // scan as identifier and report error
            while let Some(c) = self.peek() {
                if !c.is_ascii_alphanumeric() {
                    break;
                }
                self.advance();
            }
            let text = &self.source[start as usize..self.position as usize];
            self.make_token(
                start,
                JsonTokenKind::Error(format!("unexpected identifier: {text}")),
            )
        }
    }

    // utility helpers

    /// Peek at the current character without consuming it.
    fn peek(&self) -> Option<char> {
        self.source[self.position as usize..].chars().next()
    }

    /// Advance to the next character.
    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.position += c.len_utf8() as u32;
        }
    }

    /// Create a token with the given kind spanning from start to current position.
    fn make_token(&self, start: u32, kind: JsonTokenKind) -> JsonToken {
        JsonToken {
            kind,
            span: Span::new(self.file_id, start, self.position),
        }
    }
}
