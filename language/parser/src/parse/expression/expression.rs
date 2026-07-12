use crate::parse::context::{
    DecoratorContext, ExpressionContext, ExpressionMode, ExpressionStops, StatementPosition,
    TypeContext,
};
use crate::parse::expression::operator::ExpressionOperator;
use crate::{Parser, ParserResult};
use destack_dir::{
    BinaryOperator, Condition, Expression, IfForm, LocalNodeId, NodeType, OperatorPrecedence,
    RangeEnd, TokenType,
};
use destack_source::{ByteRange, NodeSpanRegion, NodeSpanType};
use smallvec::{SmallVec, smallvec};

/// One right associative value operation awaiting its final right operand.
struct ExpressionInfix {
    /// The left operand.
    left: LocalNodeId<Expression>,
    /// The infix operator.
    operator: ExpressionOperator,
    /// The operator source range.
    range: ByteRange,
}

/// One conditional expression branch awaiting its final false branch.
struct ConditionalExpressionBranch {
    /// The branch condition.
    condition: LocalNodeId<Expression>,
    /// The true branch expression.
    then_expression: LocalNodeId<Expression>,
    /// The question mark source range.
    question: ByteRange,
    /// The colon source range or recovery anchor.
    colon: ByteRange,
}

impl Parser {
    /// Parse one complete value expression.
    ///
    /// Examples:
    /// ```ds
    /// left + right * 2
    /// ```
    pub(crate) fn parse_expression(
        &mut self,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.with_recursive_descent(NodeType::Expression, |parser| {
            parser.parse_expression_after_descent(context)
        })
    }

    /// Parse one value expression after entering recursive descent state.
    fn parse_expression_after_descent(
        &mut self,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // parse decorators only at their owning expression level
        let decorators =
            if context.decorator == DecoratorContext::None && self.peek_is(TokenType::At) {
                Some(self.parse_decorators(context.function))
            } else {
                None
            };

        // parse the operand and iterative operator tail
        let first = if self.peek_is(TokenType::ElementwiseOr) {
            self.parse_leading_or_expression(context)?
        } else {
            self.parse_expression_operand(context)?
        };
        let expression = self.parse_expression_tail_after_descent(first, context)?;

        // attach decorators to declarations rather than their expression wrappers
        if let Some(decorators) = decorators {
            let owner = match self.tree.get(expression) {
                Expression::Declaration(declaration) => declaration.id,
                _ => expression.id,
            };
            self.attach_decorators(owner, decorators);
        }

        Ok(expression)
    }

    /// Parse one operator right operand within the current recursive-descent level.
    fn parse_expression_right_operand(
        &mut self,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // recover a missing operand at an enclosing grammar boundary
        if self.peek_expression_slot_boundary() {
            return Ok(self.recover_missing_expression_here(NodeType::Expression));
        }

        // guard source nesting introduced by a leading separator
        if self.peek_is(TokenType::ElementwiseOr) {
            return self.parse_expression(context);
        }

        self.parse_expression_after_descent(context)
    }

    /// Parse one value operator tail after entering recursive descent state.
    #[inline(never)]
    fn parse_expression_tail_after_descent(
        &mut self,
        mut left: LocalNodeId<Expression>,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        loop {
            // parse one conditional tail at its fixed precedence
            if self.peek_is(TokenType::Maybe)
                && !context
                    .stops
                    .contains(ExpressionStops::CONDITIONAL_QUESTION)
                && OperatorPrecedence::Conditional > context.minimum_precedence
            {
                left = self.parse_conditional_expression(left, context)?;

                continue;
            }

            // classify one infix operation owned by this expression level
            let Some(operator) = self.peek_expression_operator(left, context) else {
                break;
            };
            let precedence = operator.precedence();

            // type relations transfer the complete tail to the type reducer
            if matches!(operator, ExpressionOperator::Type(_)) {
                let type_left = self.promote_expression_type(left)?;
                let ty = self.parse_type_tail(type_left, TypeContext::from(context))?;
                left = self.insert_type_expression_value(ty);

                continue;
            }

            let range = self.peek_token().range();
            self.bump();

            // type-valued operations switch grammar for their complete right operand
            if operator.has_type_operand() {
                let target_type = self.parse_type_or_recover_missing(
                    TypeContext::from(context),
                    NodeType::Expression,
                )?;
                left = self.insert_expression_type_infix(left, operator, range, target_type)?;

                continue;
            }

            // open ranges may omit their right endpoint
            if matches!(operator, ExpressionOperator::Range(RangeEnd::Open))
                && self.peek_expression_range_end_omitted()
            {
                left = self.insert_open_range_expression(left, range);

                break;
            }

            // collect right associative runs without recursive chain depth
            if precedence.is_right_associative() {
                left = self.parse_right_associative_expression(
                    left, operator, range, precedence, context,
                )?;

                continue;
            }

            // parse the complete right operand at this operator's binding power
            let right_context = context.right(precedence);
            let right = self.parse_expression_right_operand(right_context)?;
            left = self.insert_expression_infix(left, operator, range, right)?;
        }

        Ok(left)
    }

