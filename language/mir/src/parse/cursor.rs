use tspp_source::Span;

use crate::source::{Token, TokenType};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

impl Parser {
    /// Get current position for error reporting.
    pub(super) fn pos(&self) -> usize {
        if let Some(token) = self.peek() {
            token.start()
        } else if let Some(end) = self.last_consumed_token_end() {
            end
        } else {
            let Some(source) = self.tree.source_text.as_ref() else {
                unreachable!("MIR parser tree has no source text");
            };

            source.len()
        }
    }

    /// Return the MIR token type for one source token.
    pub(super) fn token_type(&self, token: &Token) -> TokenType {
        let text = self.tree.source_text(token.span);

        match token.ty {
            TokenType::Identifier => TokenType::from_identifier(text),
            ty => ty,
        }
    }

    /// Build a span for a source slice.
    pub(super) fn span_at(&self, start: usize, length: usize) -> Span {
        Span::at(self.file_id, start as u32, length as u32)
    }

    /// Build a span for one parsed range.
    pub(super) fn span_between(&self, start: usize, end: usize) -> Span {
        assert!(end >= start, "MIR source span ends before it starts");

        self.span_at(start, end - start)
    }

    /// Build a span from one parse start to the last consumed token.
    pub(super) fn span_from_parse_start(&self, start: usize) -> Span {
        let end = self.last_consumed_token_end().unwrap_or(start);

        self.span_between(start, end)
    }

    /// Return whether source between one token and the next token crosses a line.
    pub(super) fn has_line_break_after(&self, token: &Token) -> bool {
        let Some(next) = self.peek() else {
            return false;
        };

        let start = token.span.end;
        let end = next.span.start;
        if end <= start {
            return false;
        }

        let span = Span::at(self.file_id, start, end - start);

        self.tree
            .source_text(span)
            .bytes()
            .any(|byte| matches!(byte, b'\n' | b'\r'))
    }

    /// Peek the current token, skipping trivia.
    pub(super) fn peek(&self) -> Option<&Token> {
        let mut pos = self.pos;
        let tokens = self.tree.tokens();

        while pos < tokens.len() {
            let token = &tokens[pos];
            if !token.is_trivia() {
                return Some(token);
            }
            pos += 1;
        }

        None
    }

    /// Peek the nth non-trivia token, where 0 is the current token.
    pub(super) fn peek_nth_token(&self, n: usize) -> Option<&Token> {
        let mut pos = self.pos;
        let mut seen = 0usize;
        let tokens = self.tree.tokens();

        while pos < tokens.len() {
            let token = &tokens[pos];
            if !token.is_trivia() {
                if seen == n {
                    return Some(token);
                }
                seen += 1;
            }
            pos += 1;
        }

        None
    }

    /// Advance past the current token.
    pub(super) fn bump(&mut self) {
        let tokens = self.tree.tokens();

        while self.pos < tokens.len() {
            let is_trivia = tokens[self.pos].is_trivia();
            self.pos += 1;
            if !is_trivia {
                break;
            }
        }

        // skip trailing trivia
        while self.pos < tokens.len() && tokens[self.pos].is_trivia() {
            self.pos += 1;
        }
    }

    /// Return whether the current source token matches one token type.
    pub(super) fn peek_is(&self, ty: TokenType) -> bool {
        self.peek()
            .is_some_and(|token| self.token_type(token) == ty)
    }

    /// Consume one source token with the expected token type.
    pub(super) fn eat_token(&mut self, ty: TokenType) -> ParseResult<Token> {
        let token = *self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end(&format!("{ty:?}"), self.pos()))?;
        if self.token_type(&token) != ty {
            return Err(ParseError::unexpected_token(&format!("{ty:?}"), &token));
        }

        self.bump();

        Ok(token)
    }

    /// Consume one source token when it matches one token type.
    pub(super) fn eat_token_if(&mut self, ty: TokenType) -> bool {
        if self.peek_is(ty) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consume the current identifier when it matches the expected text.
    pub(super) fn eat_name_if(&mut self, expected: &str) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };
        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        let text = self.tree.source_text(token.span);
        if text != expected {
            return false;
        }

        self.bump();

        true
    }

    /// Parse an instruction opcode.
    ///
    /// Opcodes can be identifiers or reserved opcode keywords that also have dedicated token kinds.
    pub(super) fn parse_opcode(&mut self) -> ParseResult<(String, usize)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("opcode", self.pos()))?;

        match self.token_type(token) {
            TokenType::Identifier
            | TokenType::Const
            | TokenType::Ownership
            | TokenType::Struct
            | TokenType::Call
            | TokenType::CallIndirect
            | TokenType::CallVirtual
            | TokenType::CallDynamic => {
                let text = self.tree.source_text(token.span).to_string();
                let start = token.start();
                self.bump();

                Ok((text, start))
            }
            _ => Err(ParseError::unexpected_token("opcode", token)),
        }
    }

    /// Return the exclusive end of the last consumed non-trivia token.
    pub(super) fn last_consumed_token_end(&self) -> Option<usize> {
        let mut index = self.pos;
        let tokens = self.tree.tokens();

        while index > 0 {
            index -= 1;
            let token = &tokens[index];
            if !token.is_trivia() {
                return Some(token.span.end as usize);
            }
        }

        None
    }

    /// Skip raw trivia tokens except newline.
    pub(super) fn skip_raw_trivia_except_newline(&mut self) {
        let tokens = self.tree.tokens();

        while self.pos < tokens.len() {
            let token = &tokens[self.pos];
            let ty = self.token_type(token);
            if !token.is_trivia() || ty == TokenType::Newline {
                break;
            }

            self.pos += 1;
        }
    }

    /// Return whether one token starts at the first byte of its source line.
    pub(super) fn is_token_at_line_start(&self, token: &Token) -> bool {
        let Some(source_text) = self.tree.source_text.as_deref() else {
            unreachable!("MIR tree has no parsed source text");
        };
        let start = token.span.start as usize;

        start == 0 || source_text.as_bytes().get(start - 1) == Some(&b'\n')
    }
}
