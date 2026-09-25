use crate::parse::DeclarationNesting;
use crate::{Parser, ParserError, ParserResult};
use tspp_dir::TokenType;

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

    /// Return whether the current token closes an empty expression slot.
    #[inline]
    pub(crate) fn peek_expression_slot_boundary(&self) -> bool {
        Self::is_expression_slot_boundary_token(self.peek_token_type())
            || self.peek_is(TokenType::Colon)
            || self.peek_declaration_boundary(DeclarationNesting::Expression)
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

    /// Return true when type parsing can recover at the current cursor.
    #[inline]
    pub(crate) fn peek_type_expression_recovery_boundary(&self) -> bool {
        Self::is_type_expression_boundary_token(self.peek_token_type())
            || self.peek_declaration_boundary(DeclarationNesting::None)
    }

    /// Return true when the next token is a statement stop.
    #[inline]
    pub fn peek_statement_stop(&self) -> bool {
        Self::is_statement_stop_token(self.peek_token_type())
    }

    /// Return true when the current token position can terminate a statement.
    #[inline]
    pub fn peek_semicolon_insertion(&self) -> bool {
        self.peek_is_on_new_line()
            || matches!(
                self.peek_token_type(),
                TokenType::Semicolon | TokenType::CloseBrace | TokenType::End
            )
    }

    /// Return true when the next token is an item stop.
    #[inline]
    pub fn peek_item_stop(&self) -> bool {
        Self::is_item_stop_token(self.peek_token_type())
    }

    /// Return true when the next token is any stop.
    #[inline]
    pub fn peek_any_stop(&self) -> bool {
        Self::is_any_stop_token(self.peek_token_type())
    }

    /// Eat a statement stop.
    #[inline]
    pub fn eat_statement_stop(&mut self) -> ParserResult<()> {
        if self.peek_is(TokenType::Semicolon) {
            self.bump();
            return Ok(());
        }

        if self.peek_token_type() == TokenType::End {
            return Ok(());
        }

        Err(ParserError::expected(
            self.peek_token_span(),
            TokenType::Semicolon,
        ))
    }

    /// Eat any stop.
    #[inline]
    pub fn eat_any_stop(&mut self) -> ParserResult<()> {
        if matches!(
            self.peek_token_type(),
            TokenType::Comma | TokenType::Semicolon
        ) {
            self.bump();
            return Ok(());
        }

        if self.peek_token_type() == TokenType::End {
            return Ok(());
        }

        Err(ParserError::unexpected(self.peek_token_span()))
    }
}