    /// Parse one right associative infix run without recursive chain depth.
    fn parse_right_associative_expression(
        &mut self,
        left: LocalNodeId<Expression>,
        operator: ExpressionOperator,
        range: ByteRange,
        precedence: OperatorPrecedence,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let mut operations: SmallVec<[ExpressionInfix; 4]> = smallvec![ExpressionInfix {
            left,
            operator,
            range,
        }];

        loop {
            // parse operators stronger than this right associative run
            let mut right_context = context.right(precedence);
            if matches!(operator, ExpressionOperator::Assign(_)) {
                right_context.stops = right_context
                    .stops
                    .with(ExpressionStops::NEWLINE_CALL)
                    .without(ExpressionStops::CONDITIONAL_QUESTION);
            }
            let right = self.parse_expression_right_operand(right_context)?;

            // collect another operation at exactly this precedence
            if let Some(next) = self.peek_expression_operator(right, context)
                && next.precedence() == precedence
                && next.precedence().is_right_associative()
            {
                // record the next operation in the right associative run
                let range = self.peek_token().range();
                self.bump();
                operations.push(ExpressionInfix {
                    left: right,
                    operator: next,
                    range,
                });

                continue;
            }

            // fold the right associative run from its final operand
            return operations
                .into_iter()
                .rev()
                .try_fold(right, |right, operation| {
                    self.insert_expression_infix(
                        operation.left,
                        operation.operator,
                        operation.range,
                        right,
                    )
                });
        }
    }

    /// Parse one conditional expression after its condition.
    fn parse_conditional_expression(
        &mut self,
        mut condition: LocalNodeId<Expression>,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let mut branches: SmallVec<[ConditionalExpressionBranch; 4]> = SmallVec::new();

        loop {
            // parse ? thenExpression
            let question = self.peek_token().range();
            self.bump();
            let then_expression = self.parse_expression_right_operand(ExpressionContext {
                stops: context.stops.with(ExpressionStops::CONDITIONAL_COLON),
                minimum_precedence: OperatorPrecedence::Lowest,
                ..context
            })?;

            // parse or recover the conditional colon
            let colon =
                self.eat_token_range_or_recover_missing(TokenType::Colon, NodeType::Expression);
            branches.push(ConditionalExpressionBranch {
                condition,
                then_expression,
                question,
                colon,
            });

            // parse the next false-branch head without recursive conditional depth
            condition = self.parse_expression_right_operand(ExpressionContext {
                stops: context.stops.with(ExpressionStops::CONDITIONAL_QUESTION),
                minimum_precedence: OperatorPrecedence::Lowest,
                ..context
            })?;
            if !self.peek_is(TokenType::Maybe) {
                break;
            }
        }

        // fold the right associative conditional ladder from its final branch
        let expression = branches
            .into_iter()
            .rev()
            .fold(condition, |else_expression, branch| {
                self.insert_conditional_expression(
                    branch.condition,
                    branch.then_expression,
                    else_expression,
                    branch.question,
                    branch.colon,
                )
            });

        Ok(expression)
    }

    /// Return the current infix operation owned by one expression.
    fn peek_expression_operator(
        &self,
        left: LocalNodeId<Expression>,
        context: ExpressionContext,
    ) -> Option<ExpressionOperator> {
        // classify the source token before evaluating contextual ownership
        let token_type = self.peek_token_type();
        let keyword = (token_type == TokenType::Identifier)
            .then(|| self.peek_keyword())
            .flatten();
        let operator = ExpressionOperator::from_token(token_type, keyword)?;

        if self.is_expression_operator_stopped(left, operator, context) {
            return None;
        }

        if matches!(operator, ExpressionOperator::Range(_)) && self.peek_is_on_new_line() {
            return None;
        }

        (operator.precedence() > context.minimum_precedence).then_some(operator)
    }

