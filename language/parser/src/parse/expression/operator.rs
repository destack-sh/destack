use std::str::FromStr;

use crate::{ParseError, ParseResult, Parser};

use super::common::{
    NOT_IN_FOR_EACH_BINARY_OPERATORS, NOT_IN_GENERIC_ARGUMENT_BINARY_OPERATORS,
    NOT_IN_TREE_BINARY_OPERATORS,
};

use destack_ast::{
    AssignOperator, BinaryOperator, Expression, Keyword, LocalNodeId, TokenSpan, TokenType,
    TypeBinaryOperator, TypeExpression, TypeUnaryOperator, UnaryOperator,
};
use destack_source::Span;

const AS_ASSERTION_PRECEDENCE: u16 = 1355;
const SATISFIES_ASSERTION_PRECEDENCE: u16 = 1003;

/// One parser-local infix continuation operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) enum ParseInfixOperator {
    /// A binary operator continuation.
    Binary(BinaryOperator),
    /// One `as` assertion continuation.
    As,
    /// One `satisfies` assertion continuation.
    Satisfies,
    /// A type binary operator continuation.
    TypeBinary(TypeBinaryOperator),
    /// An assignment operator continuation.
    Assign(AssignOperator),
}

impl ParseInfixOperator {
    /// Return the parser precedence for one infix continuation operator.
    #[inline]
    pub(super) fn precedence(self) -> u16 {
        match self {
            ParseInfixOperator::Binary(binary_operator) => binary_operator.precedence(),
            ParseInfixOperator::As => AS_ASSERTION_PRECEDENCE,
            ParseInfixOperator::Satisfies => SATISFIES_ASSERTION_PRECEDENCE,
            ParseInfixOperator::TypeBinary(type_binary_operator) => {
                type_binary_operator.precedence()
            }
            ParseInfixOperator::Assign(assign_operator) => assign_operator.precedence(),
        }
    }
}

impl Parser {
    /// Return the current token span or EOF span when unavailable.
    #[inline]
    fn current_span_or_eof(&mut self) -> Span {
        self.peek()
            .map(|token| token.span)
            .unwrap_or(self.eof_span())
    }

    /// Return true when contextual cast keywords continue as identifiers.
    #[inline]
    fn contextual_cast_keyword_continues_identifier(
        &self,
        keyword: Keyword,
        next_token_type: TokenType,
        next_next_token_type: TokenType,
    ) -> bool {
        // only `as` and `satisfies` are contextual type operators in value expressions
        if !matches!(keyword, Keyword::As | Keyword::Satisfies) {
            return false;
        }

        // direct and newline member access keep the keyword as an identifier
        let has_member_after_keyword = next_token_type == TokenType::Dot
            || (next_token_type == TokenType::Newline && next_next_token_type == TokenType::Dot);

        // optional chain member access keeps the keyword as an identifier
        let has_optional_member_after_keyword =
            next_token_type == TokenType::Maybe && next_next_token_type == TokenType::Dot;

        has_member_after_keyword || has_optional_member_after_keyword
    }

