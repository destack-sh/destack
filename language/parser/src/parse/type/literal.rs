use crate::{Parser, ParserError, ParserResult};

use tspp_dir::{Keyword, TokenType, TypeLiteral, VarianceBound};

impl Parser {
    /// Return the always-available type literal at the current token.
    #[inline]
    fn peek_universal_type_literal(&self) -> Option<TypeLiteral> {
        TypeLiteral::from_universal_name(self.peek_token_str())
    }

    /// Return the type-only literal at the current token sequence.
    #[inline]
    fn peek_contextual_type_literal(&self) -> Option<TypeLiteral> {
        TypeLiteral::from_contextual_name(self.peek_token_str())
    }

    /// Parse a variance bound when present.
    ///
    /// Examples:
    /// ```tspp
    /// extends T
    /// implements Shape
    /// super Base
    /// ```
    #[inline]
    pub fn parse_variance_bound_if_present(&mut self) -> Option<VarianceBound> {
        let bound = if self.peek_is_keyword(Keyword::Implements) {
            Some(VarianceBound::Implements)
        } else if self.peek_is_keyword(Keyword::Extends) {
            Some(VarianceBound::Extends)
        } else if self.peek_is_keyword(Keyword::Super) {
            Some(VarianceBound::Super)
        } else {
            None
        };

        if bound.is_some() {
            self.bump();
        }

        bound
    }

    /// Return the explicitly sized type literal at the current token.
    fn peek_sized_type_literal(&self) -> Option<TypeLiteral> {
        TypeLiteral::from_sized_name(self.peek_token_str())
    }

    /// Return the type literal represented by the current token sequence.
    pub fn peek_type_literal(&self) -> Option<TypeLiteral> {
        // require identifier text
        if !self.peek_is(TokenType::Identifier) {
            return None;
        }

        // resolve literals available in every type position
        if let Some(literal) = self.peek_universal_type_literal() {
            return Some(literal);
        }

        // resolve one token contextual literals
        if let Some(literal) = self.peek_contextual_type_literal() {
            return Some(literal);
        }

        // resolve numeric literals with width suffixes
        self.peek_sized_type_literal()
    }

    /// Parse one type literal token sequence.
    ///
    /// Examples:
    /// ```tspp
    /// string
    /// int32
    /// ```
    pub fn parse_type_literal(&mut self) -> ParserResult<TypeLiteral> {
        let literal = self
            .peek_type_literal()
            .ok_or_else(|| ParserError::unexpected(self.peek_token_span()))?;

        // consume the literal token
        self.bump();

        Ok(literal)
    }
}
