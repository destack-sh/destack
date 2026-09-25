use crate::parse::expression::operator::ExpressionOperator;
use crate::parse::{TypePosition, TypeStop};
use crate::{Parser, ParserResult};
use smallvec::{SmallVec, smallvec};
use tspp_dir::{
    BinaryOperator, Condition, Expression, IfForm, LocalNodeId, NodeType, OperatorPrecedence,
    RangeEnd, TokenType,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

/// The source position of one value expression.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum ExpressionPosition {
    /// Parse an ordinary value expression.
    #[default]
    Value,
    /// Parse an expression directly occupying a statement slot.
    Statement,
    /// Parse an expression nested within a statement.
    NestedStatement,
    /// Parse a block expression at the expression head.
    Block,
    /// Parse the first decorator path segment.
    DecoratorHead,
    /// Parse a nested decorator value.
    DecoratorValue,
    /// Parse a value embedded in tree syntax.
    Tree,
    /// Parse the operand of a `typeof` query.
    TypeQuery,
    /// Parse a constructor operand before its argument list.
    Constructor,
}

impl ExpressionPosition {
    /// Return the position inherited through explicit nesting.
    pub(crate) const fn nested(self) -> Self {
        match self {
            Self::Statement | Self::NestedStatement | Self::Block => Self::NestedStatement,
            Self::DecoratorHead | Self::DecoratorValue => Self::DecoratorValue,
            Self::Value | Self::Tree | Self::TypeQuery | Self::Constructor => Self::Value,
        }
    }

    /// Return the position inherited by an infix operand.
    pub(crate) const fn right(self) -> Self {
        match self {
            Self::Statement | Self::NestedStatement | Self::Block => Self::NestedStatement,
            Self::DecoratorHead | Self::DecoratorValue => Self::DecoratorValue,
            position => position,
        }
    }

    /// Return whether this expression directly occupies a statement slot.
    pub(crate) const fn is_statement(self) -> bool {
        matches!(self, Self::Statement | Self::Block)
    }

    /// Return whether this expression is nested within a statement.
    pub(crate) const fn is_nested_statement(self) -> bool {
        matches!(self, Self::NestedStatement)
    }

    /// Return whether this expression belongs to any statement position.
    pub(crate) const fn is_in_statement(self) -> bool {
        matches!(self, Self::Statement | Self::NestedStatement | Self::Block)
    }

    /// Return whether this expression belongs to a decorator.
    pub(crate) const fn is_decorator(self) -> bool {
        matches!(self, Self::DecoratorHead | Self::DecoratorValue)
    }
}

/// Token ownership inherited by infix operands.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct ExpressionStop(u8);

impl ExpressionStop {
    /// A colon owned by an enclosing switch selector.
    pub(crate) const SWITCH_COLON: Self = Self(1 << 0);
    /// An `of` token owned by an enclosing iteration clause.
    pub(crate) const FOR_EACH: Self = Self(1 << 1);
    /// A newline owned by an enclosing match arm.
    pub(crate) const MATCH_ARM_LINE: Self = Self(1 << 2);
    /// An angle close owned by an enclosing generic argument list.
    pub(crate) const ANGLE_CLOSE: Self = Self(1 << 3);
    /// A newline call owned by an enclosing statement expression.
    pub(crate) const NEWLINE_CALL: Self = Self(1 << 4);
    /// A brace reserved for an enclosing control body during recovery.
    pub(crate) const BODY_BRACE: Self = Self(1 << 5);
    /// A colon owned by an enclosing conditional expression.
    pub(crate) const CONDITIONAL_COLON: Self = Self(1 << 6);
    /// A question owned by an enclosing iterative conditional ladder.
    pub(crate) const CONDITIONAL_QUESTION: Self = Self(1 << 7);

    /// Add one enclosing token.
    pub(crate) const fn add(self, stop: Self) -> Self {
        Self(self.0 | stop.0)
    }

    /// Remove one enclosing token.
    pub(crate) const fn remove(self, stop: Self) -> Self {
        Self(self.0 & !stop.0)
    }

