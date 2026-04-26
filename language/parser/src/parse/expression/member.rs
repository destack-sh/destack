use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Expression, Keyword, LiteralType, LocalNodeId, NodeType, ScalarLiteral, TokenType,
    TypeExpression,
};
use destack_core::StringId;
use destack_source::Span;

impl Parser {
    /// Return true when an expression is a decimal integer token directly before `.`.
    #[inline]
    pub(super) fn expression_is_decimal_integer_before_dot(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        dot_span: Span,
    ) -> bool {
        // only integer scalar literals can use decimal separators for member access
        if !matches!(
            self.tree.get(left_expression_id),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_))
        ) {
            return false;
        }

        // the dot must be directly adjacent to the literal span
        let left_span = self.tree.get_span(left_expression_id);
        if left_span.end != dot_span.start {
            return false;
        }

        // only decimal integer source text needs the extra separator dot
        let source_text = self.get_span_str(left_span);
        !source_text.ends_with('n')
            && !source_text.starts_with("0x")
            && !source_text.starts_with("0X")
            && !source_text.starts_with("0b")
            && !source_text.starts_with("0B")
            && !source_text.starts_with("0o")
            && !source_text.starts_with("0O")
    }

    /// Return true when a decimal integer uses member access without a separator.
    #[inline]
    pub(super) fn invalid_decimal_integer_member_access(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        distance: u8,
        dot_span: Span,
    ) -> bool {
        // only direct `.name` member access needs the separator rule
        if distance != 2 {
            return false;
        }

        self.expression_is_decimal_integer_before_dot(left_expression_id, dot_span)
    }

    /// Return true when a type expression is a decimal integer token directly before `.`.
    #[inline]
    pub(super) fn type_is_decimal_integer_before_dot(
        &mut self,
        left_type_id: LocalNodeId<TypeExpression>,
        dot_span: Span,
    ) -> bool {
        // only integer scalar literals can use decimal separators for member access
        if !matches!(
            self.tree.get(left_type_id),
            TypeExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(_),
            }
        ) {
            return false;
        }

        // the dot must be directly adjacent to the literal span
        let left_span = self.tree.get_span(left_type_id);
        if left_span.end != dot_span.start {
            return false;
        }

        // only decimal integer source text needs the extra separator dot
        let source_text = self.get_span_str(left_span);
        !source_text.ends_with('n')
            && !source_text.starts_with("0x")
            && !source_text.starts_with("0X")
            && !source_text.starts_with("0b")
            && !source_text.starts_with("0B")
            && !source_text.starts_with("0o")
            && !source_text.starts_with("0O")
    }

    /// Return true when a decimal integer type uses member access without a separator.
    #[inline]
    pub(super) fn invalid_decimal_integer_type_member_access(
        &mut self,
        left_type_id: LocalNodeId<TypeExpression>,
        distance: u8,
        dot_span: Span,
    ) -> bool {
        // only direct `.name` member access needs the separator rule
        if distance != 2 {
            return false;
        }

        self.type_is_decimal_integer_before_dot(left_type_id, dot_span)
    }

    /// Eat a static member name and return both the name and its span.
    #[inline]
    pub(super) fn eat_member_name_with_span(&mut self) -> ParseResult<(StringId, Span)> {
        // identifier member name
        if self.peek_is(TokenType::Identifier) {
            self.eat_identifier_with_span()
        }
        // boolean literal member name
        else if self.peek_is(TokenType::Literal)
            && self
                .peek()
                .is_ok_and(|token| matches!(token.token.literal, Some(LiteralType::Boolean { .. })))
        {
            let token = *self.eat()?;
            let text = self.get_token_str(token).to_owned();
            let name = self.strings.intern(&text);
            Ok((name, token.span))
        }
        // invalid member name
        else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Check whether `?.` starts an optional chaining segment.
    #[inline]
    pub(super) fn is_optional_chain_after_maybe(&mut self) -> bool {
        // require ?. before we look at the target
        if self.next_token_type() != TokenType::Dot {
            return false;
        }

        // accept valid optional chain targets after ?., including line-delimited targets
        let next_target_type = self.lookahead(|parser| {
            parser.bump();
            parser.bump();
            parser.peek_token_type()
        });
        matches!(
            next_target_type,
            TokenType::Identifier
                | TokenType::OpenBracket
                | TokenType::OpenParenthesis
                | TokenType::Hash
                | TokenType::LessThan
                | TokenType::ShiftLeft
                | TokenType::TemplateStringStart
                | TokenType::TemplateString
        ) || Self::is_expression_slot_boundary_token(next_target_type)
    }

    /// Check whether `asserts` starts a type predicate.
    #[inline]
    pub(super) fn can_start_type_predicate_asserts(&mut self) -> bool {
        if !self.is_keyword(Keyword::Asserts) {
            return false;
        }

        if self.next_keyword() == Some(Keyword::This) {
            return true;
        }

        self.next_token_type() == TokenType::Identifier
    }

    /// Eat a parenthesized expression and return the inner expression.
    pub fn eat_parenthesized_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        self.eat_token(TokenType::OpenParenthesis)?;

        let expression_id = self.eat_expression(self.options)?;

        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        Ok(expression_id)
    }
}
