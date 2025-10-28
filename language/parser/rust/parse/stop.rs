use crate::{Parser, ParserError, ParserResult, TokenSpan, TokenType};

impl<'a> Parser<'a> {
    /// Peek an item stop (comma or newline).
    #[inline]
    pub fn peek_item_stop(&self) -> ParserResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline || token.token.ty == TokenType::Comma)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Comma,
            ))
        }
    }

    /// Eat an item stop (comma or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_item_stop_with_newlines(&mut self) -> ParserResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline || token.token.ty == TokenType::Comma)
        {
            self.bump();
        } else {
            return Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Comma,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek a statement stop (semicolon or newline).
    #[inline]
    pub fn peek_statement_stop(&self) -> ParserResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ))
        }
    }

    /// Eat a statement stop (semicolon or newline).
    #[inline]
    pub fn eat_statement_stop(&mut self) -> ParserResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            self.bump(); // eat semicolon or newline
        } else {
            return Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        Ok(())
    }

    /// Eat a statement stop (semicolon or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_statement_stop_with_newlines(&mut self) -> ParserResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            self.bump(); // eat semicolon or newline
        } else {
            return Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek any stop (comma, semicolon, or newline).
    #[inline]
    pub fn peek_any_stop(&self) -> ParserResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ))
        }
    }

    /// Peek next any stop (comma, semicolon, or newline).
    #[inline]
    pub fn peek_next_any_stop(&self) -> ParserResult<&TokenSpan> {
        if let Ok(token) = self.peek_next()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek_next().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ))
        }
    }

    /// Eat any stop (comma, semicolon, or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_any_stop_with_newlines(&mut self) -> ParserResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            self.bump();
        } else {
            return Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek any open parenthesis (`(`, `[`, `{`)
    #[inline]
    pub fn peek_any_open_parenthesis(&self) -> ParserResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::OpenParenthesis
                || token.token.ty == TokenType::OpenBracket
                || token.token.ty == TokenType::OpenBrace)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::OpenParenthesis,
            ))
        }
    }

    /// Peek any close parenthesis (`)`, `]`, `}`)
    #[inline]
    pub fn peek_any_close_parenthesis(&mut self) -> ParserResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::CloseParenthesis
                || token.token.ty == TokenType::CloseBracket
                || token.token.ty == TokenType::CloseBrace)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::CloseParenthesis,
            ))
        }
    }

    /// Peek next any close parenthesis (`)`, `]`, `}`)
    #[inline]
    pub fn peek_next_any_close_parenthesis(&mut self) -> ParserResult<&TokenSpan> {
        if let Ok(token) = self.peek_next()
            && (token.token.ty == TokenType::CloseParenthesis
                || token.token.ty == TokenType::CloseBracket
                || token.token.ty == TokenType::CloseBrace)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(
                self.peek_next().unwrap_or(&self.eof_token).span,
                TokenType::CloseParenthesis,
            ))
        }
    }

    /// Find a token.
    pub fn find_token(&self, target_token: TokenType) -> ParserResult<u32> {
        let mut pos = self.pos() as usize;
        while let Some(token) = self.tokens.get(pos) {
            if token.token.ty == target_token {
                return Ok(pos as u32);
            }
            pos += 1;
        }
        Err(ParserError::unexpected(
            self.peek().unwrap_or(&self.eof_token).span,
        ))
    }

    /// Find an open and a matching close token.
    pub fn find_open_and_matching_close(
        &self,
        open_token: TokenType,
        close_token: TokenType,
    ) -> ParserResult<u32> {
        let open_pos = self.find_token(open_token)?;
        let close_pos = self.find_matching_close(Some(open_pos), open_token, close_token)?;
        Ok(close_pos)
    }

    /// Find closing pair for a pair of tokens.
    pub fn find_matching_close(
        &self,
        pos: Option<u32>,
        open_token: TokenType,
        close_token: TokenType,
    ) -> ParserResult<u32> {
        let mut depth = 0;
        let mut pos = pos.unwrap_or(self.pos()) as usize;

        // we should start at the open token
        #[cfg(debug_assertions)]
        {
            let first_token = self
                .tokens
                .get(pos)
                .map(|token| token.token.ty)
                .unwrap_or(TokenType::End);
            debug_assert_eq!(
                first_token, open_token,
                "expected open token {open_token:?} but got {first_token:?}"
            );
        }

        // seek until we find the matching close token
        while let Some(token) = self.tokens.get(pos) {
            // open: +1
            if token.token.ty == open_token {
                depth += 1;
            }
            // close: -1
            else if token.token.ty == close_token {
                depth -= 1;
            }
            // end: return position
            if depth == 0 {
                return Ok(pos as u32);
            }
            pos += 1;
        }
        Err(ParserError::expected(
            self.peek().unwrap_or(&self.eof_token).span,
            open_token,
        ))
    }

    /// Find a token *within* a matching pair of tokens.
    pub fn find_before_matching_close(
        &self,
        open_token: TokenType,
        close_token: TokenType,
        target_type: TokenType,
    ) -> ParserResult<u32> {
        let mut depth = 0;
        let mut pos = self.pos() as usize;
        while let Some(token) = self.tokens.get(pos) {
            // open: +1
            if token.token.ty == open_token {
                depth += 1;
            }
            // close: -1
            else if token.token.ty == close_token {
                depth -= 1;
            }
            // token at depth 1 (immediately inside the matching pair)
            else if depth == 1 && token.token.ty == target_type {
                return Ok(pos as u32);
            }
            // end: return position
            if depth == 0 {
                break;
            }
            pos += 1;
        }
        Err(ParserError::expected(
            self.peek().unwrap_or(&self.eof_token).span,
            open_token,
        ))
    }

    /// Skip any newlines at and after a position.
    #[inline]
    pub fn skip_newlines_after(&mut self, pos: u32) -> ParserResult<u32> {
        let mut pos = pos as usize;
        while let Some(token) = self.tokens.get(pos + 1)
            && token.token.ty == TokenType::Newline
        {
            pos += 1;
        }
        Ok(pos as u32)
    }
}
