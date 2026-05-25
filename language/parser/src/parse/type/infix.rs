use crate::parse::scope::TypeScope;
use crate::parse::r#type::operator::{TypeBinaryOperator, TypeInfixOperator};
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

use destack_dir::{
    BinaryOperator, Keyword, LocalNodeId, NodeType, OperatorPrecedence, RangeEnd, TokenType,
    TypeExpression,
};
use destack_source::Span;

/// One pending conditional type node.
struct PendingTypeConditional {
    /// The conditional source start.
    start: ParserSpanStart,
    /// The checked type.
    left: LocalNodeId<TypeExpression>,
    /// The required type.
    extends_type: LocalNodeId<TypeExpression>,
    /// The type selected when the condition holds.
    then_type: LocalNodeId<TypeExpression>,
    /// The `extends` operator span.
    operator_span: Span,
}

/// A parsed false branch of a conditional type.
enum TypeConditionalElse {
    /// A complete false branch expression.
    Expression(LocalNodeId<TypeExpression>),
    /// A nested conditional type head.
    Conditional {
        /// The nested conditional source start.
        start: ParserSpanStart,
        /// The nested checked type.
        left: LocalNodeId<TypeExpression>,
        /// The nested `extends` operator span.
        operator_span: Span,
    },
}

impl Parser {
    /// Eat type infix operators.
    ///
    /// Examples:
    /// ```ds
    /// string | number
    /// A & B
    /// T extends U ? X : Y
    /// ```
    pub(super) fn eat_type_infix(
        &mut self,
        start: &ParserSpanStart,
        scope: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let type_expression_id = self.eat_type_prefix_or_primary(start)?;
        let type_expression_id = self.eat_type_postfix(start, type_expression_id, scope)?;

        self.eat_type_infix_rest(start, type_expression_id, scope)
    }

    /// Eat type infix operators after an already parsed left type.
    ///
    /// Examples:
    /// ```ds
    /// | number
    /// & Other
    /// extends Base ? Yes : No
    /// ```
    pub(super) fn eat_type_infix_rest(
        &mut self,
        start: &ParserSpanStart,
        mut left: LocalNodeId<TypeExpression>,
        scope: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        loop {
            // stop at grammar owned by the caller
            if self.type_infix_belongs_to_outer_scope(scope) {
                break;
            }

            // select one operator without consuming it
            let Some(operator) = self.peek_type_infix_operator_maybe() else {
                break;
            };

            // stop before operators that belong to another grammar layer
            if self.type_infix_stops(operator, scope) {
                break;
            }

            // parse right operand scope
            let operator_span = self.eat_type_infix_operator_span();
            let right = self.type_infix_right(operator);

            // fold operator into the left expression
            left =
                self.eat_type_infix_operator(start, left, operator, operator_span, right, scope)?;

            if operator != TypeInfixOperator::Is {
                self.tree.set_main_span(left, operator_span);
            }
        }

        Ok(left)
    }

    /// Return the current type infix operator.
    #[inline]
    fn peek_type_infix_operator_maybe(&mut self) -> Option<TypeInfixOperator> {
        match self.peek_token_type() {
            TokenType::ElementwiseOr => {
                Some(TypeInfixOperator::Binary(BinaryOperator::ElementwiseOr))
            }
            TokenType::ElementwiseAnd => {
                Some(TypeInfixOperator::Binary(BinaryOperator::ElementwiseAnd))
            }
            TokenType::Range if self.language.is_destack() => {
                Some(TypeInfixOperator::Range(RangeEnd::Open))
            }
            TokenType::RangeInclusive if self.language.is_destack() => {
                Some(TypeInfixOperator::Range(RangeEnd::Inclusive))
            }
            TokenType::Identifier => self.type_infix_operator_from_keyword(),
            _ => None,
        }
    }

    /// Return the current keyword type infix operator.
    #[inline]
    fn type_infix_operator_from_keyword(&mut self) -> Option<TypeInfixOperator> {
        match self.current_keyword()? {
            Keyword::Is => Some(TypeInfixOperator::Is),
            Keyword::Extends => Some(TypeInfixOperator::Relation(TypeBinaryOperator::Extends)),
            Keyword::Implements => {
                Some(TypeInfixOperator::Relation(TypeBinaryOperator::Implements))
            }
            _ => None,
        }
    }

