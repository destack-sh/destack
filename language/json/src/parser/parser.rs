use tspp_source::{FileId, Span};

use crate::{
    JsonDocument, JsonElement, JsonLexer, JsonProperty, JsonString, JsonToken, JsonTokenKind,
    JsonTrivia, JsonValue,
};

/// Parse result.
pub type ParseResult<T> = Result<T, ParseError>;

/// A parse error.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// The error message.
    pub message: String,
    /// The span where the error occurred.
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// A JSON/JSONC parser.
#[derive(Debug)]
pub struct JsonParser {
    /// The tokens.
    tokens: Vec<JsonToken>,
    /// Current token index.
    position: usize,
    /// The file id for spans.
    file_id: FileId,
}

impl JsonParser {
    /// Create a new parser from source text.
    pub fn new(source: &str, file_id: FileId) -> Self {
        let mut lexer = JsonLexer::new(source, file_id);
        let tokens = lexer.tokenize();
        Self {
            tokens,
            position: 0,
            file_id,
        }
    }

    /// Parse a JSON document.
    pub fn parse(&mut self) -> ParseResult<JsonDocument> {
        let start = self.current_span().start;

        // collect leading trivia
        let trivia_before = self.collect_trivia();

        // parse the root value
        let value = self.parse_value()?;

        // collect trailing trivia
        let trivia_after = self.collect_trivia();

        // expect eof
        if !self.is_at_end() {
            return Err(self.error("expected end of file"));
        }

        let end = self.tokens.last().map(|t| t.span.end).unwrap_or(start);

        Ok(JsonDocument {
            value,
            trivia_before,
            trivia_after,
            span: Span::new(self.file_id, start, end),
        })
    }

    /// Parse a JSON value.
    fn parse_value(&mut self) -> ParseResult<JsonValue> {
        // skip trivia before value
        self.skip_trivia();

        let token = self.current();

        match &token.kind {
            JsonTokenKind::Null => {
                let span = token.span;
                self.advance();
                Ok(JsonValue::Null { span })
            }

            JsonTokenKind::True => {
                let span = token.span;
                self.advance();
                Ok(JsonValue::Bool { value: true, span })
            }

            JsonTokenKind::False => {
                let span = token.span;
                self.advance();
                Ok(JsonValue::Bool { value: false, span })
            }

            JsonTokenKind::Number(raw) => {
                let span = token.span;
                let raw = raw.clone();
                self.advance();
                Ok(JsonValue::Number { raw, span })
            }

            JsonTokenKind::String(value) => {
                let span = token.span;
                let value = value.clone();
                self.advance();
                Ok(JsonValue::String { value, span })
            }

            JsonTokenKind::LeftBracket => self.parse_array(),

            JsonTokenKind::LeftBrace => self.parse_object(),

            JsonTokenKind::Error(msg) => Err(ParseError {
                message: msg.clone(),
                span: token.span,
            }),

            JsonTokenKind::Eof => Err(self.error("unexpected end of file")),

            _ => Err(self.error("expected value")),
        }
    }

    /// Parse a JSON array.
    fn parse_array(&mut self) -> ParseResult<JsonValue> {
        let start = self.current_span().start;

        // consume [
        self.expect(JsonTokenKind::LeftBracket)?;

        let mut elements: Vec<JsonElement> = Vec::new();

        loop {
            // collect trivia before element or ]
            let trivia_before = self.collect_trivia();

            // check for ]
            if self.check(&JsonTokenKind::RightBracket) {
                // attach orphan trivia to last element if present
                if !trivia_before.is_empty()
                    && let Some(last) = elements.last_mut()
                {
                    last.trivia_after.extend(trivia_before);
                }
                break;
            }

            // parse element value
            let value = self.parse_value()?;

            // collect trivia after value
            let trivia_after = self.collect_trivia();

            // check for comma
            let comma = if self.check(&JsonTokenKind::Comma) {
                let span = self.current_span();
                self.advance();
                Some(span)
            } else {
                None
            };

            elements.push(JsonElement {
                value,
                comma,
                trivia_before,
                trivia_after,
            });

            // no comma means end of elements
            if comma.is_none() {
                // collect any trailing trivia before ]
                let trailing = self.collect_trivia();
                if !trailing.is_empty()
                    && let Some(last) = elements.last_mut()
                {
                    last.trivia_after.extend(trailing);
                }
                break;
            }
        }

        // consume ]
        let end_span = self.current_span();
        self.expect(JsonTokenKind::RightBracket)?;

        Ok(JsonValue::Array {
            elements,
            span: Span::new(self.file_id, start, end_span.end),
        })
    }

