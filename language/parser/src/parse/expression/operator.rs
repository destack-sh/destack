use std::str::FromStr;

use crate::{ParseError, ParseResult, Parser};

use super::common::{
    NOT_IN_FOR_EACH_BINARY_OPERATORS, NOT_IN_GENERIC_ARGUMENT_BINARY_OPERATORS,
    NOT_IN_TREE_BINARY_OPERATORS,
};

use destack_ast::{
    AssignOperator, AssignPattern, BinaryOperator, Expression, Keyword, LocalNodeId,
    OperatorPrecedence, TokenSpan, TokenType, TypeExpression, TypePredicateSubject, UnaryOperator,
};
use destack_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Span};

const AS_ASSERTION_PRECEDENCE: u16 = 1355;
const SATISFIES_ASSERTION_PRECEDENCE: u16 = 1003;
const TYPE_UNARY_PRECEDENCE: u16 = 1800;
const IS_PREDICATE_PRECEDENCE: u16 = 1225;

/// One parser-local type unary operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TypeUnaryOperator {
    /// `typeof value`
    Typeof,
    /// `keyof T`
    Keyof,
    /// `readonly T`
    Readonly,
    /// `shared T`
    Shared,
    /// `T as comptime`
    AsComptime,
}

impl TypeUnaryOperator {
    /// Return the parser precedence for one type unary operator.
    #[inline]
    pub(crate) fn precedence(self) -> u16 {
        match self {
            TypeUnaryOperator::Typeof => TYPE_UNARY_PRECEDENCE + 4,
            TypeUnaryOperator::Keyof => TYPE_UNARY_PRECEDENCE + 3,
            TypeUnaryOperator::Readonly => TYPE_UNARY_PRECEDENCE + 3,
            TypeUnaryOperator::Shared => TYPE_UNARY_PRECEDENCE + 3,
            TypeUnaryOperator::AsComptime => TYPE_UNARY_PRECEDENCE + 2,
        }
    }

    /// Return one prefix operator from identifier text.
    #[inline]
    pub(crate) fn from_prefix_token(token_str: &str, _token_type: TokenType) -> Option<Self> {
        match token_str {
            "typeof" => Some(TypeUnaryOperator::Typeof),
            "keyof" => Some(TypeUnaryOperator::Keyof),
            "readonly" => Some(TypeUnaryOperator::Readonly),
            "shared" => Some(TypeUnaryOperator::Shared),
            _ => None,
        }
    }
}

/// One parser-local type binary operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TypeBinaryOperator {
    /// `in`
    In,
    /// `extends`
    Extends,
    /// `implements`
    Implements,
}

impl TypeBinaryOperator {
    /// Return the parser precedence for one type binary operator.
    #[inline]
    pub(crate) fn precedence(self) -> u16 {
        match self {
            TypeBinaryOperator::In => 1006,
            TypeBinaryOperator::Extends => 1002,
            TypeBinaryOperator::Implements => 1001,
        }
    }

    /// Return one type binary operator from identifier text.
    #[inline]
    pub(crate) fn from_token(token_str: &str, _token_type: TokenType) -> Option<Self> {
        match token_str {
            "in" => Some(TypeBinaryOperator::In),
            "extends" => Some(TypeBinaryOperator::Extends),
            "implements" => Some(TypeBinaryOperator::Implements),
            _ => None,
        }
    }
}

