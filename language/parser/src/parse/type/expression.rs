use crate::parse::context::{
    ConditionalTypeContext, DecoratorContext, InferExtends, TypeContext, TypeStops,
};
use crate::parse::r#type::operator::{TypeOperator, TypeRelation};
use crate::{Parser, ParserResult};
use destack_dir::{LocalNodeId, NodeType, OperatorPrecedence, RangeEnd, TokenType, TypeExpression};
use destack_source::{ByteRange, NodeSpanBoundary, NodeSpanRegion, NodeSpanType};
use smallvec::SmallVec;

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
    /// ```ds
    /// Result<Value, Error> | none
    /// ```
    pub(crate) fn parse_type(
        &mut self,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.with_recursive_descent(NodeType::TypeExpression, |parser| {
            parser.parse_type_after_descent(context)
        })
    }

    /// Parse one type expression after entering recursive descent state.
    fn parse_type_after_descent(
        &mut self,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // parse decorators only at their owning type level
        let decorators =
            if context.decorator == DecoratorContext::None && self.peek_is(TokenType::At) {
                Some(self.parse_decorators(context.function))
            } else {
                None
            };

        // parse the operand and iterative operator tail
        let first = self.parse_type_operand(context)?;
        let ty = self.parse_type_tail_after_descent(first, context)?;

        // attach decorators after the complete type is known
        if let Some(decorators) = decorators {
            self.attach_decorators(ty.id, decorators);
        }

        Ok(ty)
    }

    /// Parse the operator tail of one already parsed type head.
    pub(in crate::parse) fn parse_type_tail(
        &mut self,
        first: LocalNodeId<TypeExpression>,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.with_recursive_descent(NodeType::TypeExpression, |parser| {
            parser.parse_type_tail_after_descent(first, context)
        })
    }

    /// Parse one type operator tail after entering recursive descent state.
    fn parse_type_tail_after_descent(
        &mut self,
        mut left: LocalNodeId<TypeExpression>,
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // reduce infix operations owned by this type level
        while let Some(operator) = self.peek_type_operator(context) {
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
            let right_context = if matches!(operator, TypeOperator::Relation(TypeRelation::Extends))
            {
                TypeContext {
                    infer_extends: InferExtends::Constraint,
                    minimum_precedence: precedence,
                    ..context
                }
            } else {
                context.right(precedence)
            };
            let right =
                self.parse_type_or_recover_missing(right_context, NodeType::TypeExpression)?;

            // promote an extends relation followed by question into a conditional type
            let is_conditional = matches!(operator, TypeOperator::Relation(TypeRelation::Extends))
                && context.conditional == ConditionalTypeContext::Allowed
                && OperatorPrecedence::Conditional > context.minimum_precedence
                && self.peek_is(TokenType::Maybe);
            if is_conditional {
                left = self.parse_conditional_type_ladder(left, right, range, context)?;

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
        context: TypeContext,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut branches: SmallVec<[ConditionalTypeBranch; 4]> = SmallVec::new();

        loop {
            // parse ? thenType
            let question = self.peek_token().range();
            self.bump();
            let then_type = self.parse_type_or_recover_missing(
                TypeContext {
                    stops: context.stops.with(TypeStops::CONDITIONAL_COLON),
                    minimum_precedence: OperatorPrecedence::Lowest,
                    ..context
                },
                NodeType::TypeExpression,
            )?;

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
            left = self.parse_type_or_recover_missing(
                context.right(OperatorPrecedence::TypeRelation),
                NodeType::TypeExpression,
            )?;
            let next = self.peek_type_operator(context);
            if !matches!(next, Some(TypeOperator::Relation(TypeRelation::Extends))) {
                break;
            }

            // parse the next extends relation without recursive conditional depth
            extends_range = self.peek_token().range();
            self.bump();
            extends_type = self.parse_type_or_recover_missing(
                TypeContext {
                    infer_extends: InferExtends::Constraint,
                    minimum_precedence: OperatorPrecedence::TypeRelation,
                    ..context
                },
                NodeType::TypeExpression,
            )?;
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
    fn peek_type_operator(&self, context: TypeContext) -> Option<TypeOperator> {
        // classify the source token before evaluating contextual ownership
        let token_type = self.peek_token_type();
        let keyword = (token_type == TokenType::Identifier)
            .then(|| self.peek_keyword())
            .flatten();
        let operator = TypeOperator::from_token(token_type, keyword)?;

        if Self::is_type_operator_stopped(operator, context) {
            return None;
        }

        if matches!(operator, TypeOperator::Range(_)) && self.peek_is_on_new_line() {
            return None;
        }

        (operator.precedence() > context.minimum_precedence).then_some(operator)
    }

    /// Return whether the current token belongs to an enclosing type grammar.
    fn is_type_operator_stopped(operator: TypeOperator, context: TypeContext) -> bool {
        // leave forbidden conditional relations to the enclosing grammar
        if context.conditional == ConditionalTypeContext::Forbidden
            && matches!(operator, TypeOperator::Relation(_))
        {
            return true;
        }

        // leave an implements clause to the enclosing declaration
        if context.stops.contains(TypeStops::IMPLEMENTS)
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
            NodeSpanType::Region(NodeSpanRegion::Clause),
            colon_range,
        );

        ty
    }
}
