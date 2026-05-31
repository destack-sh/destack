use crate::parse::expression::operator::ExpressionInfixOperator;
use crate::parse::scope::{CONDITIONAL_PRECEDENCE, ExpressionScope};
use crate::parse::r#type::operator::{TypeBinaryOperator, TypeInfixOperator};
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};
use destack_dir::{
    BinaryOperator, Expression, IfCondition, IfForm, Keyword, LocalNodeId, NodeType,
    OperatorPrecedence, RangeEnd, TokenType, TypeExpression,
};
use destack_source::Span;

/// One value infix continuation with any already parsed type-space left side.
enum ValueInfixOperator {
    /// A value binary operator.
    Binary(BinaryOperator),
    /// A runtime type predicate.
    Is,
    /// A runtime constructor predicate.
    InstanceOf,
    /// A TypeScript `as` assertion.
    As,
    /// A TypeScript `satisfies` assertion.
    Satisfies,
    /// A Destack range operator.
    Range(RangeEnd),
    /// A type-space operator parsed from value position.
    Type {
        /// The parsed type binary operator.
        operator: TypeBinaryOperator,
        /// The left type expression.
        left_type: LocalNodeId<TypeExpression>,
    },
}

/// One pending ternary expression node.
struct PendingConditionalExpression {
    /// The conditional source start.
    start: ParserSpanStart,
    /// The condition expression.
    condition: LocalNodeId<Expression>,
    /// The expression selected when the condition holds.
    then_expression: LocalNodeId<Expression>,
}

/// A parsed false branch of a ternary expression.
enum ConditionalElseExpression {
    /// A complete false branch expression.
    Expression(LocalNodeId<Expression>),
    /// A nested ternary condition.
    Conditional {
        /// The nested conditional source start.
        start: ParserSpanStart,
        /// The nested condition expression.
        condition: LocalNodeId<Expression>,
    },
}

impl ValueInfixOperator {
    /// Return this operator precedence.
    fn precedence(&self) -> u16 {
        match self {
            Self::Binary(operator) => operator.precedence(),
            Self::Type { operator, .. } => operator.precedence(),
            Self::Is | Self::InstanceOf | Self::As | Self::Satisfies => {
                OperatorPrecedence::Comparison as u16
            }
            Self::Range(_) => OperatorPrecedence::Range as u16,
        }
    }
}

impl Parser {
    /// Eat binary operators with Pratt binding.
    ///
    /// Examples:
    /// ```ds
    /// left + right * other
    /// value as const
    /// start..end
    /// ```
    pub(in crate::parse::expression) fn eat_binary(
        &mut self,
        start: &ParserSpanStart,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let (left, is_parenthesized) = self.eat_value_prefix_or_primary(start)?;
        let (left, _) = self.eat_postfix(start, left, is_parenthesized, scope)?;

        self.eat_binary_rest(start, left, scope)
    }

    /// Eat binary operators after an already parsed left value.
    ///
    /// Examples:
    /// ```ds
    /// + right
    /// as Type
    /// ..end
    /// ```
    pub(in crate::parse::expression) fn eat_binary_rest(
        &mut self,
        start: &ParserSpanStart,
        mut left: LocalNodeId<Expression>,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // stop complete declarations before infix continuation
        if matches!(self.tree.get(left), Expression::Declaration(_)) {
            return Ok(left);
        }

        loop {
            // snapshot current token state once per operator
            let token_type = self.peek_token_type();
            let is_on_new_line = self.current_token_is_on_new_line();

            // select the next infix continuation
            let Some(operator) =
                self.current_value_infix_operator(left, scope, token_type, is_on_new_line)?
            else {
                break;
            };

            // parse the right side and fold the operator
            let operator_span = self.eat_value_infix_operator_span();
            let right_scope = ExpressionScope::from_flags(self.flags.not_in_position())
                .at_precedence(Some(operator.precedence()));
            left =
                self.eat_value_infix_expression(start, left, operator, operator_span, right_scope)?;
        }

        Ok(left)
    }