    /// Return whether one token belongs to the enclosing expression.
    pub(crate) const fn has(self, stop: Self) -> bool {
        self.0 & stop.0 != 0
    }
}

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
    /// ```tspp
    /// left + right * 2
    /// ```
    pub(crate) fn parse_expression(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.parse_expression_at(position, stop, OperatorPrecedence::Lowest)
    }

    /// Parse one value expression at the given minimum precedence.
    pub(crate) fn parse_expression_at(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
        minimum_precedence: OperatorPrecedence,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.with_recursive_descent(NodeType::Expression, |parser| {
            parser.parse_expression_after_descent(position, stop, minimum_precedence)
        })
    }

    /// Parse one value expression after checking the recursion depth.
    fn parse_expression_after_descent(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
        minimum_precedence: OperatorPrecedence,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let documentation = self.parse_documentation();

        // parse decorators only at their owning expression level
        let decorators = if !position.is_decorator() && self.peek_is(TokenType::At) {
            Some(self.parse_decorators())
        } else {
            None
        };

        // parse the operand and iterative operator tail
        let first = if OperatorPrecedence::Assignment > minimum_precedence
            && self.peek_destructuring_assignment()
        {
            let start = self.mark_parse_start();
            self.parse_destructuring_assignment(&start, position, stop)?
        } else if self.peek_is(TokenType::ElementwiseOr) {
            self.parse_leading_or_expression(position, stop)?
        } else {
            self.parse_expression_operand(position, stop)?
        };
        let expression =
            self.parse_expression_tail_after_descent(first, position, stop, minimum_precedence)?;

        // attach documentation to the complete expression owner
        match self.tree.get(expression) {
            Expression::Declaration(declaration) => {
                let declaration = *declaration;
                self.attach_documentation(declaration, documentation);
            }
            Expression::Missing | Expression::Error => {}
            _ => self.attach_documentation(expression, documentation),
        }

        // attach decorators to the same owner
        if let Some(decorators) = decorators {
            self.attach_expression_decorators(expression, decorators);
        }

        Ok(expression)
    }

    /// Parse one operator right operand within the current recursive-descent level.
    fn parse_expression_right_operand(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
        minimum_precedence: OperatorPrecedence,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // recover a missing operand at an enclosing expression boundary
        if self.peek_expression_slot_boundary() {
            return Ok(self.recover_missing_expression_here(NodeType::Expression));
        }

        // parse a destructuring assignment as one right associative operand
        if minimum_precedence == OperatorPrecedence::Assignment
            && self.peek_destructuring_assignment()
        {
            let start = self.mark_parse_start();

            return self.parse_destructuring_assignment(&start, position, stop);
        }

        // guard source nesting introduced by a leading separator
        if self.peek_is(TokenType::ElementwiseOr) {
            return self.parse_expression_at(position, stop, minimum_precedence);
        }

        self.parse_expression_after_descent(position, stop, minimum_precedence)
    }

    /// Parse one value operator tail within the current recursion depth.
    #[inline(never)]
    fn parse_expression_tail_after_descent(
        &mut self,
        mut left: LocalNodeId<Expression>,
        position: ExpressionPosition,
        stop: ExpressionStop,
        minimum_precedence: OperatorPrecedence,
    ) -> ParserResult<LocalNodeId<Expression>> {
        loop {
            // parse one conditional tail at its fixed precedence
            if self.peek_is(TokenType::Maybe)
                && !stop.has(ExpressionStop::CONDITIONAL_QUESTION)
                && OperatorPrecedence::Conditional > minimum_precedence
            {
                left = self.parse_conditional_expression(left, position, stop)?;

                continue;
            }

            // classify one infix operation owned by this expression level
            let Some(operator) =
                self.peek_expression_operator(left, position, stop, minimum_precedence)
            else {
                break;
            };
            let precedence = operator.precedence();

            // type relations transfer the complete tail to the type reducer
            if matches!(operator, ExpressionOperator::Type(_)) {
                let type_left = self.promote_expression_type(left)?;
                let type_stop = TypeStop::from(stop);
                let ty = self.parse_type_tail(type_left, TypePosition::Type, type_stop)?;
                left = self.insert_type_expression_value(ty);

                continue;
            }

            let range = self.peek_token().range();
            self.bump();

            // type-valued operations parse their complete right operand in type space
            if operator.has_type_operand() {
                let target_type = self.parse_type_or_recover_missing(
                    TypePosition::Type,
                    TypeStop::from(stop),
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
                    left,
                    operator,
                    range,
                    precedence,
                    position,
                    stop,
                    minimum_precedence,
                )?;

                continue;
            }

            // parse the complete right operand at this operator's binding power
            let right = self.parse_expression_right_operand(position.right(), stop, precedence)?;
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
        position: ExpressionPosition,
        stop: ExpressionStop,
        minimum_precedence: OperatorPrecedence,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let mut operations: SmallVec<[ExpressionInfix; 4]> = smallvec![ExpressionInfix {
            left,
            operator,
            range,
        }];

        loop {
            // parse operators stronger than this right associative run
            let right_position = position.right();
            let mut right_stop = stop;
            if matches!(operator, ExpressionOperator::Assign(_)) {
                right_stop = right_stop
                    .add(ExpressionStop::NEWLINE_CALL)
                    .remove(ExpressionStop::CONDITIONAL_QUESTION);
            }
            let right =
                self.parse_expression_right_operand(right_position, right_stop, precedence)?;

            // collect another operation at exactly this precedence
            if let Some(next) =
                self.peek_expression_operator(right, position, stop, minimum_precedence)
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
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let mut branches: SmallVec<[ConditionalExpressionBranch; 4]> = SmallVec::new();

        loop {
            // parse ? thenExpression
            let question = self.peek_token().range();
            self.bump();
            let then_stop = stop.add(ExpressionStop::CONDITIONAL_COLON);
            let then_expression = self.parse_expression_right_operand(
                position,
                then_stop,
                OperatorPrecedence::Lowest,
            )?;

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
            let else_stop = stop.add(ExpressionStop::CONDITIONAL_QUESTION);
            condition = self.parse_expression_right_operand(
                position,
                else_stop,
                OperatorPrecedence::Lowest,
            )?;
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
        position: ExpressionPosition,
        stop: ExpressionStop,
        minimum_precedence: OperatorPrecedence,
    ) -> Option<ExpressionOperator> {
        // classify the source token before evaluating contextual ownership
        let token_type = self.peek_token_type();
        let keyword = (token_type == TokenType::Identifier)
            .then(|| self.peek_keyword())
            .flatten();
        let operator = ExpressionOperator::from_token(token_type, keyword)?;

        if self.is_expression_operator_stopped(left, operator, position, stop) {
            return None;
        }

        if matches!(operator, ExpressionOperator::Range(_)) && self.peek_is_on_new_line() {
            return None;
        }

        (operator.precedence() > minimum_precedence).then_some(operator)
    }

    /// Parse an elementwise-or expression with a leading separator.
    fn parse_leading_or_expression(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let operator = BinaryOperator::ElementwiseOr;
        let precedence = operator.precedence();
        self.bump();

        let mut left = self.parse_expression_at(position.right(), stop, precedence)?;
        let mut has_binary = false;
        while self.peek_is(TokenType::ElementwiseOr) {
            let operator_range = self.peek_token_span().token.range();
            self.bump();
            let right = self.parse_expression_at(position.right(), stop, precedence)?;
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

    /// Return whether the current token belongs to an enclosing expression.
    fn is_expression_operator_stopped(
        &self,
        left: LocalNodeId<Expression>,
        operator: ExpressionOperator,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> bool {
        // leave generic closing angles to the enclosing argument list
        if stop.has(ExpressionStop::ANGLE_CLOSE)
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
        if position == ExpressionPosition::TypeQuery && self.peek_is_on_new_line() {
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

        // leave match continuation lines to the enclosing arm
        if stop.has(ExpressionStop::MATCH_ARM_LINE) && self.peek_is_on_new_line() {
            return true;
        }

        // keep a line-leading tree outside a completed statement
        if position.is_statement()
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
        position.is_statement()
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
        let condition_range = self.tree.get_source_extent(condition).range();
        let else_range = self.tree.get_source_extent(else_expression).range();
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
            NodeSpanType::Region(NodeSpanRegion::Alternate),
            colon_range,
        );

        expression
    }
}
