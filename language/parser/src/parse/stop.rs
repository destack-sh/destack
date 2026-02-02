use crate::{ParseError, ParseResult, Parser};
use destack_ast::{TokenSpan, TokenType};

impl Parser {
    /// Return true when the next token is a statement stop.
    #[inline]
    pub fn is_statement_stop(&self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::Newline | TokenType::Semicolon | TokenType::End
        )
    }

    /// Return true when the next token is an item stop.
    #[inline]
    pub fn is_item_stop(&self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::Comma | TokenType::Newline | TokenType::End
        )
    }

    /// Return true when the next token is any stop.
    #[inline]
    pub fn is_any_stop(&self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::Comma | TokenType::Semicolon | TokenType::Newline | TokenType::End
        )
    }

    /// Peek an item stop (comma or newline).
    #[inline]
    pub fn peek_item_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Comma || token.token.ty == TokenType::Newline)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Comma,
            ))
        }
    }

    /// Eat a comma with newlines.
    /// Eats all following newlines.
    #[inline]
    pub fn eat_item_stop_with_newlines(&mut self) -> ParseResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Comma)
        {
            self.bump();
        } else {
            return Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Comma,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek a statement stop (semicolon or newline).
    #[inline]
    pub fn peek_statement_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ))
        }
    }

    /// Eat a statement stop (semicolon or newline).
    #[inline]
    pub fn eat_statement_stop(&mut self) -> ParseResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            self.bump(); // eat semicolon or newline
        } else {
            return Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        Ok(())
    }

    /// Eat a statement stop (semicolon or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_statement_stop_with_newlines(&mut self) -> ParseResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            self.bump(); // eat semicolon or newline
        } else {
            return Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek any stop (comma, semicolon, or newline).
    #[inline]
    pub fn peek_any_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ))
        }
    }

    /// Peek next any stop (comma, semicolon, or newline).
    #[inline]
    pub fn peek_next_any_stop(&self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek_next()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(
                self.peek_next().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ))
        }
    }

    /// Eat any stop (comma, semicolon, or newline).
    /// Eats all following newlines.
    #[inline]
    pub fn eat_any_stop_with_newlines(&mut self) -> ParseResult<()> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            self.bump();
        } else {
            return Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::Newline,
            ));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek any open parenthesis (`(`, `[`, `{`)
    #[inline]
    pub fn peek_any_open_parenthesis(&self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::OpenParenthesis
                || token.token.ty == TokenType::OpenBracket
                || token.token.ty == TokenType::OpenBrace)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::OpenParenthesis,
            ))
        }
    }

    /// Peek any close parenthesis (`)`, `]`, `}`)
    #[inline]
    pub fn peek_any_close_parenthesis(&mut self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::CloseParenthesis
                || token.token.ty == TokenType::CloseBracket
                || token.token.ty == TokenType::CloseBrace)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(
                self.peek().unwrap_or(&self.eof_token).span,
                TokenType::CloseParenthesis,
            ))
        }
    }

    /// Peek next any close parenthesis (`)`, `]`, `}`)
    #[inline]
    pub fn peek_next_any_close_parenthesis(&mut self) -> ParseResult<&TokenSpan> {
        if let Ok(token) = self.peek_next()
            && (token.token.ty == TokenType::CloseParenthesis
                || token.token.ty == TokenType::CloseBracket
                || token.token.ty == TokenType::CloseBrace)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(
                self.peek_next().unwrap_or(&self.eof_token).span,
                TokenType::CloseParenthesis,
            ))
        }
    }

    /// Find a token.
    #[inline]
    pub fn find_token(&self, target_token: TokenType) -> ParseResult<u32> {
        let mut pos = self.pos() as usize;
        while let Some(token) = self.tokens.get(pos) {
            if token.token.ty == target_token {
                return Ok(pos as u32);
            }
            pos += 1;
        }
        Err(ParseError::unexpected(
            self.peek().unwrap_or(&self.eof_token).span,
        ))
    }

    /// Find a token after a position.
    #[inline]
    pub fn find_token_after(&self, pos: u32, target_token: TokenType) -> ParseResult<u32> {
        let mut pos = pos as usize;
        while let Some(token) = self.tokens.get(pos) {
            if token.token.ty == target_token {
                return Ok(pos as u32);
            }
            pos += 1;
        }
        Err(ParseError::unexpected(
            self.peek().unwrap_or(&self.eof_token).span,
        ))
    }

    /// Find an open and a matching close token.
    pub fn find_open_and_matching_close(
        &self,
        open_token: TokenType,
        close_token: TokenType,
    ) -> ParseResult<u32> {
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
    ) -> ParseResult<u32> {
        let mut depth = 0;
        let mut pos = pos.unwrap_or(self.pos()) as usize;

        // if there's a split token that matches the open token, start with depth = 1
        // (the split token counts as the opening bracket)
        let has_split_open = self.has_split_token(open_token);
        if has_split_open {
            depth = 1;
        }

        // we should start at the open token (unless we have a split token)
        #[cfg(debug_assertions)]
        if !has_split_open {
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

        // fast path for precomputed pairs
        if !has_split_open {
            let expected_close = match open_token {
                TokenType::OpenParenthesis => Some(TokenType::CloseParenthesis),
                TokenType::OpenBrace => Some(TokenType::CloseBrace),
                TokenType::OpenBracket => Some(TokenType::CloseBracket),
                _ => None,
            };
            if expected_close == Some(close_token)
                && self
                    .tokens
                    .get(pos)
                    .is_some_and(|token| token.token.ty == open_token)
            {
                let matching = self.matching_pairs.get(pos).copied().unwrap_or(u32::MAX);
                if matching != u32::MAX {
                    return Ok(matching);
                }
            }
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

        Err(ParseError::expected(
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
    ) -> ParseResult<u32> {
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
        Err(ParseError::expected(
            self.peek().unwrap_or(&self.eof_token).span,
            open_token,
        ))
    }

    /// Check whether a parenthesized expression has a top level token.
    pub fn has_token_before_matching_close(
        &self,
        open_pos: u32,
        close_pos: u32,
        target_token: TokenType,
        track_angle: bool,
    ) -> ParseResult<bool> {
        // depth tracking
        let mut paren_depth = 0u32;
        let mut brace_depth = 0u32;
        let mut bracket_depth = 0u32;
        let mut angle_depth = 0u32;
        let mut pos = open_pos as usize;
        let close_pos = close_pos as usize;

        // scan the parenthesis contents
        while pos <= close_pos {
            let ty = self
                .tokens
                .get(pos)
                .map(|token| token.token.ty)
                .unwrap_or(TokenType::End);

            match ty {
                TokenType::OpenParenthesis => paren_depth += 1,
                TokenType::CloseParenthesis => {
                    paren_depth = paren_depth.saturating_sub(1);
                    if paren_depth == 0 {
                        return Ok(false);
                    }
                }
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenType::LessThan if track_angle => angle_depth += 1,
                TokenType::GreaterThan if track_angle => {
                    angle_depth = angle_depth.saturating_sub(1);
                }
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft if track_angle => {
                    angle_depth += 2;
                }
                _ if ty == target_token
                    && paren_depth == 1
                    && brace_depth == 0
                    && bracket_depth == 0
                    && angle_depth == 0 =>
                {
                    return Ok(true);
                }
                _ => {}
            }

            pos += 1;
        }

        Ok(false)
    }

    /// Skip any newlines at and after a position.
    #[inline]
    pub fn skip_newlines(&mut self, pos: u32) -> ParseResult<u32> {
        let pos = pos as usize;
        let len = self.tokens.len();
        if pos >= len {
            return Ok(pos as u32);
        }
        let next = self
            .next_non_newline
            .get(pos)
            .copied()
            .unwrap_or(len as u32) as usize;
        if next == pos + 1 {
            return Ok(pos as u32);
        }
        if next >= len {
            if pos + 1 >= len {
                return Ok(pos as u32);
            }
            return Ok((len - 1) as u32);
        }
        Ok((next - 1) as u32)
    }

    /// Find the next token index that is not a newline.
    #[inline]
    pub fn next_non_newline_index_from(&self, start: usize) -> usize {
        // skip over any newline tokens
        let mut pos = start;
        while let Some(token) = self.tokens.get(pos)
            && token.token.ty == TokenType::Newline
        {
            pos += 1;
        }
        pos
    }

    /// Skip any newlines at and after a position and check if there's a specific token after.
    pub fn peek_token_after_newlines(
        &mut self,
        pos: u32,
        target_token: TokenType,
    ) -> ParseResult<u32> {
        let pos = pos as usize;
        let len = self.tokens.len();
        let next = self
            .next_non_newline
            .get(pos)
            .copied()
            .unwrap_or(len as u32) as usize;
        if let Some(token) = self.tokens.get(next)
            && token.token.ty == target_token
        {
            if next == 0 {
                Ok(0)
            } else {
                Ok((next - 1) as u32)
            }
        } else {
            Err(ParseError::expected(self.peek()?.span, target_token))
        }
    }
}
