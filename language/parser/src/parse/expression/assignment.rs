use crate::parse::expression::operator::ExpressionInfixOperator;
use crate::parse::scope::ExpressionScope;
use crate::{ParseError, ParseResult, Parser, ParserCheckpoint, ParserSpanStart};
use destack_dir::{
    AssignOperator, AssignPattern, AssignPatternField, Expression, Key, LocalNodeId, Name,
    NodeType, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

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
    ) -> ParseResult<LocalNodeId<Expression>> {
        // remember source for destructuring targets
        let starts_destructuring = matches!(
            self.peek_token_type(),
            TokenType::OpenBracket | TokenType::OpenBrace
        );
        let checkpoint = starts_destructuring.then(|| (self.checkpoint(), self.tree.next_id()));
        let left = self.eat_conditional(start, scope)?;

        // reparse destructuring targets as patterns
        let can_assign = !scope
            .stops_before(ExpressionInfixOperator::Assign(AssignOperator::Assign))
            && AssignOperator::from_token(self.peek_token_type()).is_some();
        if can_assign
            && matches!(
                self.tree.get(left),
                Expression::ArrayExpression { .. } | Expression::ObjectExpression { .. }
            )
        {
            return self.eat_destructuring_assignment(start, scope, checkpoint);
        }

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
    ) -> ParseResult<LocalNodeId<Expression>> {
        if scope.stops_before(ExpressionInfixOperator::Assign(AssignOperator::Assign)) {
            return Ok(left_expression);
        }

        let Some((operator, operator_span)) = self.eat_assignment_operator(scope)? else {
            return Ok(left_expression);
        };

        // validate expression target
        let left = self.assignment_pattern_from_expression(left_expression)?;
        self.require_assignable_operator(left, operator)?;

        // finish assignment
        let right = self.eat_assignment_right_expression()?;

        Ok(self.insert_assignment_expression(start, left, operator, operator_span, right))
    }

    /// Parse the right side of an assignment expression.
    ///
    /// Examples:
    /// ```ds
    /// value + 1
    /// await load()
    /// condition ? yes : no
    /// ```
    fn eat_assignment_right_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let right_scope =
            ExpressionScope::from_flags(self.flags.not_in_position().not_in_sequence_expression())
                .with_newline_call_boundary(true);

        self.eat_expression_scope(right_scope)
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
    ) -> ParseResult<Option<(AssignOperator, Span)>> {
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

    /// Require compound assignment to target a simple expression pattern.
    fn require_assignable_operator(
        &self,
        left: LocalNodeId<AssignPattern>,
        operator: AssignOperator,
    ) -> ParseResult<()> {
        if operator != AssignOperator::Assign
            && !matches!(self.tree.get(left), AssignPattern::Expression { .. })
        {
            Err(ParseError::unexpected(self.tree.get_span(left)))
        } else {
            Ok(())
        }
    }

    /// Eat a destructuring assignment from source.
    ///
    /// Examples:
    /// ```ds
    /// target = value
    /// [first, ...rest] = values
    /// { value: target = fallback } = object
    /// ```
    fn eat_destructuring_assignment(
        &mut self,
        start: &ParserSpanStart,
        scope: ExpressionScope,
        checkpoint: Option<(ParserCheckpoint, u32)>,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // restore the source shaped target
        let Some((checkpoint, mark)) = checkpoint else {
            return Err(ParseError::unexpected(self.peek()?.span));
        };
        self.restore(checkpoint, mark);

        // parse the target as a pattern
        let left = self.eat_assignment_target_pattern(false)?;
        let Some((operator, operator_span)) = self.eat_assignment_operator(scope)? else {
            return Err(ParseError::unexpected(self.peek()?.span));
        };
        self.require_assignable_operator(left, operator)?;

        // finish assignment
        let right = self.eat_assignment_right_expression()?;

        Ok(self.insert_assignment_expression(start, left, operator, operator_span, right))
    }

    /// Eat one assignment target pattern from source.
    ///
    /// Examples:
    /// ```ds
    /// target
    /// [first, second = fallback]
    /// { name, value: target }
    /// ```
    fn eat_assignment_target_pattern(
        &mut self,
        allows_default: bool,
    ) -> ParseResult<LocalNodeId<AssignPattern>> {
        let start = self.span_start();
        let mut pattern = match self.peek_token_type() {
            TokenType::OpenBracket => self.eat_assignment_sequence_pattern(&start)?,
            TokenType::OpenBrace => self.eat_assignment_object_pattern(&start)?,
            _ => self.eat_assignment_expression_pattern()?,
        };

        // default value
        if allows_default && let Some(operator) = AssignOperator::from_token(self.peek_token_type())
        {
            if operator != AssignOperator::Assign {
                self.bump();
                let value = self.eat_assignment_right_expression()?;
                let pattern_span = self.tree.get_span(pattern);
                let value_span = self.tree.get_span(value);
                let span = Span::new(pattern_span.file, pattern_span.start, value_span.end);

                return Err(ParseError::unexpected(span));
            }

            let assign_start = self.span_start();
            self.bump();
            let value = self.eat_assignment_right_expression()?;
            pattern = self.insert_node(
                AssignPattern::Assign { pattern, value },
                self.get_span_from(&assign_start),
            );
        }

        Ok(pattern)
    }

    /// Eat one sequence assignment pattern from source.
    ///
    /// Examples:
    /// ```ds
    /// []
    /// [first, , second]
    /// [first, ...rest]
    /// ```
    fn eat_assignment_sequence_pattern(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<AssignPattern>> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut fields = Vec::new();
        let mut expects_field = true;

        while self.has_more_tokens() && !self.peek_is(TokenType::CloseBracket) {
            if self.peek_is(TokenType::Comma) {
                if expects_field {
                    let span = self.peek()?.span;
                    let field = self.insert_node(AssignPatternField::Elision, span);
                    fields.push(field);
                }

                self.eat_item_stop()?;
                expects_field = true;
                continue;
            }

            let field = self.eat_assignment_sequence_field()?;
            fields.push(field);
            expects_field = false;
        }

        self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::AssignPattern)?;

        Ok(self.insert_node(
            AssignPattern::Sequence { fields },
            self.get_span_from(start),
        ))
    }

    /// Eat one sequence assignment field from source.
    ///
    /// Examples:
    /// ```ds
    /// first
    /// second = fallback
    /// ...rest
    /// ```
    fn eat_assignment_sequence_field(&mut self) -> ParseResult<LocalNodeId<AssignPatternField>> {
        let start = self.span_start();
        let field = if self.peek_is(TokenType::Spread) {
            self.bump();
            let pattern = self.eat_assignment_target_pattern(false)?;

            AssignPatternField::Spread {
                pattern: Some(pattern),
            }
        } else {
            let pattern = self.eat_assignment_target_pattern(true)?;

            AssignPatternField::Positional { pattern }
        };

        Ok(self.insert_node(field, self.get_span_from(&start)))
    }

    /// Eat one object assignment pattern from source.
    ///
    /// Examples:
    /// ```ds
    /// {}
    /// { name, value: target }
    /// { [key]: target = fallback, ...rest }
    /// ```
    fn eat_assignment_object_pattern(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParseResult<LocalNodeId<AssignPattern>> {
        self.eat_token(TokenType::OpenBrace)?;
        let mut fields = Vec::new();

        while self.has_more_tokens() && !self.peek_is(TokenType::CloseBrace) {
            if self.peek_is(TokenType::Comma) {
                self.eat_item_stop()?;
                continue;
            }

            if Self::is_any_stop_token(self.peek_token_type()) {
                self.eat_any_stop()?;
                continue;
            }

            let field = self.eat_assignment_object_field()?;
            fields.push(field);
        }

        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::AssignPattern)?;

        Ok(self.insert_node(AssignPattern::Object { fields }, self.get_span_from(start)))
    }

    /// Eat one object assignment field from source.
    ///
    /// Examples:
    /// ```ds
    /// name
    /// name = fallback
    /// value: target
    /// ```
    fn eat_assignment_object_field(&mut self) -> ParseResult<LocalNodeId<AssignPatternField>> {
        let start = self.span_start();

        if self.peek_is(TokenType::Spread) {
            self.bump();
            let pattern = self.eat_assignment_target_pattern(false)?;
            let field = AssignPatternField::Spread {
                pattern: Some(pattern),
            };

            return Ok(self.insert_node(field, self.get_span_from(&start)));
        }

        let (key, key_span) = self.eat_key_with_span()?;
        let field = self.eat_assignment_keyed_field(key, key_span)?;

        Ok(self.insert_node(field, self.get_span_from(&start)))
    }

    /// Eat an assignment field after its object key.
    ///
    /// Examples:
    /// ```ds
    /// : target
    /// = fallback
    /// ,
    /// ```
    fn eat_assignment_keyed_field(
        &mut self,
        key: Key,
        key_span: Span,
    ) -> ParseResult<AssignPatternField> {
        if self.peek_colon_is() {
            self.bump();
            let pattern = self.eat_assignment_target_pattern(true)?;
            return match key {
                Key::Name(name) => Ok(AssignPatternField::Named {
                    name,
                    pattern: Some(pattern),
                    is_shorthand: false,
                }),
                Key::Expression(key) => Ok(AssignPatternField::Computed { key, pattern }),
                Key::Private(_) => Err(ParseError::unexpected(key_span)),
            };
        }

        let Key::Name(name @ Name::Identifier(identifier)) = key else {
            return Err(ParseError::unexpected(key_span));
        };

        if !self.peek_is(TokenType::Assign) {
            return Ok(AssignPatternField::Named {
                name,
                pattern: None,
                is_shorthand: true,
            });
        }

        self.bump();
        let value = self.eat_assignment_right_expression()?;
        let target_value = self.insert_node(Expression::Identifier { name: identifier }, key_span);
        let target = self.insert_node(
            AssignPattern::Expression {
                value: target_value,
            },
            key_span,
        );
        let pattern = self.insert_node(
            AssignPattern::Assign {
                pattern: target,
                value,
            },
            self.tree.get_span(value),
        );

        Ok(AssignPatternField::Named {
            name,
            pattern: Some(pattern),
            is_shorthand: true,
        })
    }

    /// Eat one expression assignment pattern from source.
    ///
    /// Examples:
    /// ```ds
    /// target
    /// object.field
    /// array[index]
    /// ```
    fn eat_assignment_expression_pattern(&mut self) -> ParseResult<LocalNodeId<AssignPattern>> {
        let start = self.span_start();
        let expression_id =
            self.eat_conditional(&start, ExpressionScope::from_flags(self.flags))?;

        self.assignment_pattern_from_expression(expression_id)
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
    ) -> ParseResult<LocalNodeId<AssignPattern>> {
        let is_parenthesized = matches!(
            self.tree.get(expression_id),
            Expression::Parenthesized { .. }
        ) || self
            .tree
            .get_side_span(expression_id, NodeSpanType::Region(NodeSpanRegion::Wrapper))
            .is_some();

        // reject unparenthesized assertions
        if matches!(
            self.tree.get(expression_id),
            Expression::As { .. } | Expression::Satisfies { .. }
        ) && !is_parenthesized
        {
            return Err(ParseError::unexpected(self.tree.get_span(expression_id)));
        }

        // reject parenthesized assignments and destructuring expressions
        if let Expression::Parenthesized { expression } = self.tree.get(expression_id) {
            match self.tree.get(*expression) {
                Expression::Assign { .. } => {
                    return Err(ParseError::unexpected(self.tree.get_span(*expression)));
                }
                Expression::ArrayExpression { .. } | Expression::ObjectExpression { .. } => {
                    return Err(ParseError::unexpected(self.tree.get_span(expression_id)));
                }
                _ => {}
            }
        }

        // reject non assignable values
        if !self.expression_is_simple_assignment_target(expression_id) {
            return Err(ParseError::unexpected(self.tree.get_span(expression_id)));
        }

        let target_id = self.without_parentheses_expression(expression_id);

        Ok(self.insert_node(
            AssignPattern::Expression { value: target_id },
            self.tree.get_span(expression_id),
        ))
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