    /// Return the right operand scope for one type infix operator.
    fn type_infix_right(&self, operator: TypeInfixOperator) -> TypeScope {
        let flags = self.type_nested_flags();

        if operator == TypeInfixOperator::Is {
            return TypeScope::from_flags(flags);
        }

        TypeScope::from_flags(flags).at_precedence(Some(operator.precedence()))
    }

    /// Return whether the current token is an outer type boundary.
    fn type_infix_belongs_to_outer_scope(&mut self, scope: TypeScope) -> bool {
        if self.peek_is(TokenType::Maybe) {
            return true;
        }

        if scope.owns_colon_boundary && self.peek_is(TokenType::Colon) {
            return true;
        }

        scope.is_static && Self::starts_type_angle_close(self.peek_token_type())
    }

    /// Return whether one type infix operator belongs to an outer parser.
    fn type_infix_stops(&mut self, operator: TypeInfixOperator, scope: TypeScope) -> bool {
        // newline sensitive operators
        if self.current_token_is_on_new_line()
            && matches!(
                operator,
                TypeInfixOperator::Range(_) | TypeInfixOperator::Is
            )
        {
            return true;
        }

        // heritage boundary
        if (self.language.is_typescript() || scope.stops_before_implements)
            && operator == TypeInfixOperator::Relation(TypeBinaryOperator::Implements)
        {
            return true;
        }

        // precedence boundary
        if scope.stops_before(operator) {
            return true;
        }

        // conditional type boundary
        if scope.disallows_conditional
            && operator == TypeInfixOperator::Relation(TypeBinaryOperator::Extends)
        {
            return true;
        }

        false
    }

    /// Eat the current type infix operator span.
    ///
    /// Examples:
    /// ```ds
    /// |
    /// extends
    /// ..=
    /// ```
    fn eat_type_infix_operator_span(&mut self) -> Span {
        let operator_start = self.span_start();
        self.bump();

        self.get_span_from(&operator_start)
    }

    /// Eat one type infix operator after the operator token was consumed.
    ///
    /// Examples:
    /// ```ds
    /// A | B
    /// T extends U ? X : Y
    /// 1..10
    /// ```
    fn eat_type_infix_operator(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<TypeExpression>,
        operator: TypeInfixOperator,
        operator_span: Span,
        right: TypeScope,
        scope: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // conditional type
        if operator == TypeInfixOperator::Relation(TypeBinaryOperator::Extends) {
            return self.eat_type_conditional_rest(start, left, operator_span);
        }

        // range
        if let TypeInfixOperator::Range(end_kind) = operator {
            return self.eat_type_range_rest(start, left, end_kind, right);
        }

        // predicate validity
        if operator == TypeInfixOperator::Is
            && !(scope.allows_type_predicate || self.type_expression_is_bare_this(left))
        {
            return Err(ParserError::unexpected(operator_span));
        }

        // ordinary type infix expression
        let right = self.eat_type_infix_right(right)?;
        let type_id = self.make_type_infix_expression(
            self.get_span_from(start),
            self.type_expression_head_span(left),
            left,
            operator,
            operator_span,
            right,
        )?;

        Ok(type_id)
    }

    /// Eat one type infix right operand.
    ///
    /// Examples:
    /// ```ds
    /// number
    /// keyof T
    /// { id: string }
    /// ```
    fn eat_type_infix_right(
        &mut self,
        right: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        if Self::is_type_expression_boundary_token(self.peek_token_type()) {
            return Ok(self.recover_missing_type_expression_here(NodeType::TypeExpression));
        }

        self.eat_type_operand(right)
    }

