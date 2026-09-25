use crate::parse::ExpressionStop;
use crate::parse::r#type::operator::{TypeOperator, TypeRelation};
use crate::{Parser, ParserResult};
use smallvec::SmallVec;
use tspp_dir::{LocalNodeId, NodeType, OperatorPrecedence, RangeEnd, TokenType, TypeExpression};
use tspp_source::{ByteRange, NodeSpanBoundary, NodeSpanRegion, NodeSpanType};

/// The source position of one type expression.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) enum TypePosition {
    /// Parse an ordinary type expression.
    #[default]
    Type,
    /// Parse an arrow return annotation.
    ArrowReturn,
}

/// Token ownership inherited by type infix operands.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub(crate) struct TypeStop(u8);

impl TypeStop {
    /// A colon owned by an enclosing conditional type.
    pub(crate) const CONDITIONAL_COLON: Self = Self(1 << 0);
    /// An `implements` token owned by an enclosing heritage clause.
    pub(crate) const IMPLEMENTS: Self = Self(1 << 1);
    /// An angle close owned by an enclosing generic argument list.
    pub(crate) const ANGLE_CLOSE: Self = Self(1 << 2);
    /// A relation operator owned by the enclosing production.
    pub(crate) const RELATION: Self = Self(1 << 3);
    /// An `extends` relation owned by an inferred type.
    pub(crate) const INFER_CONSTRAINT: Self = Self(1 << 4);

    /// Add one enclosing token.
    pub(crate) const fn add(self, stop: Self) -> Self {
        Self(self.0 | stop.0)
    }

    /// Return whether one token belongs to the enclosing type.
    pub(crate) const fn has(self, stop: Self) -> bool {
        self.0 & stop.0 != 0
    }

    /// Return the stops inherited through explicit type nesting.
    pub(crate) const fn nest(self) -> Self {
        Self(self.0 & Self::RELATION.0)
    }
}

impl From<ExpressionStop> for TypeStop {
    /// Preserve a generic close across value and type parsing.
    fn from(stop: ExpressionStop) -> Self {
        if stop.has(ExpressionStop::ANGLE_CLOSE) {
            TypeStop::ANGLE_CLOSE
        } else {
            TypeStop::default()
        }
    }
}

/// One conditional type branch awaiting its final false branch.
struct ConditionalTypeBranch {
    /// The checked type.
    check: LocalNodeId<TypeExpression>,
    /// The extends constraint.
    extends_type: LocalNodeId<TypeExpression>,
    /// The true branch type.
    then_type: LocalNodeId<TypeExpression>,
    /// The extends keyword source range.
    extends: ByteRange,
    /// The question mark source range.
    question: ByteRange,
    /// The colon source range or recovery anchor.
    colon: ByteRange,
}

impl Parser {
    /// Parse one complete type expression.
    ///
    /// Examples:
    /// ```tspp
    /// Result<Value, Error> | none
    /// ```
    pub(crate) fn parse_type(
        &mut self,
        position: TypePosition,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.parse_type_at(position, stop, OperatorPrecedence::Lowest)
    }

    /// Parse one type expression at the given minimum precedence.
    pub(in crate::parse::r#type) fn parse_type_at(
        &mut self,
        position: TypePosition,
        stop: TypeStop,
        minimum_precedence: OperatorPrecedence,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.with_recursive_descent(NodeType::TypeExpression, |parser| {
            parser.parse_type_after_descent(position, stop, minimum_precedence)
        })
    }

    /// Parse one type expression after checking the recursion depth.
    fn parse_type_after_descent(
        &mut self,
        position: TypePosition,
        stop: TypeStop,
        minimum_precedence: OperatorPrecedence,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let documentation = self.parse_documentation();

        // parse decorators at their owning type level
        let decorators = self.peek_is(TokenType::At).then(|| self.parse_decorators());

        // parse the operand and iterative operator tail
        let first = self.parse_type_operand(position, stop)?;
        let ty = self.parse_type_tail_after_descent(first, position, stop, minimum_precedence)?;

        // attach documentation and decorators after the complete type is known
        if !matches!(
            self.tree.get(ty),
            TypeExpression::Missing | TypeExpression::Error
        ) {
            self.attach_documentation(ty, documentation);
        }
        if let Some(decorators) = decorators {
            self.attach_decorators(ty.id, decorators);
        }

        Ok(ty)
    }