    /// Return the current value infix operator when the current grammar owns it.
    ///
    /// Examples:
    /// ```ds
    /// + right
    /// as Type
    /// extends Type
    /// ```
    fn current_value_infix_operator(
        &mut self,
        left: LocalNodeId<Expression>,
        scope: ExpressionScope,
        token_type: TokenType,
        is_on_new_line: bool,
    ) -> ParserResult<Option<ValueInfixOperator>> {
        if self.value_infix_is_boundary(left, scope, token_type, is_on_new_line) {
            return Ok(None);
        }

        let Some(operator) = self.infix_operator_from_current_token_type(token_type) else {
            return Ok(None);
        };

        if is_on_new_line && matches!(operator, ExpressionInfixOperator::Range(_)) {
            return Ok(None);
        }

        if is_on_new_line && self.can_start_tree_literal() {
            if scope.is_statement_position {
                return Ok(None);
            }

            return Err(ParserError::unexpected(self.peek()?.span));
        }

        if is_on_new_line
            && matches!(
                operator,
                ExpressionInfixOperator::As | ExpressionInfixOperator::Satisfies
            )
        {
            return Ok(None);
        }

        if scope.stops_before(operator) {
            return Ok(None);
        }

        match operator {
            ExpressionInfixOperator::Assign(_) => Ok(None),
            ExpressionInfixOperator::TypeBinary(operator) => {
                let Some(left_type) = self.static_type_left(left) else {
                    return Ok(None);
                };

                Ok(Some(ValueInfixOperator::Type {
                    operator,
                    left_type,
                }))
            }
            ExpressionInfixOperator::Binary(operator) => {
                Ok(Some(ValueInfixOperator::Binary(operator)))
            }
            ExpressionInfixOperator::Is => Ok(Some(ValueInfixOperator::Is)),
            ExpressionInfixOperator::InstanceOf => Ok(Some(ValueInfixOperator::InstanceOf)),
            ExpressionInfixOperator::As => Ok(Some(ValueInfixOperator::As)),
            ExpressionInfixOperator::Satisfies => Ok(Some(ValueInfixOperator::Satisfies)),
            ExpressionInfixOperator::Range(end_kind) => {
                Ok(Some(ValueInfixOperator::Range(end_kind)))
            }
        }
    }

    /// Return the static type left side for one type relation.
    ///
    /// Examples:
    /// ```ds
    /// TypeA extends TypeB
    /// namespace.Type implements Contract
    /// Row extends string ? 4 : 2
    /// ```
    fn static_type_left(
        &mut self,
        left: LocalNodeId<Expression>,
    ) -> Option<LocalNodeId<TypeExpression>> {
        if let Expression::Type { value } = self.tree.get(left) {
            return Some(*value);
        }

        self.language
            .is_destack()
            .then(|| self.static_type_head_from_expression(left))
            .flatten()
    }

    /// Eat the current value infix operator span.
    ///
    /// Examples:
    /// ```ds
    /// +
    /// as
    /// ..
    /// ```
    fn eat_value_infix_operator_span(&mut self) -> Span {
        let operator_start = self.span_start();
        self.bump();

        self.get_span_from(&operator_start)
    }

    /// Eat one value infix operator after the operator token was consumed.
    ///
    /// Examples:
    /// ```ds
    /// left + right
    /// value satisfies Shape
    /// TypeA | TypeB
    /// ```
    fn eat_value_infix_expression(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        operator: ValueInfixOperator,
        operator_span: Span,
        right_scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        match operator {
            ValueInfixOperator::As => self.eat_value_assertion_expression(
                start,
                left,
                ExpressionInfixOperator::As,
                operator_span,
                right_scope,
            ),
            ValueInfixOperator::Satisfies => self.eat_value_assertion_expression(
                start,
                left,
                ExpressionInfixOperator::Satisfies,
                operator_span,
                right_scope,
            ),
            ValueInfixOperator::Is => {
                self.eat_value_predicate_expression(start, left, operator_span, right_scope)
            }
            ValueInfixOperator::Range(end_kind) => {
                self.eat_value_range_operator(start, left, end_kind, operator_span, right_scope)
            }
            ValueInfixOperator::Type {
                operator: TypeBinaryOperator::Extends,
                left_type,
            } => {
                let type_id = self.eat_type_conditional_rest(start, left_type, operator_span)?;

                Ok(self.insert_value_infix_expression(
                    start,
                    Expression::Type { value: type_id },
                    operator_span,
                    None,
                ))
            }
            ValueInfixOperator::Type {
                operator,
                left_type,
            } => self.eat_value_type_binary_operator(
                start,
                operator,
                left_type,
                operator_span,
                right_scope,
            ),
            ValueInfixOperator::Binary(operator) => {
                let right = self.eat_value_operand(right_scope)?;
                let expression = Expression::Binary {
                    left,
                    operator,
                    right,
                };

                Ok(self.insert_value_infix_expression(start, expression, operator_span, None))
            }
            ValueInfixOperator::InstanceOf => {
                let right = self.eat_value_operand(right_scope)?;
                let expression = Expression::InstanceOf {
                    value: left,
                    target: right,
                };

                Ok(self.insert_value_infix_expression(start, expression, operator_span, None))
            }
        }
    }