    /// Eat a type range after the range operator.
    ///
    /// Examples:
    /// ```ds
    /// 1..10
    /// 1..
    /// 1..=10
    /// ```
    fn eat_type_range_rest(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<TypeExpression>,
        end_kind: RangeEnd,
        right: TypeScope,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let end = self.eat_type_range_end(end_kind, right)?;

        Ok(self.insert_node(
            TypeExpression::Range {
                start: Some(left),
                end,
                end_kind,
            },
            self.get_span_from(start),
        ))
    }

    /// Eat a type conditional after `extends`.
    ///
    /// Examples:
    /// ```ds
    /// T extends U ? X : Y
    /// T extends U
    /// T extends keyof U ? A : B
    /// ```
    pub(crate) fn eat_type_conditional_rest(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<TypeExpression>,
        operator_span: Span,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut pending = Vec::new();
        let mut start = *start;
        let mut left = left;
        let mut operator_span = operator_span;

        loop {
            // parse relation target
            let extends_type = self.eat_type_extends_operand()?;

            // finish a plain extends expression
            if !self.peek_is(TokenType::Maybe) {
                let else_type = self.finish_type_extends_expression(&start, left, extends_type)?;

                return Ok(self.finish_pending_type_conditionals(pending, else_type));
            }

            // parse conditional branches
            self.bump();
            let then_type = self.eat_type_conditional_then()?;
            self.eat_colon()?;

            // queue outer conditional until the final false branch is known
            pending.push(PendingTypeConditional {
                start,
                left,
                extends_type,
                then_type,
                operator_span,
            });

            // continue through nested false branch conditionals
            match self.eat_type_conditional_else()? {
                TypeConditionalElse::Expression(else_type) => {
                    return Ok(self.finish_pending_type_conditionals(pending, else_type));
                }
                TypeConditionalElse::Conditional {
                    start: else_start,
                    left: else_left,
                    operator_span: else_operator_span,
                } => {
                    start = else_start;
                    left = else_left;
                    operator_span = else_operator_span;
                }
            }
        }
    }

    /// Eat the right operand of an `extends` type relation.
    ///
    /// Examples:
    /// ```ds
    /// U
    /// keyof U
    /// { id: string }
    /// ```
    fn eat_type_extends_operand(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        let flags = self
            .flags
            .not_in_position()
            .in_type()
            .disallow_type_conditional();
        let scope = TypeScope::from_flags(flags);

        if Self::is_type_expression_boundary_token(self.peek_token_type()) {
            return Ok(self.recover_missing_type_expression_here(NodeType::TypeExpression));
        }

        self.eat_type_operand(scope)
    }

    /// Finish a non-conditional `extends` type expression.
    fn finish_type_extends_expression(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<TypeExpression>,
        extends_type: LocalNodeId<TypeExpression>,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        if self.language.is_typescript() {
            return Err(ParserError::expected(
                self.anchor_span_here(),
                TokenType::Maybe,
            ));
        }

        Ok(self.insert_node(
            TypeExpression::Extends {
                left,
                right: extends_type,
            },
            self.get_span_from(start),
        ))
    }

    /// Eat the true branch of a conditional type.
    ///
    /// Examples:
    /// ```ds
    /// X
    /// readonly X
    /// X | Y
    /// ```
    fn eat_type_conditional_then(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        let flags = self
            .flags
            .not_in_position()
            .in_type()
            .in_type_conditional_right();

        self.eat_type_expression_or_recover_missing(flags, NodeType::TypeExpression)
    }