    /// Parse the operator tail of one already parsed type head.
    pub(in crate::parse) fn parse_type_tail(
        &mut self,
        first: LocalNodeId<TypeExpression>,
        position: TypePosition,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.with_recursive_descent(NodeType::TypeExpression, |parser| {
            parser.parse_type_tail_after_descent(first, position, stop, OperatorPrecedence::Lowest)
        })
    }

    /// Parse one type operator tail within the current recursion depth.
    fn parse_type_tail_after_descent(
        &mut self,
        mut left: LocalNodeId<TypeExpression>,
        position: TypePosition,
        stop: TypeStop,
        minimum_precedence: OperatorPrecedence,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // reduce infix operations owned by this type level
        while let Some(operator) = self.peek_type_operator(stop, minimum_precedence) {
            let precedence = operator.precedence();
            let range = self.peek_token().range();
            self.bump();

            // open ranges may omit their right endpoint
            if let TypeOperator::Range(end_kind) = operator
                && self.peek_type_range_end_omitted()
            {
                left = self.insert_type_range_with_omitted_end(left, end_kind, range);

                break;
            }

            // parse the complete right operand at this operator's binding power
            let right_stop = if matches!(operator, TypeOperator::Relation(TypeRelation::Extends)) {
                stop.add(TypeStop::INFER_CONSTRAINT)
            } else {
                stop
            };
            let right = if self.peek_type_expression_recovery_boundary() {
                self.recover_missing_type_expression_here(NodeType::TypeExpression)
            } else {
                self.parse_type_at(position, right_stop, precedence)?
            };

            // promote an extends relation followed by question into a conditional type
            let is_conditional = matches!(operator, TypeOperator::Relation(TypeRelation::Extends))
                && !stop.has(TypeStop::RELATION)
                && OperatorPrecedence::Conditional > minimum_precedence
                && self.peek_is(TokenType::Maybe);
            if is_conditional {
                left = self.parse_conditional_type_ladder(left, right, range, position, stop)?;

                continue;
            }

            left = self.insert_type_infix(left, operator, range, right)?;
        }

        Ok(left)
    }

    /// Parse one right associative conditional type ladder.
    fn parse_conditional_type_ladder(
        &mut self,
        mut left: LocalNodeId<TypeExpression>,
        mut extends_type: LocalNodeId<TypeExpression>,
        mut extends_range: ByteRange,
        position: TypePosition,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut branches: SmallVec<[ConditionalTypeBranch; 4]> = SmallVec::new();

        loop {
            // parse ? thenType
            let question = self.peek_token().range();
            self.bump();
            let then_stop = stop.add(TypeStop::CONDITIONAL_COLON);
            let then_type = if self.peek_type_expression_recovery_boundary() {
                self.recover_missing_type_expression_here(NodeType::TypeExpression)
            } else {
                self.parse_type_at(position, then_stop, OperatorPrecedence::Lowest)?
            };

            // parse or recover the conditional colon
            let colon =
                self.eat_token_range_or_recover_missing(TokenType::Colon, NodeType::TypeExpression);
            branches.push(ConditionalTypeBranch {
                check: left,
                extends_type,
                then_type,
                extends: extends_range,
                question,
                colon,
            });

            // parse the next fallback through operators stronger than relations
            left = if self.peek_type_expression_recovery_boundary() {
                self.recover_missing_type_expression_here(NodeType::TypeExpression)
            } else {
                self.parse_type_at(position, stop, OperatorPrecedence::TypeRelation)?
            };
            let next = self.peek_type_operator(stop, OperatorPrecedence::Lowest);
            if !matches!(next, Some(TypeOperator::Relation(TypeRelation::Extends))) {
                break;
            }

            // parse the next extends relation without recursive conditional depth
            extends_range = self.peek_token().range();
            self.bump();
            let extends_stop = stop.add(TypeStop::INFER_CONSTRAINT);
            extends_type = if self.peek_type_expression_recovery_boundary() {
                self.recover_missing_type_expression_here(NodeType::TypeExpression)
            } else {
                self.parse_type_at(position, extends_stop, OperatorPrecedence::TypeRelation)?
            };
            if !self.peek_is(TokenType::Maybe) {
                left = self.insert_type_infix(
                    left,
                    TypeOperator::Relation(TypeRelation::Extends),
                    extends_range,
                    extends_type,
                )?;

                break;
            }
        }

        // fold the conditional ladder from its final fallback
        let ty = branches.into_iter().rev().fold(left, |else_type, branch| {
            self.insert_conditional_type(
                branch.check,
                branch.extends_type,
                branch.then_type,
                else_type,
                branch.extends,
                branch.question,
                branch.colon,
            )
        });

        Ok(ty)
    }