    /// Insert one folded value infix expression.
    fn insert_value_infix_expression(
        &mut self,
        start: &ParserSpanStart,
        expression: Expression,
        main_span: Span,
        head_span: Option<Span>,
    ) -> LocalNodeId<Expression> {
        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, main_span);
        if let Some(span) = head_span {
            self.tree.set_head_span(expression_id, span);
        }

        expression_id
    }

    /// Eat an assertion operator in value space.
    ///
    /// Examples:
    /// ```ds
    /// value as string
    /// value as const
    /// value satisfies Shape
    /// ```
    fn eat_value_assertion_expression(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        operator: ExpressionInfixOperator,
        operator_span: Span,
        right_scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let target_type = self.eat_type_expression_or_recover_missing(
            self.flags
                .with_type(true)
                .with_expression_context(right_scope.flags),
            NodeType::Expression,
        )?;

        let (expression, main_span, head_span) = if operator == ExpressionInfixOperator::As {
            let expression = Expression::As {
                expression: left,
                target_type,
            };
            if matches!(self.tree.get(target_type), TypeExpression::Const) {
                let target_span = self.tree.get_span(target_type);
                let main_span = Span::new(operator_span.file, operator_span.start, target_span.end);

                (expression, main_span, Some(self.expression_head_span(left)))
            } else {
                (expression, operator_span, None)
            }
        } else {
            let expression = Expression::Satisfies {
                expression: left,
                target_type,
            };

            (expression, operator_span, None)
        };

        Ok(self.insert_value_infix_expression(start, expression, main_span, head_span))
    }

    /// Eat a runtime type predicate in value space.
    ///
    /// Examples:
    /// ```ds
    /// value is string
    /// value is Ready
    /// value is { id: string }
    /// ```
    fn eat_value_predicate_expression(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        operator_span: Span,
        right_scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let target_type = self.eat_type_expression_or_recover_missing(
            self.flags
                .with_type(true)
                .with_expression_context(right_scope.flags),
            NodeType::Expression,
        )?;
        self.set_node_leading_span(target_type, operator_span.end);

        Ok(self.insert_value_infix_expression(
            start,
            Expression::Is {
                value: left,
                target_type,
            },
            operator_span,
            None,
        ))
    }

    /// Eat a range operator in value space.
    ///
    /// Examples:
    /// ```ds
    /// start..end
    /// start..
    /// start..=end
    /// ```
    fn eat_value_range_operator(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        end_kind: RangeEnd,
        operator_span: Span,
        right_scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let end = self.eat_value_range_end(end_kind, right_scope)?;

        Ok(self.insert_value_infix_expression(
            start,
            Expression::RangeExpression {
                start: Some(left),
                end,
                end_kind,
            },
            operator_span,
            None,
        ))
    }

    /// Eat the optional end expression of a value range.
    ///
    /// Examples:
    /// ```ds
    /// end
    /// call()
    /// value + offset
    /// ```
    fn eat_value_range_end(
        &mut self,
        end_kind: RangeEnd,
        right_scope: ExpressionScope,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        if self.current_token_is_on_new_line()
            || Self::is_expression_slot_boundary_token(self.peek_token_type())
        {
            if end_kind == RangeEnd::Inclusive {
                return Ok(Some(
                    self.recover_missing_expression_here(NodeType::Expression),
                ));
            }

            return Ok(None);
        }

        let end = self.eat_value_operand(right_scope)?;

        Ok(Some(end))
    }

    /// Eat a type binary operator from value space.
    ///
    /// Examples:
    /// ```ds
    /// A | B
    /// A & B
    /// A extends B ? C : D
    /// ```
    fn eat_value_type_binary_operator(
        &mut self,
        start: &ParserSpanStart,
        operator: TypeBinaryOperator,
        left_type: LocalNodeId<TypeExpression>,
        operator_span: Span,
        right_scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let right = self.eat_type_expression_or_recover_missing(
            self.flags
                .with_type(true)
                .with_expression_context(right_scope.flags),
            NodeType::Expression,
        )?;
        let type_id = self.make_type_infix_expression(
            self.get_span_from(start),
            self.type_expression_head_span(left_type),
            left_type,
            TypeInfixOperator::Relation(operator),
            right,
        )?;

        Ok(self.insert_value_infix_expression(
            start,
            Expression::Type { value: type_id },
            operator_span,
            None,
        ))
    }

    /// Return whether the current token belongs to an outer value grammar boundary.
    fn value_infix_is_boundary(
        &mut self,
        left: LocalNodeId<Expression>,
        scope: ExpressionScope,
        token_type: TokenType,
        is_on_new_line: bool,
    ) -> bool {
        if scope.is_new_receiver {
            return true;
        }

        if scope.is_static && Self::starts_type_angle_close(token_type) {
            return true;
        }

        if scope.is_typeof_query && is_on_new_line {
            return true;
        }

        if token_type == TokenType::Maybe {
            return true;
        }

        if scope.owns_colon_boundary && token_type == TokenType::Colon {
            return true;
        }

        if is_on_new_line
            && (scope.is_statement_position || scope.is_match_case_body)
            && self.tree.get(left).ends_statement_on_newline()
        {
            return true;
        }

        if scope.owns_for_each_boundary
            && matches!(self.current_keyword(), Some(Keyword::In | Keyword::Of))
        {
            return true;
        }

        false
    }

    /// Eat a conditional expression.
    ///
    /// Examples:
    /// ```ds
    /// condition ? then : else
    /// left + right
    /// value as Type
    /// ```
    pub(in crate::parse::expression) fn eat_conditional(
        &mut self,
        start: &ParserSpanStart,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let left = self.eat_binary(start, scope)?;

        self.eat_conditional_rest(start, left, scope)
    }

    /// Eat a conditional expression after an already parsed condition.
    ///
    /// Examples:
    /// ```ds
    /// ? then : else
    /// ? call() : fallback
    /// ? yes : other ? nested : fallback
    /// ```
    pub(in crate::parse::expression) fn eat_conditional_rest(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<Expression>,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let mut pending = Vec::new();
        let mut start = *start;
        let mut left = left;
        let mut condition_scope = scope;

        while self.current_token_starts_conditional(condition_scope) {
            // parse conditional branches
            self.bump();
            let then_expression = self.eat_conditional_then()?;
            self.eat_colon()?;

            // queue outer conditional until the final false branch is known
            pending.push(PendingConditionalExpression {
                start,
                condition: left,
                then_expression,
            });

            // continue through nested false branch conditionals
            match self.eat_conditional_else()? {
                ConditionalElseExpression::Expression(else_expression) => {
                    left = self.finish_pending_conditional_expressions(pending, else_expression);
                    break;
                }
                ConditionalElseExpression::Conditional {
                    start: else_start,
                    condition,
                } => {
                    start = else_start;
                    left = condition;
                    condition_scope = self.conditional_else_scope();
                }
            }
        }

        // tree literal boundary
        if !scope.is_statement_position
            && self.current_token_is_on_new_line()
            && self.can_start_tree_literal()
        {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        Ok(left)
    }

    /// Return whether a conditional expression can be parsed here.
    fn current_token_starts_conditional(&mut self, scope: ExpressionScope) -> bool {
        scope
            .minimum_precedence
            .is_none_or(|precedence| precedence < CONDITIONAL_PRECEDENCE)
            && self.peek_is(TokenType::Maybe)
    }

    /// Eat the true branch of a conditional expression.
    ///
    /// Examples:
    /// ```ds
    /// then
    /// call()
    /// a ? b : c
    /// ```
    fn eat_conditional_then(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_expression(
            self.flags
                .not_in_position()
                .in_ternary_condition()
                .not_in_sequence_expression(),
        )
    }

    /// Eat the false branch of a conditional expression.
    ///
    /// Examples:
    /// ```ds
    /// else
    /// call()
    /// (a ? b : c)
    /// ```
    fn eat_conditional_else(&mut self) -> ParserResult<ConditionalElseExpression> {
        // recover empty branch
        if Self::is_expression_slot_boundary_token(self.peek_token_type()) {
            let expression = self.recover_missing_expression_here(NodeType::Expression);

            return Ok(ConditionalElseExpression::Expression(expression));
        }

        let scope = self.conditional_else_scope();

        self.with_flags(scope.flags, |parser| {
            // parse up to a possible nested conditional
            let start = parser.span_start();
            let condition = parser.eat_binary(&start, scope)?;

            // continue through the nested conditional
            if parser.current_token_starts_conditional(scope) {
                return Ok(ConditionalElseExpression::Conditional { start, condition });
            }

            // finish ordinary false branch expression
            let expression = parser.eat_assignment_rest(&start, condition, scope)?;
            let expression = parser.eat_sequence_rest(&start, expression, scope)?;

            Ok(ConditionalElseExpression::Expression(expression))
        })
    }

    /// Return the scope for a ternary false branch.
    fn conditional_else_scope(&self) -> ExpressionScope {
        ExpressionScope::from_flags(self.flags.not_in_position().not_in_sequence_expression())
    }

    /// Finish pending right-associative ternary expression nodes.
    fn finish_pending_conditional_expressions(
        &mut self,
        pending: Vec<PendingConditionalExpression>,
        mut else_expression: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        for frame in pending.into_iter().rev() {
            // fold one pending ternary expression
            let else_span = self.tree.get_source_extent(else_expression);
            let span = Span::new(self.file_id, frame.start.token_start(), else_span.end);
            let expression = Expression::If {
                form: IfForm::Ternary,
                condition: IfCondition::Expression {
                    condition: frame.condition,
                },
                then_expression: frame.then_expression,
                else_expression: Some(else_expression),
            };

            else_expression = self.insert_node(expression, span);
        }

        else_expression
    }

    /// Eat a sequence expression after one parsed expression.
    ///
    /// Examples:
    /// ```ds
    /// first, second
    /// first, second, third
    /// first, call(second)
    /// ```
    pub(crate) fn eat_sequence_rest(
        &mut self,
        start: &ParserSpanStart,
        mut left: LocalNodeId<Expression>,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if scope.minimum_precedence.is_some()
            || !scope.allows_sequence
            || !(self.language.is_javascript() || self.language.is_typescript())
            || !self.peek_is(TokenType::Comma)
        {
            return Ok(left);
        }

        let expressions = self.eat_sequence_expressions(left)?;
        left = self.insert_node(
            Expression::SequenceExpression { expressions },
            self.get_span_from(start),
        );

        Ok(left)
    }

    /// Eat sequence expression operands after the first expression.
    ///
    /// Examples:
    /// ```ds
    /// , second
    /// , second, third
    /// , call(second)
    /// ```
    fn eat_sequence_expressions(
        &mut self,
        first: LocalNodeId<Expression>,
    ) -> ParserResult<Vec<LocalNodeId<Expression>>> {
        let mut expressions = vec![first];
        while self.peek_is(TokenType::Comma) {
            self.bump();
            let expression =
                self.eat_expression(self.flags.not_in_position().not_in_sequence_expression())?;
            expressions.push(expression);
        }

        Ok(expressions)
    }

    /// Parse a startless value range.
    ///
    /// Examples:
    /// ```ds
    /// ..end
    /// ..
    /// ..=end
    /// ```
    pub(in crate::parse::expression) fn eat_value_startless_range(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let end_kind = if self.peek_is(TokenType::RangeInclusive) {
            RangeEnd::Inclusive
        } else {
            RangeEnd::Open
        };
        self.bump();

        let right_scope = ExpressionScope::from_flags(self.flags.not_in_position())
            .at_precedence(Some(OperatorPrecedence::Range as u16));
        let end = self.eat_value_range_end(end_kind, right_scope)?;

        Ok(self.insert_node(
            Expression::RangeExpression {
                start: None,
                end,
                end_kind,
            },
            self.get_span_from(start),
        ))
    }

    /// Parse Destack reference operators in value space.
    ///
    /// Examples:
    /// ```ds
    /// &mut value
    /// &shared value
    /// ^local value
    /// ```
    pub(in crate::parse::expression) fn eat_value_reference_operator(
        &mut self,
        start: &ParserSpanStart,
        token_type: TokenType,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.bump();
        let mutability = self.eat_reference_mutability_maybe()?;
        let variance = self.eat_variance_bound_if_present()?;
        let right_scope =
            ExpressionScope::from_flags(self.flags.not_in_position().not_in_before_block())
                .at_precedence(Some(OperatorPrecedence::Prefix as u16));
        let right = self.with_recursive_descent(NodeType::Expression, |parser| {
            parser.eat_value_operand(right_scope)
        })?;
        let expression = if token_type == TokenType::ElementwiseAnd {
            Expression::BorrowOf {
                mutability,
                variance,
                right,
            }
        } else {
            Expression::MoveOf {
                mutability,
                variance,
                right,
            }
        };

        Ok(self.insert_node(expression, self.get_span_from(start)))
    }
}
