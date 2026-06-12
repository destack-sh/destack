use crate::{Parser, ParserError, ParserResult};
use destack_dir::{TokenSpan, TokenType};
use destack_source::Span;

impl Parser {
    /// Return true when a token type closes one grouping delimiter.
    #[inline]
    pub(crate) const fn is_close_delimiter_token(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::CloseParenthesis | TokenType::CloseBracket | TokenType::CloseBrace
        )
    }

    /// Return true when a token type is a statement stop.
    #[inline]
    pub(crate) const fn is_statement_stop_token(token_type: TokenType) -> bool {
        matches!(token_type, TokenType::Semicolon | TokenType::End)
    }

    /// Return true when a token type is an item stop.
    #[inline]
    pub(crate) const fn is_item_stop_token(token_type: TokenType) -> bool {
        matches!(token_type, TokenType::Comma | TokenType::End)
    }

    /// Return true when a token type is any stop.
    #[inline]
    pub(crate) const fn is_any_stop_token(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Comma | TokenType::Semicolon | TokenType::End
        )
    }

    /// Return true when an expression child can recover a missing node here.
    #[inline]
    pub(crate) const fn is_expression_slot_boundary_token(token_type: TokenType) -> bool {
        Self::is_close_delimiter_token(token_type)
            || matches!(
                token_type,
                TokenType::Comma | TokenType::Semicolon | TokenType::End
            )
    }

    /// Return true when a close delimiter can recover a missing token here.
    #[inline]
    pub(crate) const fn is_close_delimiter_boundary_token(token_type: TokenType) -> bool {
        Self::is_close_delimiter_token(token_type) || Self::is_statement_stop_token(token_type)
    }

    /// Return true when a type expression can recover a missing child here.
    #[inline]
    pub(crate) const fn is_type_expression_boundary_token(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Colon
                | TokenType::Assign
                | TokenType::ArrowWide
                | TokenType::GreaterThan
                | TokenType::Maybe
                | TokenType::End
        ) || Self::is_close_delimiter_token(token_type)
    }

    /// Return true when a type container can recover a missing close token here.
    #[inline]
    pub(crate) const fn is_type_container_boundary_token(token_type: TokenType) -> bool {
        Self::is_type_expression_boundary_token(token_type)
            || matches!(
                token_type,
                TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::Dot
            )
    }

    /// Return true when a type expression can recover a missing child here.
    #[inline]
    pub(crate) fn is_type_expression_boundary(&mut self) -> bool {
        Self::is_type_expression_boundary_token(self.peek_token_type())
    }

    /// Return true when the next token is a statement stop.
    #[inline]
    pub fn is_statement_stop(&mut self) -> bool {
        Self::is_statement_stop_token(self.peek_token_type())
    }

    /// Return true when the current token position can terminate a statement.
    #[inline]
    pub fn can_insert_semicolon(&mut self) -> bool {
        self.current_token_is_on_new_line()
            || matches!(
                self.peek_token_type(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            )
    }

    /// Return true when the next token is an item stop.
    #[inline]
    pub fn is_item_stop(&mut self) -> bool {
        Self::is_item_stop_token(self.peek_token_type())
    }

    /// Return true when the next token is any stop.
    #[inline]
    pub fn is_any_stop(&mut self) -> bool {
        Self::is_any_stop_token(self.peek_token_type())
    }

    /// Return true when the next token is any stop.
    #[inline]
    pub fn is_next_any_stop(&mut self) -> bool {
        let token = self.next_token();

        token.is_on_new_line() || Self::is_any_stop_token(token.ty())
    }

    /// Peek an item stop.
    #[inline]
    pub fn peek_item_stop(&mut self) -> ParserResult<TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && Self::is_item_stop_token(token.token.ty())
            && token.token.ty() != TokenType::End
        {
            Ok(token)
        } else {
            Err(ParserError::expected(eof_span, TokenType::Comma))
        }
    }

    /// Eat an item stop.
    #[inline]
    pub fn eat_item_stop(&mut self) -> ParserResult<()> {
        if let Ok(token) = self.peek()
            && token.token.ty() == TokenType::Comma
        {
            self.bump();
        } else {
            return Err(ParserError::expected(self.eof_span(), TokenType::Comma));
        }
        Ok(())
    }

    /// Peek a statement stop.
    #[inline]
    pub fn peek_statement_stop(&mut self) -> ParserResult<TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && Self::is_statement_stop_token(token.token.ty())
        {
            Ok(token)
        } else {
            Err(ParserError::expected(eof_span, TokenType::Semicolon))
        }
    }

    /// Eat a statement stop.
    #[inline]
    pub fn eat_statement_stop(&mut self) -> ParserResult<()> {
        if let Ok(token) = self.peek()
            && token.token.ty() == TokenType::Semicolon
        {
            self.bump(); // eat semicolon
            return Ok(());
        }

        if self.peek_token_type() == TokenType::End {
            return Ok(());
        }

        Err(ParserError::expected(self.eof_span(), TokenType::Semicolon))
    }

    /// Peek any stop.
    #[inline]
    pub fn peek_any_stop(&mut self) -> ParserResult<TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && Self::is_any_stop_token(token.token.ty())
        {
            Ok(token)
        } else {
            Err(ParserError::expected(eof_span, TokenType::Semicolon))
        }
    }

    /// Peek next any stop.
    #[inline]
    pub fn peek_next_any_stop(&mut self) -> ParserResult<TokenSpan> {
        let eof_span = self.eof_span();
        let token = self.next_token();
        if token.is_on_new_line() || Self::is_any_stop_token(token.ty()) {
            return Ok(TokenSpan::new(token, self.file_id));
        }

        Err(ParserError::expected(eof_span, TokenType::Semicolon))
    }

    /// Eat any stop.
    #[inline]
    pub fn eat_any_stop(&mut self) -> ParserResult<()> {
        if let Ok(token) = self.peek()
            && matches!(token.token.ty(), TokenType::Comma | TokenType::Semicolon)
        {
            self.bump();
            return Ok(());
        }

        if self.peek_token_type() == TokenType::End {
            return Ok(());
        }

        Err(ParserError::expected(self.eof_span(), TokenType::Semicolon))
    }

    /// Peek any open parenthesis (`(`, `[`, `{`)
    #[inline]
    pub fn peek_any_open_parenthesis(&mut self) -> ParserResult<TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && (token.token.ty() == TokenType::OpenParenthesis
                || token.token.ty() == TokenType::OpenBracket
                || token.token.ty() == TokenType::OpenBrace)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(eof_span, TokenType::OpenParenthesis))
        }
    }

    /// Peek any close parenthesis (`)`, `]`, `}`)
    #[inline]
    pub fn peek_any_close_parenthesis(&mut self) -> ParserResult<TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && (token.token.ty() == TokenType::CloseParenthesis
                || token.token.ty() == TokenType::CloseBracket)
        {
            Ok(token)
        } else {
            Err(ParserError::expected(eof_span, TokenType::CloseParenthesis))
        }
    }

    /// Peek next any close parenthesis (`)`, `]`, `}`)
    #[inline]
    pub fn peek_next_any_close_parenthesis(&mut self) -> ParserResult<TokenSpan> {
        let eof_span = self.eof_span();
        let token = self.next_token();
        if token.ty() == TokenType::CloseParenthesis || token.ty() == TokenType::CloseBracket {
            return Ok(TokenSpan::new(token, self.file_id));
        }

        Err(ParserError::expected(eof_span, TokenType::CloseParenthesis))
    }

    /// Return true when the next token is any close parenthesis.
    #[inline]
    pub fn is_next_any_close_parenthesis(&mut self) -> bool {
        matches!(
            self.token_type_at_offset(1),
            TokenType::CloseParenthesis | TokenType::CloseBracket | TokenType::CloseBrace
        )
    }

    /// Find a token at or after the current cursor.
    #[inline]
    pub fn find_token(&mut self, target_token: TokenType) -> ParserResult<Span> {
        self.lookahead(|parser| {
            while parser.peek_token_type() != TokenType::End {
                if parser.peek_is(target_token) {
                    return Ok(parser.current_token().span(parser.file_id));
                }

                parser.bump();
            }

            Err(ParserError::unexpected(parser.eof_span()))
        })
    }

    /// Find a token at or after a source byte position.
    #[inline]
    pub fn find_token_after(
        &mut self,
        position: u32,
        target_token: TokenType,
    ) -> ParserResult<Span> {
        self.lookahead(|parser| {
            while parser.current_token().start() < position
                && parser.peek_token_type() != TokenType::End
            {
                parser.bump();
            }

            while parser.peek_token_type() != TokenType::End {
                if parser.peek_is(target_token) {
                    return Ok(parser.current_token().span(parser.file_id));
                }

                parser.bump();
            }

            Err(ParserError::unexpected(parser.eof_span()))
        })
    }

    /// Find the matching close token for the current opening delimiter.
    pub fn find_matching_close(
        &mut self,
        open_token: TokenType,
        close_token: TokenType,
    ) -> ParserResult<Span> {
        self.lookahead(|parser| {
            if parser.peek_token_type() != open_token {
                return Err(ParserError::expected(
                    parser.current_token().span(parser.file_id),
                    open_token,
                ));
            }

            let mut depth = 0u32;
            while parser.peek_token_type() != TokenType::End {
                let token_type = parser.peek_token_type();
                if token_type == open_token {
                    depth += 1;
                } else if token_type == close_token {
                    depth = depth.saturating_sub(1);
                }

                if depth == 0 {
                    return Ok(parser.current_token().span(parser.file_id));
                }

                parser.bump();
            }

            Err(ParserError::expected(parser.eof_span(), open_token))
        })
    }

    /// Find the matching close token after the opening delimiter has been consumed.
    pub fn find_matching_close_after_open(
        &mut self,
        open_token: TokenType,
        close_token: TokenType,
    ) -> ParserResult<Span> {
        self.lookahead(|parser| {
            if parser.prev_token_type() != open_token {
                let span = parser
                    .prev()
                    .map(|token| token.span)
                    .unwrap_or_else(|| parser.current_token().span(parser.file_id));
                return Err(ParserError::expected(span, open_token));
            }

            let mut depth = 1u32;
            while parser.peek_token_type() != TokenType::End {
                let token_type = parser.peek_token_type();
                if token_type == open_token {
                    depth += 1;
                } else if token_type == close_token {
                    depth = depth.saturating_sub(1);
                }

                if depth == 0 {
                    return Ok(parser.current_token().span(parser.file_id));
                }

                parser.bump();
            }

            Err(ParserError::expected(parser.eof_span(), open_token))
        })
    }

    /// Find a matching close token, returning `None` when the close token is missing.
    pub fn find_matching_close_maybe(
        &mut self,
        open_token: TokenType,
        close_token: TokenType,
    ) -> Option<Span> {
        self.find_matching_close(open_token, close_token).ok()
    }

    /// Find a matching close token after an opening token, returning `None` when the close token is missing.
    pub fn find_matching_close_after_open_maybe(
        &mut self,
        open_token: TokenType,
        close_token: TokenType,
    ) -> Option<Span> {
        self.find_matching_close_after_open(open_token, close_token)
            .ok()
    }

    /// Find a token *within* a matching pair of tokens.
    pub fn find_before_matching_close(
        &mut self,
        open_token: TokenType,
        close_token: TokenType,
        target_type: TokenType,
    ) -> ParserResult<Span> {
        self.lookahead(|parser| {
            let mut depth = 0u32;
            while parser.peek_token_type() != TokenType::End {
                let token_type = parser.peek_token_type();
                if token_type == open_token {
                    depth += 1;
                } else if token_type == close_token {
                    depth = depth.saturating_sub(1);
                } else if depth == 1 && token_type == target_type {
                    return Ok(parser.current_token().span(parser.file_id));
                }

                if depth == 0 {
                    break;
                }

                parser.bump();
            }

            Err(ParserError::expected(parser.eof_span(), open_token))
        })
    }

    /// Check whether a parenthesized expression has a top level token.
    pub fn has_token_before_matching_close_after_open(
        &mut self,
        close_span: Span,
        target_token: TokenType,
        track_angle: bool,
    ) -> ParserResult<bool> {
        self.lookahead(|parser| {
            let mut paren_depth = 1u32;
            let mut brace_depth = 0u32;
            let mut bracket_depth = 0u32;
            let mut angle_depth = 0u32;

            while parser.current_token().start() <= close_span.start
                && parser.peek_token_type() != TokenType::End
            {
                let token_type = parser.peek_token_type();
                match token_type {
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
                    TokenType::ShiftLeft if track_angle => {
                        angle_depth += 2;
                    }
                    TokenType::ShiftRight if track_angle => {
                        angle_depth = angle_depth.saturating_sub(2);
                    }
                    TokenType::UnsignedShiftRight if track_angle => {
                        angle_depth = angle_depth.saturating_sub(3);
                    }
                    _ if token_type == target_token
                        && paren_depth == 1
                        && brace_depth == 0
                        && bracket_depth == 0
                        && angle_depth == 0 =>
                    {
                        return Ok(true);
                    }
                    _ => {}
                }

                parser.bump();
            }

            Ok(false)
        })
    }
}