    /// Return the keyword token text for identifier based infix operators.
    #[inline]
    fn infix_identifier_keyword_text(keyword: Keyword) -> Option<&'static str> {
        match keyword {
            Keyword::In => Some("in"),
            Keyword::Is => Some("is"),
            Keyword::As => Some("as"),
            Keyword::Extends => Some("extends"),
            Keyword::Implements => Some("implements"),
            Keyword::InstanceOf => Some("instanceof"),
            Keyword::Satisfies => Some("satisfies"),
            _ => None,
        }
    }

    /// Make an infix operator from the current parser context.
    fn classify_infix_operator(
        &self,
        token_str: &str,
        token: &TokenSpan,
        next_token: Option<&TokenSpan>,
        next_next_token: Option<&TokenSpan>,
        has_newline: bool,
    ) -> ParseResult<(ParseInfixOperator, u8)> {
        // regular binary operator
        // (only a subset of binary operators are allowed in generic argument and tree contexts)
        if let Some(binary_operator) = BinaryOperator::from_token(token_str, token.token.ty)
            && (!self.options.is_in_type()
                || self.options.is_in_static()
                || matches!(
                    binary_operator,
                    BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                ))
            && (!self.options.is_in_static()
                || !NOT_IN_GENERIC_ARGUMENT_BINARY_OPERATORS.contains(&binary_operator))
            && (!self.options.is_in_tree_literal()
                || !NOT_IN_TREE_BINARY_OPERATORS.contains(&binary_operator))
            && (!self.options.is_in_for_each()
                || !NOT_IN_FOR_EACH_BINARY_OPERATORS.contains(&binary_operator))
        {
            return Ok((ParseInfixOperator::Binary(binary_operator), 1));
        }

        // assertion operators in value expressions
        if !self.options.is_in_super_type()
            && !self.options.is_in_type()
            && token.token.ty == TokenType::Identifier
            && let Some(keyword) = Keyword::from_str(token_str).ok()
            && matches!(keyword, Keyword::As | Keyword::Satisfies)
            && (self.language.is_destack() || !has_newline)
        {
            let next_token_type = next_token
                .map(|next| next.token.ty)
                .unwrap_or(TokenType::End);
            let next_next_token_type = next_next_token
                .map(|next| next.token.ty)
                .unwrap_or(TokenType::End);

            if self.contextual_cast_keyword_continues_identifier(
                keyword,
                next_token_type,
                next_next_token_type,
            ) {
                return Err(ParseError::unexpected(token.span));
            }

            let operator = match keyword {
                Keyword::As => ParseInfixOperator::As,
                Keyword::Satisfies => ParseInfixOperator::Satisfies,
                _ => unreachable!("checked assertion keyword"),
            };

            return Ok((operator, 1));
        }

        // regular type binary operator
        // (forbidden in super type clauses, avoid newline glue in TS mode)
        if !self.options.is_in_super_type()
            && let Some(type_binary_operator) =
                TypeBinaryOperator::from_token(token_str, token.token.ty)
            && (!self.options.is_in_for_each() || type_binary_operator != TypeBinaryOperator::In)
            && (self.options.is_in_type()
                || (self.language.is_destack()
                    && matches!(
                        type_binary_operator,
                        TypeBinaryOperator::Extends
                            | TypeBinaryOperator::Implements
                            | TypeBinaryOperator::Is
                    )))
            && (self.language.is_destack() || !has_newline)
        {
            return Ok((ParseInfixOperator::TypeBinary(type_binary_operator), 1));
        }

        // regular assign operator
        // (not allowed in static, type, and tree contexts)
        if !self.options.is_in_static()
            && !self.options.is_in_type()
            && !self.options.is_in_tree_literal()
            && let Some(assign_operator) = AssignOperator::from_token(token.token.ty)
        {
            return Ok((ParseInfixOperator::Assign(assign_operator), 1));
        }

        Err(ParseError::unexpected(token.span))
    }

    /// Peek a unary prefix operator.
    #[inline]
    pub fn peek_unary_prefix_operator_maybe(&mut self) -> Option<UnaryOperator> {
        let token = *self.peek().ok()?;
        if token.token.ty == TokenType::Identifier && !self.options.is_in_type() {
            let token_str = self.get_span_str(token.span);
            if let Ok(keyword) = Keyword::from_str(token_str)
                && let Some(operator) = UnaryOperator::from_prefix_keyword(keyword)
            {
                return Some(operator);
            }
        }
        let operator = UnaryOperator::from_prefix_token(token.token.ty)?;

        // dereference (*x) is not valid in JS/TS compatibility mode
        if operator == UnaryOperator::Dereference && !self.language.is_destack() {
            return None;
        }

        // spread (...x) is not a valid standalone expression in JS/TS
        if operator == UnaryOperator::Spread && !self.language.is_destack() {
            return None;
        }

        Some(operator)
    }

    /// Peek a unary prefix operator.
    #[inline]
    pub fn peek_unary_prefix_operator(&mut self) -> ParseResult<UnaryOperator> {
        self.peek_unary_prefix_operator_maybe()
            .ok_or(ParseError::unexpected(self.current_span_or_eof()))
    }

    /// Peek a unary postfix operator.
    #[inline]
    pub fn peek_unary_postfix_operator(&mut self) -> ParseResult<UnaryOperator> {
        let token = *self.peek()?;
        UnaryOperator::from_postfix_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a type unary prefix operator.
    #[inline]
    pub fn peek_type_unary_prefix_operator_maybe(&mut self) -> Option<TypeUnaryOperator> {
        let token = *self.peek().ok()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
    }

    /// Peek a type unary operator.
    #[inline]
    pub fn peek_type_unary_prefix_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        self.peek_type_unary_prefix_operator_maybe()
            .ok_or(ParseError::unexpected(self.current_span_or_eof()))
    }

    /// Peek a type unary postfix operator.
    #[inline]
    pub fn peek_type_unary_postfix_operator_maybe(&mut self) -> Option<TypeUnaryOperator> {
        let token = *self.peek().ok()?;
        if token.token.ty != TokenType::Identifier {
            return None;
        }

        let token_str = self.get_span_str(token.span);
        if token_str != "as" {
            return None;
        }

        // allow `as comptime` across line breaks
        let next_index = self.next_non_newline_index_from(self.pos_index().saturating_add(1));
        if self.token_type_at(next_index) != TokenType::Identifier {
            return None;
        }

        let next_keyword = self.keyword_for_index(next_index);
        match next_keyword {
            Some(Keyword::Comptime) => Some(TypeUnaryOperator::AsComptime),
            _ => None,
        }
    }

    /// Peek a type unary postfix operator.
    #[inline]
    pub fn peek_type_unary_postfix_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        self.peek_type_unary_postfix_operator_maybe()
            .ok_or(ParseError::unexpected(self.current_span_or_eof()))
    }

    /// Peek a next type unary operator.
    #[inline]
    pub fn peek_next_type_unary_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        let token = *self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&mut self) -> ParseResult<AssignOperator> {
        let token = *self.peek()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek next assign operator.
    #[inline]
    pub fn peek_next_assign_operator(&mut self) -> ParseResult<AssignOperator> {
        let token = *self.peek_next()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Return true when the next token is an assignment operator.
    #[inline]
    pub fn peek_next_assign_operator_is(&mut self) -> bool {
        AssignOperator::from_token(self.peek_next_token_type()).is_some()
    }

    /// Return true when a token index could be an infix or assign operator.
    #[inline]
    pub(super) fn has_infix_or_assign_operator_at_index(&mut self, index: usize) -> bool {
        let token_type = self.token_type_at(index);
        if AssignOperator::from_token(token_type).is_some() {
            return true;
        }
        if token_type != TokenType::Identifier {
            return BinaryOperator::from_token("", token_type).is_some();
        }
        // only identifiers mapped to contextual operator keywords can act as infix operators
        let Some(keyword) = self.keyword_for_index(index) else {
            return false;
        };

        // `as` and `satisfies` followed by member access continue as identifiers
        let next_token_type = self.token_type_at(index.saturating_add(1));
        let next_next_token_type = self.token_type_at(index.saturating_add(2));
        if self.contextual_cast_keyword_continues_identifier(
            keyword,
            next_token_type,
            next_next_token_type,
        ) {
            return false;
        }

        matches!(
            keyword,
            Keyword::In
                | Keyword::InstanceOf
                | Keyword::As
                | Keyword::Is
                | Keyword::Satisfies
                | Keyword::Extends
                | Keyword::Implements
        )
    }

    /// Peek an infix operator at a semantic token index.
    #[inline]
    pub(super) fn peek_infix_operator_at_index_maybe(
        &mut self,
        index: usize,
        has_newline: bool,
    ) -> Option<(ParseInfixOperator, u8)> {
        let token = *self.token_ref_at(index)?;
        let token_type = token.token.ty;
        let (next_token, next_next_token) =
            if matches!(token_type, TokenType::GreaterThan | TokenType::Identifier) {
                (self.token_at(index + 1), self.token_at(index + 2))
            } else {
                (None, None)
            };
        let token_str = if token_type == TokenType::Identifier {
            let keyword = self.keyword_for_index(index)?;
            Self::infix_identifier_keyword_text(keyword)?
        } else {
            ""
        };
        self.classify_infix_operator(
            token_str,
            &token,
            next_token.as_ref(),
            next_next_token.as_ref(),
            has_newline,
        )
        .ok()
    }

    /// Make an expression from an infix operator.
    #[inline]
    pub(super) fn make_infix_expression(
        &mut self,
        left: LocalNodeId<Expression>,
        operator: ParseInfixOperator,
        right: LocalNodeId<Expression>,
    ) -> ParseResult<Expression> {
        Ok(match operator {
            ParseInfixOperator::Binary(binary_operator) => {
                if self.options.is_in_type()
                    && matches!(
                        binary_operator,
                        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                    )
                {
                    let left_type = self.expect_type_expression_value(left)?;
                    let right_type = self.expect_type_expression_value(right)?;
                    let type_expression_id = self.append_type_chain_element(
                        left,
                        binary_operator,
                        left_type,
                        right_type,
                    );

                    Expression::Type {
                        value: type_expression_id,
                    }
                } else {
                    Expression::Binary {
                        left,
                        operator: binary_operator,
                        right,
                    }
                }
            }
            ParseInfixOperator::As | ParseInfixOperator::Satisfies => {
                unreachable!("assertion operators are built in continuation parsing")
            }
            ParseInfixOperator::TypeBinary(type_binary_operator) => {
                if self.options.is_in_type()
                    && type_binary_operator == TypeBinaryOperator::Is
                    && let Some(subject) = self.type_predicate_subject_maybe(left)
                {
                    let target = self.expect_type_expression_value(right)?;
                    let predicate_id = self.insert_node(
                        TypeExpression::Predicate {
                            asserts: false,
                            subject,
                            target: Some(target),
                        },
                        self.tree.get_span(left),
                    );

                    return Ok(Expression::Type {
                        value: predicate_id,
                    });
                }
                return Err(ParseError::unexpected(self.tree.get_span(right)));
            }
            ParseInfixOperator::Assign(assign_operator) => Expression::Assign {
                left,
                operator: assign_operator,
                right,
            },
        })
    }

    /// Append one type element to a union or intersection chain.
    fn append_type_chain_element(
        &mut self,
        source_id: LocalNodeId<Expression>,
        operator: BinaryOperator,
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        match (operator, self.tree.get(left).clone()) {
            (BinaryOperator::ElementwiseOr, TypeExpression::Union { mut elements }) => {
                elements.push(right);
                self.insert_wrapped_type_expression(source_id, TypeExpression::Union { elements })
            }
            (BinaryOperator::ElementwiseAnd, TypeExpression::Intersection { mut elements }) => {
                elements.push(right);
                self.insert_wrapped_type_expression(
                    source_id,
                    TypeExpression::Intersection { elements },
                )
            }
            (BinaryOperator::ElementwiseOr, _) => self.insert_wrapped_type_expression(
                source_id,
                TypeExpression::Union {
                    elements: vec![left, right],
                },
            ),
            (BinaryOperator::ElementwiseAnd, _) => self.insert_wrapped_type_expression(
                source_id,
                TypeExpression::Intersection {
                    elements: vec![left, right],
                },
            ),
            _ => unreachable!(),
        }
    }
}