    /// Eat the false branch of a conditional type.
    ///
    /// Examples:
    /// ```ds
    /// Y
    /// never
    /// X extends Y ? A : B
    /// ```
    fn eat_type_conditional_else(&mut self) -> ParserResult<TypeConditionalElse> {
        // recover empty branch
        if self.is_type_expression_boundary() {
            let else_type = self.recover_missing_type_expression_here(NodeType::TypeExpression);

            return Ok(TypeConditionalElse::Expression(else_type));
        }

        // parse up to a possible nested conditional
        let flags = self.flags.not_in_position().in_type();
        let scope = TypeScope::from_flags(flags);
        let head_scope = scope.at_precedence(Some(OperatorPrecedence::Comparison as u16));
        let start = self.span_start();
        let left = self.with_flags(head_scope.flags, |parser| {
            parser.eat_type_expression_body(head_scope)
        })?;
        let operator = TypeInfixOperator::Relation(TypeBinaryOperator::Extends);

        // finish when the outer grammar owns the operator
        if self.type_infix_belongs_to_outer_scope(scope) || self.type_infix_stops(operator, scope) {
            let else_type = self.eat_type_infix_rest(&start, left, scope)?;

            return Ok(TypeConditionalElse::Expression(else_type));
        }

        // finish when no nested conditional follows
        if self.peek_type_infix_operator_maybe() != Some(operator) {
            let else_type = self.eat_type_infix_rest(&start, left, scope)?;

            return Ok(TypeConditionalElse::Expression(else_type));
        }

        // continue through the nested conditional
        let operator_span = self.eat_type_infix_operator_span();

        Ok(TypeConditionalElse::Conditional {
            start,
            left,
            operator_span,
        })
    }

    /// Finish pending right-associative conditional type nodes.
    fn finish_pending_type_conditionals(
        &mut self,
        pending: Vec<PendingTypeConditional>,
        mut else_type: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        for frame in pending.into_iter().rev() {
            let type_id = self.insert_node(
                TypeExpression::Conditional {
                    left: frame.left,
                    extends_type: frame.extends_type,
                    then_type: frame.then_type,
                    else_type,
                },
                self.get_span_from(&frame.start),
            );
            self.tree.set_main_span(type_id, frame.operator_span);
            else_type = type_id;
        }

        else_type
    }

    /// Parse a startless type range.
    ///
    /// Examples:
    /// ```ds
    /// ..10
    /// ..
    /// ..=10
    /// ```
    pub(super) fn eat_type_startless_range(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // operator
        let end_kind = if self.peek_is(TokenType::RangeInclusive) {
            RangeEnd::Inclusive
        } else {
            RangeEnd::Open
        };
        self.bump();

        // end
        let right = TypeScope::from_flags(self.type_nested_flags())
            .at_precedence(Some(OperatorPrecedence::Range as u16));
        let end = self.eat_type_range_end(end_kind, right)?;

        Ok(self.insert_node(
            TypeExpression::Range {
                start: None,
                end,
                end_kind,
            },
            self.get_span_from(start),
        ))
    }

    /// Eat the optional end of a type range.
    ///
    /// Examples:
    /// ```ds
    /// 10
    /// Length
    /// N + 1
    /// ```
    fn eat_type_range_end(
        &mut self,
        end_kind: RangeEnd,
        right: TypeScope,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if self.current_token_is_on_new_line() || self.is_type_expression_boundary() {
            if end_kind == RangeEnd::Inclusive {
                return Ok(Some(
                    self.recover_missing_type_expression_here(NodeType::TypeExpression),
                ));
            }

            return Ok(None);
        }

        let end = self.eat_type_operand(right)?;

        Ok(Some(end))
    }

    /// Parse Destack reference operators in type space.
    ///
    /// Examples:
    /// ```ds
    /// &mut T
    /// &shared T
    /// ^local T
    /// ```
    pub(super) fn eat_type_reference_operator(
        &mut self,
        start: &ParserSpanStart,
        token_type: TokenType,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // operator
        let operator_start = self.span_start();
        self.bump();
        let operator_span = self.get_span_from(&operator_start);

        // modifiers
        let mutability = self.eat_reference_mutability_maybe()?;
        let variance = self.eat_variance_bound_if_present()?;

        // target
        let target = TypeScope::from_flags(self.type_nested_flags())
            .at_precedence(Some(OperatorPrecedence::Prefix as u16));
        let target_type = self.with_recursive_descent(NodeType::TypeExpression, |parser| {
            parser.eat_type_infix_right(target)
        })?;

        // node
        let expression = if token_type == TokenType::ElementwiseAnd {
            TypeExpression::BorrowedOf {
                mutability,
                variance,
                target_type,
            }
        } else {
            TypeExpression::OwnedOf {
                mutability,
                variance,
                target_type,
            }
        };

        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, operator_span);

        Ok(expression_id)
    }
}