/// One parser-local infix continuation operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(super) enum ParseInfixOperator {
    /// A binary operator continuation.
    Binary(BinaryOperator),
    /// One `is` guard continuation.
    Is,
    /// One `instanceof` guard continuation.
    InstanceOf,
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
            ParseInfixOperator::Is => IS_PREDICATE_PRECEDENCE,
            ParseInfixOperator::InstanceOf => OperatorPrecedence::Comparison as u16,
            ParseInfixOperator::As => AS_ASSERTION_PRECEDENCE,
            ParseInfixOperator::Satisfies => SATISFIES_ASSERTION_PRECEDENCE,
            ParseInfixOperator::TypeBinary(type_binary_operator) => {
                type_binary_operator.precedence()
            }
            ParseInfixOperator::Assign(assign_operator) => assign_operator.precedence(),
        }
    }

    /// Return whether one infix continuation operator binds right associatively.
    #[inline]
    pub(super) fn is_right_associative(self) -> bool {
        matches!(self, ParseInfixOperator::Assign(_))
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

        // member access keeps the keyword as an identifier
        let has_member_after_keyword = next_token_type == TokenType::Dot;

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
            Keyword::InstanceOf => Some("instanceof"),
            Keyword::As => Some("as"),
            Keyword::Extends => Some("extends"),
            Keyword::Implements => Some("implements"),
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
        // identifier keyword
        let identifier_keyword = if token.token.ty == TokenType::Identifier {
            Keyword::from_str(token_str).ok()
        } else {
            None
        };

        // regular binary operator
        // (only a subset of binary operators are allowed in generic argument and tree contexts)
        if let Some(binary_operator) = BinaryOperator::from_token(token_str, token.token.ty)
            && (!self.flags.is_in_type()
                || self.flags.is_in_static()
                || matches!(
                    binary_operator,
                    BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                ))
            && (!self.flags.is_in_static()
                || !NOT_IN_GENERIC_ARGUMENT_BINARY_OPERATORS.contains(&binary_operator))
            && (!self.flags.is_in_tree_literal()
                || !NOT_IN_TREE_BINARY_OPERATORS.contains(&binary_operator))
            && (!self.flags.is_in_for_each()
                || !NOT_IN_FOR_EACH_BINARY_OPERATORS.contains(&binary_operator))
        {
            return Ok((ParseInfixOperator::Binary(binary_operator), 1));
        }

        // type predicate and runtime `is` guard
        if !self.flags.is_in_super_type()
            && identifier_keyword == Some(Keyword::Is)
            && (self.flags.is_in_type() || self.language.is_destack())
            && !has_newline
        {
            return Ok((ParseInfixOperator::Is, 1));
        }

        // assertion operators in value expressions
        if !self.flags.is_in_super_type()
            && !self.flags.is_in_type()
            && let Some(keyword) = identifier_keyword
            && matches!(keyword, Keyword::As | Keyword::Satisfies)
            && !has_newline
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

        // runtime `instanceof` guard
        if !self.flags.is_in_super_type()
            && !self.flags.is_in_type()
            && let Some(keyword) = identifier_keyword
        {
            // `x instanceof C`
            if keyword == Keyword::InstanceOf {
                return Ok((ParseInfixOperator::InstanceOf, 1));
            }
        }

        // regular type binary operator
        // forbidden in super type clauses, and semicolon statement forms avoid newline glue here
        if !self.flags.is_in_super_type()
            && let Some(type_binary_operator) =
                TypeBinaryOperator::from_token(token_str, token.token.ty)
            && (!self.flags.is_in_for_each() || type_binary_operator != TypeBinaryOperator::In)
            && (self.language.is_destack()
                || self.flags.is_in_type() && type_binary_operator == TypeBinaryOperator::Extends)
            && !has_newline
        {
            return Ok((ParseInfixOperator::TypeBinary(type_binary_operator), 1));
        }

        // regular assign operator
        // (not allowed in static, type, and tree contexts)
        if !self.flags.is_in_static()
            && !self.flags.is_in_type()
            && !self.flags.is_in_tree_literal()
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
        if token.token.ty == TokenType::Identifier && !self.flags.is_in_type() {
            let token_str = self.get_span_str(token.span);
            if let Ok(keyword) = Keyword::from_str(token_str)
                && let Some(operator) = UnaryOperator::from_prefix_keyword(keyword)
            {
                return Some(operator);
            }
        }
        let operator = UnaryOperator::from_prefix_token(token.token.ty)?;

        // dereference (*x) is not valid in semicolon statement value mode
        if operator == UnaryOperator::Dereference && !self.language.is_destack() {
            return None;
        }

        // spread (...x) is not a valid standalone expression in semicolon statement value mode
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
    pub(crate) fn peek_type_unary_prefix_operator_maybe(&mut self) -> Option<TypeUnaryOperator> {
        let token = *self.peek().ok()?;
        let token_str = self.get_span_str(token.span);
        let operator = TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)?;
        if operator == TypeUnaryOperator::Shared && !self.language.is_destack() {
            return None;
        }

        Some(operator)
    }

    /// Peek a type unary postfix operator.
    #[inline]
    pub(crate) fn peek_type_unary_postfix_operator_maybe(&mut self) -> Option<TypeUnaryOperator> {
        let token = *self.peek().ok()?;
        if token.token.ty != TokenType::Identifier {
            return None;
        }

        let token_str = self.get_span_str(token.span);
        if token_str != "as" {
            return None;
        }

        // allow `as comptime` across line breaks
        let next_token = self.next_token();
        if next_token.token.ty != TokenType::Identifier {
            return None;
        }

        let next_keyword = self.next_keyword();
        match next_keyword {
            Some(Keyword::Comptime) => Some(TypeUnaryOperator::AsComptime),
            _ => None,
        }
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
        let token = self.next_token();
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Return true when the next token is an assignment operator.
    #[inline]
    pub fn peek_next_assign_operator_is(&mut self) -> bool {
        AssignOperator::from_token(self.lookahead(|parser| {
            parser.bump();
            parser.peek_token_type()
        }))
        .is_some()
    }

    /// Return true when the current token could be an infix or assign operator.
    #[inline]
    pub(crate) fn current_token_can_start_infix_or_assign_operator(&mut self) -> bool {
        let token_type = self.peek_token_type();
        if AssignOperator::from_token(token_type).is_some() {
            return true;
        }
        if token_type != TokenType::Identifier {
            return BinaryOperator::from_token("", token_type).is_some();
        }

        // only identifiers mapped to contextual operator keywords can act as infix operators
        let Some(keyword) = self.current_keyword() else {
            return false;
        };

        // `as` and `satisfies` followed by member access continue as identifiers
        let next_token_type = self.next_token_type();
        let next_next_token_type = self.lookahead(|parser| {
            parser.bump();
            parser.bump();
            parser.peek_token_type()
        });
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

    /// Peek an infix operator at the current parser token.
    #[inline]
    pub(super) fn peek_infix_operator_maybe(
        &mut self,
        has_newline: bool,
    ) -> Option<(ParseInfixOperator, u8)> {
        let token = self.current_token();
        let token_type = token.token.ty;
        let (next_token, next_next_token) =
            if matches!(token_type, TokenType::GreaterThan | TokenType::Identifier) {
                let next_token = self.next_token();
                let next_next_token = self.lookahead(|parser| {
                    parser.bump();
                    parser.bump();
                    parser.current_token()
                });

                (Some(next_token), Some(next_next_token))
            } else {
                (None, None)
            };
        let token_str = if token_type == TokenType::Identifier {
            let keyword = self.current_keyword()?;
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

    /// Make one value-space expression from one non-assertion infix operator.
    #[inline]
    pub(super) fn make_value_infix_expression(
        &mut self,
        left: LocalNodeId<Expression>,
        operator: ParseInfixOperator,
        right: LocalNodeId<Expression>,
    ) -> ParseResult<Expression> {
        Ok(match operator {
            ParseInfixOperator::Binary(binary_operator) => Expression::Binary {
                left,
                operator: binary_operator,
                right,
            },
            ParseInfixOperator::Is => {
                return Err(ParseError::unexpected(self.tree.get_span(right)));
            }
            ParseInfixOperator::InstanceOf => Expression::InstanceOf {
                value: left,
                target: right,
            },
            ParseInfixOperator::As | ParseInfixOperator::Satisfies => {
                unreachable!("assertion operators are built in continuation parsing")
            }
            ParseInfixOperator::TypeBinary(_type_binary_operator) => {
                return Err(ParseError::unexpected(self.tree.get_span(right)));
            }
            ParseInfixOperator::Assign(assign_operator) => {
                let left = self.expression_to_assign_pattern(left)?;

                // only plain `=` may target destructuring patterns
                if assign_operator != AssignOperator::Assign
                    && !matches!(self.tree.get(left), AssignPattern::Expression { .. })
                {
                    return Err(ParseError::unexpected(self.tree.get_span(left)));
                }

                Expression::Assign {
                    left,
                    operator: assign_operator,
                    right,
                }
            }
        })
    }

    /// Make one type-space expression from an infix operator.
    #[inline]
    pub(super) fn make_type_infix_expression(
        &mut self,
        source_span: Span,
        head_span: Span,
        left_type_id: LocalNodeId<TypeExpression>,
        operator: ParseInfixOperator,
        operator_span: Span,
        right_type_id: LocalNodeId<TypeExpression>,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        match operator {
            // `A | B` and `A & B`
            ParseInfixOperator::Binary(binary_operator)
                if matches!(
                    binary_operator,
                    BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                ) =>
            {
                let type_expression_id = self.append_type_chain_element(
                    head_span,
                    binary_operator,
                    operator_span,
                    left_type_id,
                    right_type_id,
                );

                Ok(type_expression_id)
            }

            // `value is T`
            ParseInfixOperator::Is => {
                let Some(subject) = self.type_predicate_subject_from_type_expression(left_type_id)
                else {
                    return Err(ParseError::unexpected(self.tree.get_span(left_type_id)));
                };

                self.set_node_leading_span(right_type_id, operator_span.end);

                let predicate_id = self.insert_node(
                    TypeExpression::Predicate {
                        asserts: false,
                        subject,
                        target: Some(right_type_id),
                    },
                    source_span,
                );
                self.tree.set_head_span(predicate_id, head_span);
                self.tree.set_side_span(
                    predicate_id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    operator_span,
                );

                Ok(predicate_id)
            }

            // `K in T`, `T extends U`, `T implements U`
            ParseInfixOperator::TypeBinary(type_binary_operator) => {
                let expression = match type_binary_operator {
                    TypeBinaryOperator::In => TypeExpression::In {
                        left: left_type_id,
                        right: right_type_id,
                    },
                    TypeBinaryOperator::Extends => TypeExpression::Extends {
                        left: left_type_id,
                        right: right_type_id,
                    },
                    TypeBinaryOperator::Implements => TypeExpression::Implements {
                        left: left_type_id,
                        right: right_type_id,
                    },
                };
                let expression_id = self.insert_node(expression, source_span);
                self.tree.set_head_span(expression_id, head_span);

                Ok(expression_id)
            }

            // everything else is value-only here
            _ => Err(ParseError::unexpected(self.tree.get_span(right_type_id))),
        }
    }

    /// Append one type element to a union or intersection chain.
    fn append_type_chain_element(
        &mut self,
        head_span: Span,
        operator: BinaryOperator,
        operator_span: Span,
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        // `A | B`, source span excludes leading separators owned by side spans
        let left_span = self.tree.get_span(left);
        let right_span = self.tree.get_span(right);
        let source_span = Span::new(left_span.file, left_span.start, right_span.end);
        let leading_span = self
            .tree
            .get_side_span(left, NodeSpanType::Boundary(NodeSpanBoundary::Leading));
        let leading_operator_span = self.tree.get_side_span(
            left,
            NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
        );

        // `A | B`, arms own trivia up to and after the operator
        self.set_node_trailing_span(left, operator_span.start);
        self.set_node_leading_span(right, operator_span.end);

        // `A | B | C`, reuse the existing chain container
        let expression = match (operator, self.tree.get(left).clone()) {
            (BinaryOperator::ElementwiseOr, TypeExpression::Union { mut elements }) => {
                elements.push(right);
                TypeExpression::Union { elements }
            }
            (BinaryOperator::ElementwiseAnd, TypeExpression::Intersection { mut elements }) => {
                elements.push(right);
                TypeExpression::Intersection { elements }
            }
            (BinaryOperator::ElementwiseOr, _) => TypeExpression::Union {
                elements: vec![left, right],
            },
            (BinaryOperator::ElementwiseAnd, _) => TypeExpression::Intersection {
                elements: vec![left, right],
            },
            _ => unreachable!(),
        };

        // `| A | B`, keep the explicit leading separator on the chain root
        let expression_id = self.insert_node(expression, source_span);
        self.tree.set_head_span(expression_id, head_span);
        if let Some(leading_span) = leading_span {
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Boundary(NodeSpanBoundary::Leading),
                leading_span,
            );
        }
        if let Some(leading_operator_span) = leading_operator_span {
            self.tree.set_side_span(
                expression_id,
                NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
                leading_operator_span,
            );
        }

        expression_id
    }

    /// Return one type predicate subject from one type expression.
    pub(crate) fn type_predicate_subject_from_type_expression(
        &self,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> Option<TypePredicateSubject> {
        match self.tree.get(expression_id) {
            TypeExpression::Reference {
                path,
                generic_arguments,
            } if generic_arguments.is_empty() && path.segments.len() == 1 => {
                Some(TypePredicateSubject::Identifier(path.segments[0]))
            }
            TypeExpression::This => Some(TypePredicateSubject::This),
            _ => None,
        }
    }

    /// Return whether one type expression is a bare `this`.
    pub(crate) fn type_expression_is_bare_this(
        &self,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        matches!(self.tree.get(expression_id), TypeExpression::This)
    }
}
