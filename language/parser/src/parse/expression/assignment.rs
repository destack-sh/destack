use crate::parse::expression::operator::ExpressionInfixOperator;
use crate::parse::scope::ExpressionScope;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};
use destack_dir::{
    Argument, AssignOperator, AssignPattern, AssignPatternField, Expression, Key, LocalNodeId,
    Name, Property,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// One pending right-associative assignment expression.
struct PendingAssignmentExpression {
    /// The assignment source start.
    start: ParserSpanStart,
    /// The assignment target.
    left: LocalNodeId<AssignPattern>,
    /// The assignment operator.
    operator: AssignOperator,
    /// The assignment operator span.
    operator_span: Span,
}

impl Parser {
    /// Eat an assignment expression.
    ///
    /// Examples:
    /// ```ds
    /// target = value
    /// object.field += amount
    /// [first, second = fallback] = values
    /// ```
    pub(crate) fn eat_assignment(
        &mut self,
        start: &ParserSpanStart,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let left = self.eat_conditional(start, scope)?;

        self.eat_assignment_rest(start, left, scope)
    }

    /// Eat an assignment tail after an already parsed left value.
    ///
    /// Examples:
    /// ```ds
    /// = value
    /// += amount
    /// ??= fallback
    /// ```
    pub(in crate::parse::expression) fn eat_assignment_rest(
        &mut self,
        start: &ParserSpanStart,
        left_expression: LocalNodeId<Expression>,
        scope: ExpressionScope,
    ) -> ParserResult<LocalNodeId<Expression>> {
        if scope.stops_before(ExpressionInfixOperator::Assign(AssignOperator::Assign)) {
            return Ok(left_expression);
        }

        let Some((operator, operator_span)) = self.eat_assignment_operator(scope)? else {
            return Ok(left_expression);
        };

        // validate expression target
        let left = self.assignment_pattern_from_expression(left_expression)?;
        self.require_assignable_operator(left, operator)?;
        let pending = vec![PendingAssignmentExpression {
            start: *start,
            left,
            operator,
            operator_span,
        }];

        self.eat_assignment_right_fold(pending)
    }

    /// Return the expression scope for assignment right sides.
    fn assignment_right_scope(&self) -> ExpressionScope {
        ExpressionScope::from_flags(self.flags.not_in_position().not_in_sequence_expression())
            .with_newline_call_boundary(true)
    }

    /// Eat a right-associative assignment tail and fold it from the right.
    fn eat_assignment_right_fold(
        &mut self,
        mut pending: Vec<PendingAssignmentExpression>,
    ) -> ParserResult<LocalNodeId<Expression>> {
        loop {
            let right_start = self.span_start();
            let right_scope = self.assignment_right_scope();

            // parse up to the next assignment operator
            let right = self.with_flags(right_scope.flags, |parser| {
                parser.eat_conditional(&right_start, right_scope)
            })?;
            let has_assignment_operator = self.assignment_operator_is_present(right_scope);
            if !has_assignment_operator {
                return Ok(self.finish_pending_assignment_expressions(pending, right));
            }

            let left = self.assignment_pattern_from_expression(right)?;
            let Some((operator, operator_span)) = self.eat_assignment_operator(right_scope)? else {
                return Err(ParserError::unexpected(self.peek()?.span));
            };
            self.require_assignable_operator(left, operator)?;
            pending.push(PendingAssignmentExpression {
                start: right_start,
                left,
                operator,
                operator_span,
            });
        }
    }

    /// Finish pending right-associative assignment expression nodes.
    fn finish_pending_assignment_expressions(
        &mut self,
        pending: Vec<PendingAssignmentExpression>,
        mut right: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        for frame in pending.into_iter().rev() {
            right = self.insert_assignment_expression(
                &frame.start,
                frame.left,
                frame.operator,
                frame.operator_span,
                right,
            );
        }

        right
    }

    /// Build an assignment expression node.
    fn insert_assignment_expression(
        &mut self,
        start: &ParserSpanStart,
        left: LocalNodeId<AssignPattern>,
        operator: AssignOperator,
        operator_span: Span,
        right: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let expression_id = self.insert_node(
            Expression::Assign {
                left,
                operator,
                right,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(expression_id, operator_span);

        expression_id
    }

    /// Eat an assignment operator owned by this expression scope.
    fn eat_assignment_operator(
        &mut self,
        scope: ExpressionScope,
    ) -> ParserResult<Option<(AssignOperator, Span)>> {
        let operator_is_outer =
            scope.stops_before(ExpressionInfixOperator::Assign(AssignOperator::Assign));
        if operator_is_outer {
            return Ok(None);
        }

        let Some(operator) = AssignOperator::from_token(self.peek_token_type()) else {
            return Ok(None);
        };

        let operator_start = self.span_start();
        self.bump();
        let operator_span = self.get_span_from(&operator_start);

        Ok(Some((operator, operator_span)))
    }

    /// Return whether an assignment operator is present in this scope.
    fn assignment_operator_is_present(&mut self, scope: ExpressionScope) -> bool {
        if scope.stops_before(ExpressionInfixOperator::Assign(AssignOperator::Assign)) {
            return false;
        }

        AssignOperator::from_token(self.peek_token_type()).is_some()
    }

    /// Require compound assignment to target a simple expression pattern.
    fn require_assignable_operator(
        &self,
        left: LocalNodeId<AssignPattern>,
        operator: AssignOperator,
    ) -> ParserResult<()> {
        if operator != AssignOperator::Assign
            && !matches!(self.tree.get(left), AssignPattern::Expression { .. })
        {
            Err(ParserError::unexpected(self.tree.get_span(left)))
        } else {
            Ok(())
        }
    }

    /// Build an assignment pattern from an already parsed value expression.
    ///
    /// Examples:
    /// ```ds
    /// target
    /// object.field
    /// array[index]
    /// ```
    fn assignment_pattern_from_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let is_destructuring_expression = matches!(
            self.tree.get(expression_id),
            Expression::ArrayExpression { .. } | Expression::ObjectExpression { .. }
        );

        // lower recursive destructuring under stack growth
        if is_destructuring_expression {
            destack_core::ensure_sufficient_stack(|| {
                self.lower_assignment_pattern_expression(expression_id)
            })
        } else {
            self.lower_assignment_pattern_expression(expression_id)
        }
    }

    /// Lower an assignment pattern expression at the current stack depth.
    fn lower_assignment_pattern_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let expression = self.tree.get(expression_id).clone();
        let is_parenthesized = matches!(expression, Expression::Parenthesized { .. })
            || self
                .tree
                .get_side_span(expression_id, NodeSpanType::Region(NodeSpanRegion::Wrapper))
                .is_some();

        // reject unparenthesized assertions
        if matches!(
            self.tree.get(expression_id),
            Expression::As { .. } | Expression::Satisfies { .. }
        ) && !is_parenthesized
        {
            return Err(ParserError::unexpected(self.tree.get_span(expression_id)));
        }

        // reject parenthesized assignments and destructuring expressions
        if let Expression::Parenthesized { expression } = &expression {
            match self.tree.get(*expression) {
                Expression::Assign { .. } => {
                    return Err(ParserError::unexpected(self.tree.get_span(*expression)));
                }
                Expression::ArrayExpression { .. } | Expression::ObjectExpression { .. } => {
                    return Err(ParserError::unexpected(self.tree.get_span(expression_id)));
                }
                _ => {}
            }
        }
        if is_parenthesized
            && matches!(
                self.tree.get(expression_id),
                Expression::Assign { .. }
                    | Expression::ArrayExpression { .. }
                    | Expression::ObjectExpression { .. }
            )
        {
            return Err(ParserError::unexpected(self.tree.get_span(expression_id)));
        }

        match expression {
            Expression::ArrayExpression { elements } => {
                return self.assignment_pattern_from_array_expression(expression_id, elements);
            }
            Expression::ObjectExpression { properties } => {
                return self.assignment_pattern_from_object_expression(expression_id, properties);
            }
            Expression::Assign {
                left,
                operator: AssignOperator::Assign,
                right,
            } => {
                return Ok(self.insert_node(
                    AssignPattern::Assign {
                        pattern: left,
                        value: right,
                    },
                    self.tree.get_span(expression_id),
                ));
            }
            Expression::Assign { .. } => {
                return Err(ParserError::unexpected(self.tree.get_span(expression_id)));
            }
            _ => {}
        }

        // reject non assignable values
        if !self.expression_is_simple_assignment_target(expression_id) {
            return Err(ParserError::unexpected(self.tree.get_span(expression_id)));
        }

        let target_id = self.without_parentheses_expression(expression_id);

        Ok(self.insert_node(
            AssignPattern::Expression { value: target_id },
            self.tree.get_span(expression_id),
        ))
    }

    /// Build a sequence assignment pattern from an array expression.
    fn assignment_pattern_from_array_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        elements: Vec<LocalNodeId<Argument>>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let fields = elements
            .into_iter()
            .map(|argument_id| self.assignment_field_from_argument(argument_id))
            .collect::<ParserResult<Vec<_>>>()?;

        Ok(self.insert_node(
            AssignPattern::Sequence { fields },
            self.tree.get_span(expression_id),
        ))
    }

    /// Build one sequence assignment field from an array expression argument.
    fn assignment_field_from_argument(
        &mut self,
        argument_id: LocalNodeId<Argument>,
    ) -> ParserResult<LocalNodeId<AssignPatternField>> {
        let argument = self.tree.get(argument_id).clone();
        let span = self.tree.get_span(argument_id);

        let field = match argument {
            Argument::Positional { value } if matches!(self.tree.get(value), Expression::Stub) => {
                AssignPatternField::Elision
            }
            Argument::Positional { value } => {
                let pattern = self.assignment_pattern_from_expression(value)?;

                AssignPatternField::Positional { pattern }
            }
            Argument::Spread { value, .. } => {
                let pattern = self.assignment_pattern_from_expression(value)?;

                AssignPatternField::Spread {
                    pattern: Some(pattern),
                }
            }
            Argument::Named { .. } | Argument::Labeled { .. } | Argument::Error => {
                return Err(ParserError::unexpected(span));
            }
        };

        Ok(self.insert_node(field, span))
    }

    /// Build an object assignment pattern from an object expression.
    fn assignment_pattern_from_object_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        properties: Vec<LocalNodeId<Property>>,
    ) -> ParserResult<LocalNodeId<AssignPattern>> {
        let fields = properties
            .into_iter()
            .map(|property_id| self.assignment_field_from_property(property_id))
            .collect::<ParserResult<Vec<_>>>()?;

        Ok(self.insert_node(
            AssignPattern::Object { fields },
            self.tree.get_span(expression_id),
        ))
    }

    /// Build one object assignment field from an object expression property.
    fn assignment_field_from_property(
        &mut self,
        property_id: LocalNodeId<Property>,
    ) -> ParserResult<LocalNodeId<AssignPatternField>> {
        let property = self.tree.get(property_id).clone();
        let span = self.tree.get_span(property_id);

        let field = match property {
            Property::Field {
                key: Key::Name(name),
                value,
                is_shorthand,
            } => {
                let pattern = if is_shorthand && self.expression_is_identifier_name(value, &name) {
                    None
                } else {
                    Some(self.assignment_pattern_from_expression(value)?)
                };

                AssignPatternField::Named {
                    name,
                    pattern,
                    is_shorthand,
                }
            }
            Property::Field {
                key: Key::Expression(key),
                value,
                ..
            } => {
                let pattern = self.assignment_pattern_from_expression(value)?;

                AssignPatternField::Computed { key, pattern }
            }
            Property::Field {
                key: Key::Private(_),
                ..
            }
            | Property::Method { .. }
            | Property::Error => {
                return Err(ParserError::unexpected(span));
            }
            Property::Spread { value } => {
                let pattern = self.assignment_pattern_from_expression(value)?;

                AssignPatternField::Spread {
                    pattern: Some(pattern),
                }
            }
        };

        Ok(self.insert_node(field, span))
    }

    /// Return whether one expression is the shorthand value for a name.
    fn expression_is_identifier_name(
        &self,
        expression_id: LocalNodeId<Expression>,
        name: &Name,
    ) -> bool {
        let Name::Identifier(expected) = name else {
            return false;
        };

        matches!(
            self.tree.get(expression_id),
            Expression::Identifier { name } if name == expected
        )
    }

    /// Return whether one expression is a simple assignment target.
    fn expression_is_simple_assignment_target(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let expression_id = self.without_parentheses_expression(expression_id);

        match self.tree.get(expression_id) {
            Expression::Identifier { .. } => true,
            Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. } => !self.expression_contains_optional_chain(expression_id),
            Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
                self.expression_is_simple_assignment_target(*expression)
            }
            Expression::Unary {
                operator: destack_dir::UnaryOperator::Dereference,
                right,
            } if self.language.is_destack() => self.expression_is_simple_assignment_target(*right),
            Expression::Must { left, .. } => self.expression_is_simple_assignment_target(*left),
            _ => false,
        }
    }

    /// Return whether one expression target contains optional chaining.
    fn expression_contains_optional_chain(&self, expression_id: LocalNodeId<Expression>) -> bool {
        let expression_id = self.without_parentheses_expression(expression_id);

        match self.tree.get(expression_id) {
            Expression::Maybe { .. } => true,
            Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
                self.expression_contains_optional_chain(*left)
            }
            Expression::Index { left, .. } => self.expression_contains_optional_chain(*left),
            Expression::As { expression, .. } | Expression::Satisfies { expression, .. } => {
                self.expression_contains_optional_chain(*expression)
            }
            Expression::Must { left, .. } => self.expression_contains_optional_chain(*left),
            _ => false,
        }
    }
}
