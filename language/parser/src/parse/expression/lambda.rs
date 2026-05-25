use crate::parse::DeclarationHeader;
use crate::parse::scan::DelimiterDepth;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};
use destack_dir::{Expression, Keyword, LocalNodeId, TokenType};

/// The commitment level of a possible lambda head.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum LambdaHead {
    /// The cursor does not start a lambda head.
    None,
    /// The cursor starts an ambiguous lambda head.
    Tentative,
    /// The cursor definitely starts a lambda head.
    Definite,
}

impl Parser {
    /// Try to parse an ambiguous arrow function expression.
    ///
    /// Examples:
    /// ```ds
    /// (value) => value
    /// value => value
    /// async <T>(value: T) => value
    /// ```
    pub(in crate::parse::expression) fn eat_lambda_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        let head = self.current_lambda_head();
        if head == LambdaHead::None {
            return Ok(None);
        }

        let checkpoint = self.checkpoint();
        let mark = self.tree.next_id();
        match self.eat_function(start, DeclarationHeader::default()) {
            Ok(declaration) => Ok(Some(self.declaration_expression(start, declaration))),
            Err(error) => {
                if head == LambdaHead::Definite {
                    return self.definite_lambda_error(error);
                }

                self.restore(checkpoint, mark);
                Err(error)
            }
        }
    }

    /// Return the current lambda head commitment.
    fn current_lambda_head(&mut self) -> LambdaHead {
        if self.current_keyword() == Some(Keyword::Async) {
            return self.current_async_lambda_head();
        }

        if self.peek_is(TokenType::OpenParenthesis) && self.current_parenthesis_starts_lambda_head()
        {
            return LambdaHead::Definite;
        }

        if self.peek_is(TokenType::Identifier)
            && matches!(self.next_token_type(), TokenType::ArrowWide)
        {
            return LambdaHead::Tentative;
        }

        LambdaHead::None
    }

    /// Return the current async lambda head commitment.
    fn current_async_lambda_head(&mut self) -> LambdaHead {
        if matches!(self.next_token_type(), TokenType::ArrowWide) {
            return LambdaHead::Tentative;
        }

        if !self.can_start_async_lambda_head() {
            return LambdaHead::None;
        }

        if self.next_token_type() == TokenType::OpenParenthesis
            && self.parenthesized_lambda_head_starts_at(1)
        {
            return LambdaHead::Definite;
        }

        LambdaHead::Tentative
    }

    /// Return a parse error from a definite lambda head.
    fn definite_lambda_error(
        &mut self,
        error: ParserError,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        if self.peek_is(TokenType::CloseParenthesis) {
            let error = ParserError::unexpected(self.peek()?.span);
            self.error(&error);

            return Err(ParserError::from_source(error.span, error));
        }

        Err(error)
    }

    /// Return whether `async` can begin an async lambda or function expression.
    fn can_start_async_lambda_head(&mut self) -> bool {
        if self.next_token().token.is_on_new_line {
            return false;
        }

        if self.next_keyword() == Some(Keyword::Function) {
            return true;
        }

        match self.next_token_type() {
            TokenType::Identifier => matches!(self.token_type_at_offset(2), TokenType::ArrowWide),
            TokenType::OpenParenthesis => self.parenthesized_lambda_head_starts_at(1),
            TokenType::LessThan => self.async_generic_lambda_head_starts_after_async(),
            _ => false,
        }
    }

    /// Return whether `async <...>` starts a generic lambda head.
    fn async_generic_lambda_head_starts_after_async(&mut self) -> bool {
        let checkpoint = self.cursor_checkpoint();
        self.bump();
        let starts = self.can_start_generic_arrow_expression();
        self.rewind(checkpoint);

        starts
    }

    /// Return whether the current parenthesized group is directly followed by an arrow marker.
    fn current_parenthesis_starts_lambda_head(&mut self) -> bool {
        self.parenthesized_lambda_head_starts_at(0)
    }

    /// Return whether an offset parenthesized group starts a lambda head.
    fn parenthesized_lambda_head_starts_at(&mut self, start_offset: usize) -> bool {
        self.lookahead(|parser| parser.scan_parenthesized_lambda_head_starts_at(start_offset))
    }

    /// Scan whether an offset parenthesized group starts a lambda head.
    fn scan_parenthesized_lambda_head_starts_at(&mut self, start_offset: usize) -> bool {
        for _ in 0..start_offset {
            if self.peek_is(TokenType::End) {
                return false;
            }

            self.bump();
        }

        if !self.peek_is(TokenType::OpenParenthesis) {
            return false;
        }

        self.bump();
        if !self.current_token_is_plausible_parameter_head() {
            return false;
        }

        self.scan_parenthesized_follow_token_after_open()
            .is_some_and(|_| self.current_token_starts_lambda_head_follow())
    }

    /// Return whether the current token can begin a parameter list.
    fn current_token_is_plausible_parameter_head(&mut self) -> bool {
        match self.peek_token_type() {
            TokenType::CloseParenthesis => true,
            TokenType::Identifier => self.identifier_parameter_head_follow_is_plausible(),
            TokenType::OpenBrace | TokenType::OpenBracket | TokenType::Spread => true,
            TokenType::At => true,
            _ => false,
        }
    }

    /// Return whether an identifier can continue as a parameter head.
    fn identifier_parameter_head_follow_is_plausible(&mut self) -> bool {
        let token_type = self.next_token_type();

        // accept optional parameters only in annotation form
        if token_type == TokenType::Maybe {
            return self.token_type_at_offset(2) == TokenType::Colon;
        }

        matches!(
            token_type,
            TokenType::CloseParenthesis
                | TokenType::Comma
                | TokenType::Colon
                | TokenType::Assign
                | TokenType::Identifier
        )
    }

    /// Return whether a closed parameter list is followed by an arrow.
    fn current_token_starts_lambda_head_follow(&mut self) -> bool {
        match self.peek_token_type() {
            TokenType::ArrowWide => true,
            TokenType::Colon => {
                self.bump();

                self.return_type_is_followed_by_arrow()
            }
            _ => false,
        }
    }

    /// Return whether a return type annotation is followed by an arrow.
    fn return_type_is_followed_by_arrow(&mut self) -> bool {
        let mut depth = DelimiterDepth::default();

        loop {
            let token_type = self.peek_token_type();

            // recover before rescanning later statements
            if self.current_token_is_statement_recovery_boundary(token_type) {
                return false;
            }

            // arrow closes the return type
            if depth.is_top_level() && matches!(token_type, TokenType::ArrowWide) {
                return true;
            }

            // hard boundaries cannot belong to the return type
            if depth.is_top_level()
                && matches!(
                    token_type,
                    TokenType::End
                        | TokenType::Comma
                        | TokenType::Semicolon
                        | TokenType::CloseParenthesis
                        | TokenType::CloseBracket
                        | TokenType::CloseBrace
                )
            {
                return false;
            }

            if !depth.advance(token_type) {
                return false;
            }

            self.bump();
        }
    }
}