    /// Parse a JSON object.
    fn parse_object(&mut self) -> ParseResult<JsonValue> {
        let start = self.current_span().start;

        // consume {
        self.expect(JsonTokenKind::LeftBrace)?;

        let mut properties: Vec<JsonProperty> = Vec::new();

        loop {
            // collect trivia before property or }
            let trivia_before = self.collect_trivia();

            // check for }
            if self.check(&JsonTokenKind::RightBrace) {
                // attach orphan trivia to last property
                if !trivia_before.is_empty()
                    && let Some(last) = properties.last_mut()
                {
                    last.trivia_after.extend(trivia_before);
                }
                break;
            }

            // parse key
            let key = self.parse_string_key()?;

            // skip trivia between key and colon
            self.skip_trivia();

            // expect colon
            let colon = self.current_span();
            self.expect(JsonTokenKind::Colon)?;

            // skip trivia between colon and value
            self.skip_trivia();

            // parse value
            let value = self.parse_value()?;

            // collect trivia after value
            let trivia_after = self.collect_trivia();

            // check for comma
            let comma = if self.check(&JsonTokenKind::Comma) {
                let span = self.current_span();
                self.advance();
                Some(span)
            } else {
                None
            };

            properties.push(JsonProperty {
                key,
                colon,
                value,
                comma,
                trivia_before,
                trivia_after,
            });

            // no comma means end of properties
            if comma.is_none() {
                // collect trailing trivia before }
                let trailing = self.collect_trivia();
                if !trailing.is_empty()
                    && let Some(last) = properties.last_mut()
                {
                    last.trivia_after.extend(trailing);
                }
                break;
            }
        }

        // consume }
        let end_span = self.current_span();
        self.expect(JsonTokenKind::RightBrace)?;

        Ok(JsonValue::Object {
            properties,
            span: Span::new(self.file_id, start, end_span.end),
        })
    }

    /// Parse a string key.
    fn parse_string_key(&mut self) -> ParseResult<JsonString> {
        self.skip_trivia();

        let token = self.current();

        match &token.kind {
            JsonTokenKind::String(value) => {
                let span = token.span;
                let value = value.clone();
                self.advance();
                Ok(JsonString { value, span })
            }
            _ => Err(self.error("expected string key")),
        }
    }

    // trivia handling

    /// Collect trivia tokens (whitespace, comments) into a list.
    fn collect_trivia(&mut self) -> Vec<JsonTrivia> {
        let mut trivia = Vec::new();

        while !self.is_at_end() {
            let token = self.current();
            match &token.kind {
                JsonTokenKind::Whitespace => {
                    trivia.push(JsonTrivia::Whitespace { span: token.span });
                    self.advance();
                }
                JsonTokenKind::Newline => {
                    trivia.push(JsonTrivia::Newline { span: token.span });
                    self.advance();
                }
                JsonTokenKind::LineComment(content) => {
                    trivia.push(JsonTrivia::LineComment {
                        content: content.clone(),
                        span: token.span,
                    });
                    self.advance();
                }
                JsonTokenKind::BlockComment(content) => {
                    trivia.push(JsonTrivia::BlockComment {
                        content: content.clone(),
                        span: token.span,
                    });
                    self.advance();
                }
                _ => break,
            }
        }

        trivia
    }

    /// Skip over trivia tokens without collecting them.
    fn skip_trivia(&mut self) {
        while !self.is_at_end() {
            match self.current().kind {
                JsonTokenKind::Whitespace
                | JsonTokenKind::Newline
                | JsonTokenKind::LineComment(_)
                | JsonTokenKind::BlockComment(_) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    // utility helpers

    /// Get the current token.
    fn current(&self) -> &JsonToken {
        &self.tokens[self.position]
    }

    /// Get the span of the current token.
    fn current_span(&self) -> Span {
        self.current().span
    }

    /// Check if we've reached the end of input.
    fn is_at_end(&self) -> bool {
        self.current().kind == JsonTokenKind::Eof
    }

    /// Advance to the next token.
    fn advance(&mut self) {
        if !self.is_at_end() {
            self.position += 1;
        }
    }

    /// Check if the current token matches the given kind.
    fn check(&self, kind: &JsonTokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    /// Expect the current token to match the given kind, advancing if it does.
    fn expect(&mut self, kind: JsonTokenKind) -> ParseResult<()> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("expected {kind:?}")))
        }
    }

    /// Create a parse error at the current position.
    fn error(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            span: self.current_span(),
        }
    }
}

/// Parse JSON/JSONC source text into a document.
pub fn parse(source: &str, file_id: FileId) -> ParseResult<JsonDocument> {
    let mut parser = JsonParser::new(source, file_id);
    parser.parse()
}