    /// Parse an elementwise-or expression with a leading separator.
    fn parse_leading_or_expression(
        &mut self,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let operator = BinaryOperator::ElementwiseOr;
        let precedence = operator.precedence();
        self.bump();

        let mut left = self.parse_expression(context.right(precedence))?;
        let mut has_binary = false;
        while self.peek_is(TokenType::ElementwiseOr) {
            let operator_range = self.peek_token_span().token.range();
            self.bump();
            let right = self.parse_expression(context.right(precedence))?;
            left = self.insert_expression_infix(
                left,
                ExpressionOperator::Binary(operator),
                operator_range,
                right,
            )?;
            has_binary = true;
        }

        // preserve the leading separator in the binary chain source range
        if has_binary {
            self.tree.set_range(left, self.range_since(&start));
        }

        Ok(left)
    }

    /// Return whether the current source position omits a range end.
    pub(in crate::parse::expression) fn peek_expression_range_end_omitted(&self) -> bool {
        self.peek_is_on_new_line() || self.peek_expression_slot_boundary()
    }

    /// Return whether the current token belongs to an enclosing value grammar.
    fn is_expression_operator_stopped(
        &self,
        left: LocalNodeId<Expression>,
        operator: ExpressionOperator,
        context: ExpressionContext,
    ) -> bool {
        // leave every operator outside a constructor receiver
        if context.mode == ExpressionMode::NewReceiver {
            return true;
        }

        // leave generic closing angles to the enclosing argument list
        if context.stops.contains(ExpressionStops::ANGLE_CLOSE)
            && matches!(
                operator,
                ExpressionOperator::Binary(
                    BinaryOperator::GreaterThan
                        | BinaryOperator::ShiftRight
                        | BinaryOperator::UnsignedShiftRight
                )
            )
        {
            return true;
        }

        // terminate typeof queries at a line boundary
        if context.mode == ExpressionMode::TypeofQuery && self.peek_is_on_new_line() {
            return true;
        }

        // keep line-leading type assertions outside the preceding expression
        if self.peek_is_on_new_line()
            && matches!(
                operator,
                ExpressionOperator::As | ExpressionOperator::Satisfies
            )
        {
            return true;
        }

        // leave match continuation lines to the enclosing case
        if context.stops.contains(ExpressionStops::MATCH_LINE) && self.peek_is_on_new_line() {
            return true;
        }

        // leave iteration relation keywords to the enclosing loop
        if context.stops.contains(ExpressionStops::FOR_EACH)
            && matches!(operator, ExpressionOperator::Binary(BinaryOperator::In))
        {
            return true;
        }

        // keep a line-leading tree outside a completed statement
        if context.statement == StatementPosition::Direct
            && self.peek_is_on_new_line()
            && matches!(
                operator,
                ExpressionOperator::Binary(BinaryOperator::LessThan)
            )
            && self.peek_tree_literal_start()
        {
            return true;
        }

        // honor expressions that terminate a direct statement on newline
        context.statement == StatementPosition::Direct
            && self.peek_is_on_new_line()
            && self.tree.get(left).ends_statement_on_newline()
    }

    /// Insert one conditional expression.
    fn insert_conditional_expression(
        &mut self,
        condition: LocalNodeId<Expression>,
        then_expression: LocalNodeId<Expression>,
        else_expression: LocalNodeId<Expression>,
        question_range: ByteRange,
        colon_range: ByteRange,
    ) -> LocalNodeId<Expression> {
        let condition_range = self.tree.get_range(condition);
        let else_range = self.tree.get_range(else_expression);
        let range = ByteRange {
            start: condition_range.start,
            end: else_range.end,
        };
        let expression = self.insert_node(
            Expression::If {
                form: IfForm::Ternary,
                condition: Condition::expression(condition),
                then_expression,
                else_expression: Some(else_expression),
            },
            range,
        );
        self.tree.set_main_range(expression, question_range);
        self.tree.set_side_range(
            expression,
            NodeSpanType::Region(NodeSpanRegion::Clause),
            colon_range,
        );

        expression
    }
}
