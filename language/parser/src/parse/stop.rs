use std::str::FromStr;

use crate::{EXPRESSION_START_TOKEN_TYPES, ParseError, ParseResult, Parser};
use destack_ast::{Keyword, LiteralType, TokenSpan, TokenType, UnaryOperator};

impl Parser {
    /// Return true when the next token is a statement stop.
    #[inline]
    pub fn is_statement_stop(&mut self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::Newline | TokenType::Semicolon | TokenType::End
        )
    }

    /// Return true when the next token is an item stop.
    #[inline]
    pub fn is_item_stop(&mut self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::Comma | TokenType::Newline | TokenType::End
        )
    }

    /// Return true when the next token is any stop.
    #[inline]
    pub fn is_any_stop(&mut self) -> bool {
        matches!(
            self.peek_token_type(),
            TokenType::Comma | TokenType::Semicolon | TokenType::Newline | TokenType::End
        )
    }

    /// Return true when the next token is any stop.
    #[inline]
    pub fn is_next_any_stop(&mut self) -> bool {
        matches!(
            self.peek_next_token_type(),
            TokenType::Comma | TokenType::Semicolon | TokenType::Newline | TokenType::End
        )
    }

    /// Peek an item stop (comma or newline).
    #[inline]
    pub fn peek_item_stop(&mut self) -> ParseResult<&TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Comma || token.token.ty == TokenType::Newline)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(eof_span, TokenType::Comma))
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
            return Err(ParseError::expected(self.eof_span(), TokenType::Comma));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek a statement stop (semicolon or newline).
    #[inline]
    pub fn peek_statement_stop(&mut self) -> ParseResult<&TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(eof_span, TokenType::Newline))
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
            return Err(ParseError::expected(self.eof_span(), TokenType::Newline));
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
            return Err(ParseError::expected(self.eof_span(), TokenType::Newline));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek any stop (comma, semicolon, or newline).
    #[inline]
    pub fn peek_any_stop(&mut self) -> ParseResult<&TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(eof_span, TokenType::Newline))
        }
    }

    /// Peek next any stop (comma, semicolon, or newline).
    #[inline]
    pub fn peek_next_any_stop(&mut self) -> ParseResult<&TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek_next()
            && (token.token.ty == TokenType::Newline
                || token.token.ty == TokenType::Semicolon
                || token.token.ty == TokenType::Comma
                || token.token.ty == TokenType::End)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(eof_span, TokenType::Newline))
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
            return Err(ParseError::expected(self.eof_span(), TokenType::Newline));
        }
        self.eat_newlines_maybe()?;
        Ok(())
    }

    /// Peek any open parenthesis (`(`, `[`, `{`)
    #[inline]
    pub fn peek_any_open_parenthesis(&mut self) -> ParseResult<&TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::OpenParenthesis
                || token.token.ty == TokenType::OpenBracket
                || token.token.ty == TokenType::OpenBrace)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(eof_span, TokenType::OpenParenthesis))
        }
    }

    /// Peek any close parenthesis (`)`, `]`, `}`)
    #[inline]
    pub fn peek_any_close_parenthesis(&mut self) -> ParseResult<&TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek()
            && (token.token.ty == TokenType::CloseParenthesis
                || token.token.ty == TokenType::CloseBracket
                || token.token.ty == TokenType::CloseBrace)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(eof_span, TokenType::CloseParenthesis))
        }
    }

    /// Peek next any close parenthesis (`)`, `]`, `}`)
    #[inline]
    pub fn peek_next_any_close_parenthesis(&mut self) -> ParseResult<&TokenSpan> {
        let eof_span = self.eof_span();
        if let Ok(token) = self.peek_next()
            && (token.token.ty == TokenType::CloseParenthesis
                || token.token.ty == TokenType::CloseBracket
                || token.token.ty == TokenType::CloseBrace)
        {
            Ok(token)
        } else {
            Err(ParseError::expected(eof_span, TokenType::CloseParenthesis))
        }
    }

    /// Return true when the next token is any close parenthesis.
    #[inline]
    pub fn is_next_any_close_parenthesis(&mut self) -> bool {
        matches!(
            self.peek_next_token_type(),
            TokenType::CloseParenthesis | TokenType::CloseBracket | TokenType::CloseBrace
        )
    }

    /// Find a token.
    #[inline]
    pub fn find_token(&mut self, target_token: TokenType) -> ParseResult<u32> {
        let mut pos = self.pos() as usize;
        loop {
            self.ensure_token(pos);
            let Some(token) = self.tokens().get(pos) else {
                break;
            };
            if token.token.ty == target_token {
                return Ok(pos as u32);
            }
            pos += 1;
        }
        Err(ParseError::unexpected(self.eof_span()))
    }

    /// Find a token after a position.
    #[inline]
    pub fn find_token_after(&mut self, pos: u32, target_token: TokenType) -> ParseResult<u32> {
        let mut pos = pos as usize;
        loop {
            self.ensure_token(pos);
            let Some(token) = self.tokens().get(pos) else {
                break;
            };
            if token.token.ty == target_token {
                return Ok(pos as u32);
            }
            pos += 1;
        }
        Err(ParseError::unexpected(self.eof_span()))
    }

    /// Find an open and a matching close token.
    pub fn find_open_and_matching_close(
        &mut self,
        open_token: TokenType,
        close_token: TokenType,
    ) -> ParseResult<u32> {
        let open_pos = self.find_token(open_token)?;
        let close_pos = self.find_matching_close(Some(open_pos), open_token, close_token)?;
        Ok(close_pos)
    }

    /// Find closing pair for a pair of tokens.
    pub fn find_matching_close(
        &mut self,
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
            self.ensure_token(pos);
            let first_token = self
                .token_ref_at(pos)
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
            self.ensure_token(pos);
            if expected_close == Some(close_token)
                && self
                    .token_ref_at(pos)
                    .is_some_and(|token| token.token.ty == open_token)
                && let Some(matching) = self.matching_pair(pos)
            {
                return Ok(matching as u32);
            }
        }

        // seek until we find the matching close token
        loop {
            self.ensure_token(pos);
            let Some(token) = self.tokens().get(pos) else {
                break;
            };
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

        Err(ParseError::expected(self.eof_span(), open_token))
    }

    /// Find a matching close token in expression contexts with tree literal awareness.
    pub fn find_matching_close_in_expression(
        &mut self,
        open_pos: u32,
        open_token: TokenType,
        close_token: TokenType,
    ) -> ParseResult<u32> {
        let mut depth = 0u32;
        let mut pos = open_pos as usize;
        let mut last_non_whitespace_index: Option<usize> = None;
        let mut last_semantic_index: Option<usize> = None;
        let mut prev_semantic_index: Option<usize> = None;
        let tree_literals_allowed =
            self.language.supports_jsx() && self.allow_tree_literals() && !self.options.in_type;

        // we should start at the expected open token
        #[cfg(debug_assertions)]
        {
            self.ensure_token(pos);
            let first_token = self
                .token_ref_at(pos)
                .map(|token| token.token.ty)
                .unwrap_or(TokenType::End);
            debug_assert_eq!(
                first_token, open_token,
                "expected open token {open_token:?} but got {first_token:?}"
            );
        }

        // scan until we reach the matching close token
        while let Some(token) = self.token_ref_at(pos) {
            let ty = token.token.ty;

            // enter tree literal mode when a JSX literal starts at an expression boundary
            if ty == TokenType::LessThan && tree_literals_allowed {
                let can_start_expression = self.is_expression_start_after_tokens(
                    last_non_whitespace_index,
                    last_semantic_index,
                    prev_semantic_index,
                );
                if can_start_expression {
                    let can_start_tree =
                        self.with_pos(pos, |parser| parser.can_start_tree_literal());
                    if can_start_tree {
                        self.enter_tree_opening_tag();
                    }
                }
            }

            // track opening and closing tokens
            if ty == open_token {
                depth += 1;
            } else if ty == close_token {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(pos as u32);
                }
            }

            // track previous tokens for expression boundary checks
            last_non_whitespace_index = Some(pos);
            if ty != TokenType::Newline {
                prev_semantic_index = last_semantic_index;
                last_semantic_index = Some(pos);
            }
            pos += 1;
        }

        Err(ParseError::expected(self.eof_span(), open_token))
    }

    /// Find a token *within* a matching pair of tokens.
    pub fn find_before_matching_close(
        &mut self,
        open_token: TokenType,
        close_token: TokenType,
        target_type: TokenType,
    ) -> ParseResult<u32> {
        let mut depth = 0;
        let mut pos = self.pos() as usize;
        loop {
            self.ensure_token(pos);
            let Some(token) = self.tokens().get(pos) else {
                break;
            };
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
        Err(ParseError::expected(self.eof_span(), open_token))
    }

    /// Check whether a parenthesized expression has a top level token.
    pub fn has_token_before_matching_close(
        &mut self,
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
        let mut last_non_whitespace_index: Option<usize> = None;
        let mut last_semantic_index: Option<usize> = None;
        let mut prev_semantic_index: Option<usize> = None;
        let tree_literals_allowed =
            self.language.supports_jsx() && self.allow_tree_literals() && !self.options.in_type;

        // scan the parenthesis contents
        while pos <= close_pos {
            let ty = match self.token_ref_at(pos) {
                Some(token) => token.token.ty,
                None => break,
            };

            // enter tree literal mode when a JSX literal starts at an expression boundary
            if ty == TokenType::LessThan && tree_literals_allowed {
                let can_start_expression = self.is_expression_start_after_tokens(
                    last_non_whitespace_index,
                    last_semantic_index,
                    prev_semantic_index,
                );
                if can_start_expression {
                    let can_start_tree =
                        self.with_pos(pos, |parser| parser.can_start_tree_literal());
                    if can_start_tree {
                        self.enter_tree_opening_tag();
                    }
                }
            }

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

            last_non_whitespace_index = Some(pos);
            if ty != TokenType::Newline {
                prev_semantic_index = last_semantic_index;
                last_semantic_index = Some(pos);
            }
            pos += 1;
        }

        Ok(false)
    }

    /// Return true when the current position can start an expression.
    fn is_expression_start_after_tokens(
        &mut self,
        last_non_whitespace_index: Option<usize>,
        last_semantic_index: Option<usize>,
        prev_semantic_index: Option<usize>,
    ) -> bool {
        let Some(last_non_whitespace_index) = last_non_whitespace_index else {
            return true;
        };
        let Some(last_non_whitespace) = self.token_ref_at(last_non_whitespace_index) else {
            return true;
        };

        if last_non_whitespace.token.ty == TokenType::Newline {
            let Some(last_semantic_index) = last_semantic_index else {
                return true;
            };
            let Some(last_semantic) = self.token_ref_at(last_semantic_index).copied() else {
                return true;
            };
            let prev_semantic =
                prev_semantic_index.and_then(|index| self.token_ref_at(index).copied());
            return self.is_statement_boundary_after_newline_tokens(
                &last_semantic,
                prev_semantic.as_ref(),
            );
        }

        if EXPRESSION_START_TOKEN_TYPES.contains(&last_non_whitespace.token.ty) {
            return true;
        }

        if last_non_whitespace.token.ty == TokenType::Identifier {
            let keyword = self.keyword_for_index(last_non_whitespace_index);
            return keyword
                .map(|keyword| {
                    keyword.is_control()
                        || keyword == Keyword::Delete
                        || UnaryOperator::from_prefix_keyword(keyword).is_some()
                })
                .unwrap_or(false);
        }

        false
    }

    /// Return true when a newline terminates a statement at this token.
    fn is_statement_boundary_after_newline_tokens(
        &self,
        last_semantic: &TokenSpan,
        prev_semantic: Option<&TokenSpan>,
    ) -> bool {
        if last_semantic.token.ty == TokenType::Semicolon {
            return true;
        }

        if last_semantic.token.ty == TokenType::CloseBrace {
            return true;
        }

        if last_semantic.token.ty == TokenType::Literal
            && matches!(
                last_semantic.token.literal,
                Some(LiteralType::RegexString { .. })
            )
        {
            return true;
        }

        if last_semantic.token.ty == TokenType::Identifier
            && let Ok(keyword) = Keyword::from_str(self.get_span_str(last_semantic.span))
            && matches!(
                keyword,
                Keyword::Return
                    | Keyword::Break
                    | Keyword::Continue
                    | Keyword::Debugger
                    | Keyword::Yield
            )
        {
            return true;
        }

        if let Some(prev_token) = prev_semantic
            && last_semantic.token.ty == TokenType::Identifier
            && prev_token.token.ty == TokenType::Identifier
            && let Ok(keyword) = Keyword::from_str(self.get_span_str(prev_token.span))
            && matches!(keyword, Keyword::Let | Keyword::Var)
        {
            return true;
        }

        false
    }

    /// Skip any newlines at and after a position.
    #[inline]
    pub fn skip_newlines(&mut self, pos: u32) -> ParseResult<u32> {
        let pos = pos as usize;
        self.ensure_token(pos);
        let len = self.tokens().len();
        if pos >= len {
            return Ok(pos as u32);
        }

        let next = self.next_non_newline_index_from_stream(pos);
        if next == pos {
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
    pub fn next_non_newline_index_from(&mut self, start: usize) -> usize {
        self.next_non_newline_index_from_stream(start)
    }

    /// Skip any newlines at and after a position and check if there's a specific token after.
    #[inline]
    pub fn is_token_after_newlines(&mut self, pos: u32, target_token: TokenType) -> bool {
        let mut start = pos as usize;
        self.ensure_token(start);
        if self
            .tokens()
            .get(start)
            .is_some_and(|token| token.token.ty != TokenType::Newline)
        {
            start = start.saturating_add(1);
        }

        let cursor = self.non_newline_cursor_from(start);
        cursor.token_type == target_token
    }

    /// Skip any newlines at and after a position and check if there's a specific token after.
    pub fn peek_token_after_newlines(
        &mut self,
        pos: u32,
        target_token: TokenType,
    ) -> ParseResult<u32> {
        let mut start = pos as usize;
        self.ensure_token(start);
        if self
            .tokens()
            .get(start)
            .is_some_and(|token| token.token.ty != TokenType::Newline)
        {
            start = start.saturating_add(1);
        }

        let cursor = self.non_newline_cursor_from(start);
        if cursor.token_type == target_token {
            if cursor.index == 0 {
                Ok(0)
            } else {
                Ok((cursor.index - 1) as u32)
            }
        } else {
            let span = self
                .tokens()
                .get(cursor.index)
                .map(|token| token.span)
                .unwrap_or(self.eof_span());
            Err(ParseError::expected(span, target_token))
        }
    }
}