    /// Return the current infix operation owned by one type expression.
    fn peek_type_operator(
        &self,
        stop: TypeStop,
        minimum_precedence: OperatorPrecedence,
    ) -> Option<TypeOperator> {
        // classify the source token before evaluating contextual ownership
        let token_type = self.peek_token_type();
        let keyword = (token_type == TokenType::Identifier)
            .then(|| self.peek_keyword())
            .flatten();
        let operator = TypeOperator::from_token(token_type, keyword)?;

        if Self::is_type_operator_stopped(operator, stop) {
            return None;
        }

        if matches!(operator, TypeOperator::Range(_)) && self.peek_is_on_new_line() {
            return None;
        }

        (operator.precedence() > minimum_precedence).then_some(operator)
    }

    /// Return whether the current token belongs to the enclosing production.
    fn is_type_operator_stopped(operator: TypeOperator, stop: TypeStop) -> bool {
        // leave forbidden conditional relations to the enclosing production
        if stop.has(TypeStop::RELATION) && matches!(operator, TypeOperator::Relation(_)) {
            return true;
        }

        // leave an implements clause to the enclosing declaration
        if stop.has(TypeStop::IMPLEMENTS)
            && matches!(operator, TypeOperator::Relation(TypeRelation::Implements))
        {
            return true;
        }

        false
    }

    /// Insert one range type whose source omits the right endpoint.
    fn insert_type_range_with_omitted_end(
        &mut self,
        left: LocalNodeId<TypeExpression>,
        end_kind: RangeEnd,
        operator_range: ByteRange,
    ) -> LocalNodeId<TypeExpression> {
        let end = if end_kind == RangeEnd::Open {
            None
        } else {
            Some(self.recover_missing_type_expression_here(NodeType::TypeExpression))
        };
        let left_range = self.tree.get_range(left);
        let source_range = ByteRange {
            start: left_range.start,
            end: operator_range.end,
        };
        let ty = self.insert_node(
            TypeExpression::Range {
                start: Some(left),
                end,
                end_kind,
            },
            source_range,
        );
        self.tree.set_main_range(ty, operator_range);

        ty
    }

    /// Return whether the current source position omits a range endpoint.
    pub(in crate::parse::r#type) fn peek_type_range_end_omitted(&self) -> bool {
        self.peek_is_on_new_line() || self.peek_type_expression_recovery_boundary()
    }

    /// Insert one conditional type.
    fn insert_conditional_type(
        &mut self,
        left: LocalNodeId<TypeExpression>,
        extends_type: LocalNodeId<TypeExpression>,
        then_type: LocalNodeId<TypeExpression>,
        else_type: LocalNodeId<TypeExpression>,
        extends_range: ByteRange,
        question_range: ByteRange,
        colon_range: ByteRange,
    ) -> LocalNodeId<TypeExpression> {
        let left_range = self.tree.get_range(left);
        let else_range = self.tree.get_range(else_type);
        let source_range = ByteRange {
            start: left_range.start,
            end: else_range.end,
        };
        let ty = self.insert_node(
            TypeExpression::Conditional {
                left,
                extends_type,
                then_type,
                else_type,
            },
            source_range,
        );
        self.tree.set_main_range(ty, extends_range);
        self.tree.set_side_range(
            ty,
            NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
            question_range,
        );
        self.tree.set_side_range(
            ty,
            NodeSpanType::Region(NodeSpanRegion::Alternate),
            colon_range,
        );

        ty
    }
}
