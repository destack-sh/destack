use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, OperatorDispatch, OperatorFailureReason, OperatorTerm,
    OperatorTermKind, Origin, TermId, TypeLiteralTerm, TypeOperand, TypeTerm,
};

/// Primitive numeric operand kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericKind {
    /// Integer literal or primitive.
    Integer,
    /// Floating-point literal or primitive.
    Float,
}

/// Solved numeric operand classification.
#[derive(Debug, Clone, PartialEq)]
struct NumericOperand {
    /// The operand kind.
    kind: NumericKind,
    /// The concrete primitive type, when the operand already has one.
    primitive: Option<dir::PrimitiveType>,
}

/// Matched builtin numeric binary operation.
#[derive(Debug, Clone, PartialEq)]
struct NumericBinaryMatch {
    /// The operator result.
    return_type: TypeOperand,
}

/// Builtin operator candidate.
struct BuiltinOperatorMatch {
    /// The operator result.
    return_type: TypeOperand,
}

/// Builtin operator decision before committing operand expectations.
enum BuiltinOperatorDecision {
    /// Builtin operator match is waiting for dependency.
    Pending(SmallVec<[Dependency; 2]>),
    /// Builtin operator match rejected the operands.
    Rejected(OperatorFailureReason),
    /// Builtin operator match accepted the operands.
    Accepted(BuiltinOperatorMatch),
}

impl CheckState<'_> {
    /// Select one builtin operator.
    pub(in crate::check) fn select_builtin_operator_dispatch(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<OperatorDispatch>> {
        let decision = match operator.kind {
            OperatorTermKind::Unary(kind) => self.decide_builtin_unary(kind, receiver, expected)?,
            OperatorTermKind::Binary(kind) => {
                self.decide_builtin_binary(origin, operator, kind, receiver, expected)?
            }
        };
        let Some(decision) = decision else {
            return Ok(None);
        };

        // preserve pending and rejection decisions without side effects
        let selection = match decision {
            BuiltinOperatorDecision::Pending(blockers) => {
                return Ok(Some(OperatorDispatch::Pending(blockers)));
            }
            BuiltinOperatorDecision::Rejected(reason) => {
                return Ok(Some(OperatorDispatch::NoMatch { reason }));
            }
            BuiltinOperatorDecision::Accepted(selection) => selection,
        };

        // require a closed expected result decision
        let return_type = selection.return_type;
        match self.decide_expected_result(return_type, expected)? {
            Answer::Ready(true) => {}
            Answer::Pending(blockers) => return Ok(Some(OperatorDispatch::Pending(blockers))),
            Answer::Ready(false) => return Ok(Some(Self::operator_rejected())),
        }

        Ok(Some(OperatorDispatch::Builtin { return_type }))
    }

    /// Decide one builtin unary operator case.
    fn decide_builtin_unary(
        &mut self,
        kind: dir::UnaryOperator,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<BuiltinOperatorDecision>> {
        let selection = match kind {
            dir::UnaryOperator::Not => {
                let target = self.boolean_type_operand();

                BuiltinOperatorMatch {
                    return_type: target,
                }
            }
            dir::UnaryOperator::PostIncrement
            | dir::UnaryOperator::PostDecrement
            | dir::UnaryOperator::PreIncrement
            | dir::UnaryOperator::PreDecrement
            | dir::UnaryOperator::Plus
            | dir::UnaryOperator::Negate
            | dir::UnaryOperator::ElementwiseNot => {
                let Some(numeric) = self.numeric_operand(receiver)? else {
                    return Ok(None);
                };
                if kind == dir::UnaryOperator::ElementwiseNot && numeric.kind == NumericKind::Float
                {
                    return Ok(None);
                }
                let target = self.unary_numeric_result_type(kind, &numeric, expected)?;

                BuiltinOperatorMatch {
                    return_type: target,
                }
            }
            dir::UnaryOperator::Void => BuiltinOperatorMatch {
                return_type: self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Void)),
            },
            _ => return Ok(None),
        };

        Ok(Some(BuiltinOperatorDecision::Accepted(selection)))
    }

    /// Decide one builtin binary operator case.
    fn decide_builtin_binary(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        kind: dir::BinaryOperator,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<BuiltinOperatorDecision>> {
        let Some(argument) = operator.argument else {
            return Ok(Some(BuiltinOperatorDecision::Rejected(
                OperatorFailureReason::NoMatch,
            )));
        };
        let Answer::Ready(argument_type) = self.reduce_type_operand(origin, argument)? else {
            return Ok(Some(BuiltinOperatorDecision::Pending(
                argument.dependencies(self),
            )));
        };

        // choose strict identity comparison
        if kind.is_strict_equality() {
            let is_compatible = self.decide_strict_equality(origin, receiver, argument_type)?;
            let target = self.boolean_type_operand();

            if let Answer::Pending(blockers) = is_compatible {
                return Ok(Some(BuiltinOperatorDecision::Pending(blockers)));
            }
            if is_compatible == Answer::Ready(true) {
                let selection = BuiltinOperatorMatch {
                    return_type: target,
                };

                return Ok(Some(BuiltinOperatorDecision::Accepted(selection)));
            }

            return Ok(Some(BuiltinOperatorDecision::Rejected(
                OperatorFailureReason::InvalidStrictEquality,
            )));
        }

        // choose builtin value equality
        if kind.is_value_equality() {
            let decision = self.decide_builtin_value_equality(origin, receiver, argument_type)?;

            if let Answer::Pending(blockers) = decision {
                return Ok(Some(BuiltinOperatorDecision::Pending(blockers)));
            }
            if decision == Answer::Ready(true) {
                let target = self.boolean_type_operand();
                let selection = BuiltinOperatorMatch {
                    return_type: target,
                };

                return Ok(Some(BuiltinOperatorDecision::Accepted(selection)));
            }
        }

        // choose primitive numeric operators
        if let Some(left) = self.numeric_operand(receiver)?
            && let Some(right) = self.numeric_operand(argument_type)?
            && let Some(selection) = self.select_numeric_binary(kind, &left, &right, expected)?
        {
            let selection = BuiltinOperatorMatch {
                return_type: selection.return_type,
            };

            return Ok(Some(BuiltinOperatorDecision::Accepted(selection)));
        }

        // choose boolean logical operators
        if kind.is_logical_boolean()
            && self.is_boolean_assignable(receiver)?
            && self.is_boolean_assignable(argument_type)?
        {
            let target = self.boolean_type_operand();
            let selection = BuiltinOperatorMatch {
                return_type: target,
            };

            return Ok(Some(BuiltinOperatorDecision::Accepted(selection)));
        }

        Ok(None)
    }

    /// Classify one solved type operand as a builtin numeric operand.
    fn numeric_operand(&mut self, operand: TypeOperand) -> CompilerResult<Option<NumericOperand>> {
        let Some(term) = self.type_operand_term_id(operand)? else {
            return Ok(None);
        };
        let operand = Self::numeric_type_term(self.inference.term(term));

        Ok(operand)
    }

    /// Classify one solved type term as a builtin numeric operand.
    fn numeric_type_term(term: &TypeTerm) -> Option<NumericOperand> {
        let operand = match term {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(dir::ScalarLiteral::Integer(_))) => {
                NumericOperand {
                    kind: NumericKind::Integer,
                    primitive: None,
                }
            }
            TypeTerm::Literal(TypeLiteralTerm::Scalar(dir::ScalarLiteral::Float(_))) => {
                NumericOperand {
                    kind: NumericKind::Float,
                    primitive: None,
                }
            }
            TypeTerm::Literal(TypeLiteralTerm::Primitive(primitive)) => {
                let Some(kind) = Self::numeric_primitive(*primitive) else {
                    return None;
                };

                NumericOperand {
                    kind,
                    primitive: Some(*primitive),
                }
            }
            _ => return None,
        };

        Some(operand)
    }

    /// Classify one primitive as a builtin numeric primitive.
    fn numeric_primitive(primitive: dir::PrimitiveType) -> Option<NumericKind> {
        let numeric = match primitive {
            dir::PrimitiveType::Integer(_) => NumericKind::Integer,
            dir::PrimitiveType::Float(_) => NumericKind::Float,
            _ => return None,
        };

        Some(numeric)
    }

    /// Return the builtin result type for one unary numeric operator.
    fn unary_numeric_result_type(
        &mut self,
        operator: dir::UnaryOperator,
        operand: &NumericOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<TypeOperand> {
        if let Some(primitive) = operand.primitive {
            let ty =
                self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Primitive(primitive)));

            return Ok(ty);
        }
        if let Some(expected) = self.expected_numeric_type(expected)?
            && (operator != dir::UnaryOperator::ElementwiseNot
                || self
                    .numeric_operand(expected)?
                    .is_some_and(|numeric| numeric.kind != NumericKind::Float))
        {
            return Ok(expected);
        }
        let literal = if operator == dir::UnaryOperator::ElementwiseNot {
            TypeLiteralTerm::integer()
        } else {
            TypeLiteralTerm::number()
        };

        Ok(self.type_term_operand(TypeTerm::Literal(literal)))
    }

    /// Select one primitive numeric binary operator.
    fn select_numeric_binary(
        &mut self,
        operator: dir::BinaryOperator,
        left: &NumericOperand,
        right: &NumericOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<NumericBinaryMatch>> {
        if !operator.has_numeric_builtin() {
            return Ok(None);
        }
        if operator.requires_integer_numeric_operands()
            && (left.kind == NumericKind::Float || right.kind == NumericKind::Float)
        {
            return Ok(None);
        }
        let Some(operand_type) =
            self.numeric_binary_operand_type(operator, left, right, expected)?
        else {
            return Ok(None);
        };
        let return_type = if operator.returns_boolean_for_numeric_operands() {
            self.boolean_type_operand()
        } else {
            operand_type
        };

        Ok(Some(NumericBinaryMatch { return_type }))
    }

    /// Return the operand type for one numeric binary operator.
    fn numeric_binary_operand_type(
        &mut self,
        operator: dir::BinaryOperator,
        left: &NumericOperand,
        right: &NumericOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<TypeOperand>> {
        if let Some(primitive) = Self::selected_numeric_primitive(left, right) {
            let ty =
                self.type_term_operand(TypeTerm::Literal(TypeLiteralTerm::Primitive(primitive)));

            return Ok(Some(ty));
        }
        if left.primitive.is_some() || right.primitive.is_some() {
            return Ok(None);
        }
        if !operator.returns_boolean_for_numeric_operands()
            && let Some(expected) = self.expected_numeric_type(expected)?
        {
            return Ok(Some(expected));
        }
        let literal = if operator.requires_integer_numeric_operands() {
            TypeLiteralTerm::integer()
        } else {
            TypeLiteralTerm::number()
        };

        Ok(Some(self.type_term_operand(TypeTerm::Literal(literal))))
    }

    /// Return an expected type when it is primitive numeric.
    fn expected_numeric_type(
        &mut self,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(expected) = expected else {
            return Ok(None);
        };
        let Some(term) = self.type_operand_term_id(expected)? else {
            return Ok(None);
        };
        if Self::numeric_type_term(self.inference.term(term)).is_none() {
            return Ok(None);
        }

        Ok(Some(term.into()))
    }

    /// Return the concrete primitive selected by numeric operands.
    fn selected_numeric_primitive(
        left: &NumericOperand,
        right: &NumericOperand,
    ) -> Option<dir::PrimitiveType> {
        match (left.primitive, right.primitive) {
            (Some(left), Some(right)) if left == right => Some(left),
            (Some(left), None) => Some(left),
            (None, Some(right)) => Some(right),
            _ => None,
        }
    }

    /// Decide whether strict equality can compare both operands.
    fn decide_strict_equality(
        &mut self,
        origin: Origin,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<Answer<bool>> {
        let Answer::Ready(left) = self.equality_term(origin, left)? else {
            return Ok(Answer::pending(left.dependencies(self)));
        };
        let Answer::Ready(right) = self.equality_term(origin, right)? else {
            return Ok(Answer::pending(right.dependencies(self)));
        };
        let left = self.decide_strict_identity(left)?;
        let right = self.decide_strict_identity(right)?;

        Ok(left.and(right))
    }

    /// Return one type term used by builtin equality decisions.
    fn equality_term(
        &mut self,
        origin: Origin,
        operand: TypeOperand,
    ) -> CompilerResult<Answer<TermId<TypeTerm>>> {
        let Answer::Ready(operand) = self.reduce_type_operand(origin, operand)? else {
            return Ok(Answer::pending(operand.dependencies(self)));
        };
        let Some(term) = self.type_operand_term_id(operand)? else {
            return Ok(Answer::pending(operand.dependencies(self)));
        };

        Ok(Answer::Ready(term))
    }

    /// Decide whether builtin value equality can compare both operands.
    fn decide_builtin_value_equality(
        &mut self,
        origin: Origin,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<Answer<bool>> {
        let Answer::Ready(left) = self.equality_term(origin, left)? else {
            return Ok(Answer::pending(left.dependencies(self)));
        };
        let Answer::Ready(right) = self.equality_term(origin, right)? else {
            return Ok(Answer::pending(right.dependencies(self)));
        };

        self.decide_value_equality(left, right)
    }

    /// Decide whether builtin value equality can compare both terms.
    fn decide_value_equality(
        &mut self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let nullish = self.decide_nullish_equality(left, right)?;
        if nullish != Answer::Ready(false) {
            return Ok(nullish);
        }

        // unwrap value forms
        if let TypeTerm::Form { payload, .. } = self.inference.term(left) {
            let payload = *payload;
            let Some(left) = self.type_operand_term_id(payload)? else {
                return Ok(Answer::pending(payload.dependencies(self)));
            };

            return self.decide_value_equality(left, right);
        }
        if let TypeTerm::Form { payload, .. } = self.inference.term(right) {
            let payload = *payload;
            let Some(right) = self.type_operand_term_id(payload)? else {
                return Ok(Answer::pending(payload.dependencies(self)));
            };

            return self.decide_value_equality(left, right);
        }

        // compare a union when any member can use builtin equality
        if let TypeTerm::Union { elements: _ } = self.inference.term(left) {
            return self.decide_union_value_equality(left, right);
        }
        if let TypeTerm::Union { elements: _ } = self.inference.term(right) {
            return self.decide_union_value_equality(right, left);
        }

        // compare builtin literal families
        let decision = match (self.inference.term(left), self.inference.term(right)) {
            (TypeTerm::Literal(left), TypeTerm::Literal(right)) => {
                Answer::from(left.has_builtin_value_equality_with(right))
            }
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide whether any union member can compare by builtin value equality.
    fn decide_union_value_equality(
        &mut self,
        union: TermId<TypeTerm>,
        other: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let TypeTerm::Union { elements } = self.inference.term(union) else {
            return Ok(Answer::Ready(false));
        };
        let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
        let mut decision = Answer::Ready(false);

        // accept once any known member can compare
        for element in elements {
            let Some(element) = self.type_operand_term_id(element)? else {
                decision = decision.or(Answer::pending(element.dependencies(self)));
                continue;
            };
            let element = self.decide_value_equality(element, other)?;

            decision = decision.or(element);
        }

        Ok(decision)
    }

    /// Decide whether builtin value equality compares one nullish domain.
    fn decide_nullish_equality(
        &mut self,
        left: TermId<TypeTerm>,
        right: TermId<TypeTerm>,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match (self.inference.term(left), self.inference.term(right)) {
            (TypeTerm::Literal(left_literal), TypeTerm::Literal(right_literal)) => {
                Answer::from(left_literal.is_same_nullish_literal(right_literal))
            }
            (TypeTerm::Literal(left_literal), _) if left_literal.is_nullish() => {
                let left_literal = *left_literal;

                self.decide_term_contains_nullish(right, &left_literal)?
            }
            (_, TypeTerm::Literal(right_literal)) if right_literal.is_nullish() => {
                let right_literal = *right_literal;

                self.decide_term_contains_nullish(left, &right_literal)?
            }
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide whether one type term can contain one nullish literal.
    fn decide_term_contains_nullish(
        &mut self,
        term: TermId<TypeTerm>,
        nullish: &TypeLiteralTerm,
    ) -> CompilerResult<Answer<bool>> {
        let decision = match self.inference.term(term) {
            TypeTerm::Literal(literal) => Answer::from(literal.is_same_nullish_literal(nullish)),
            TypeTerm::Form { payload, .. } => {
                let payload = *payload;
                let Some(term) = self.type_operand_term_id(payload)? else {
                    return Ok(Answer::pending(payload.dependencies(self)));
                };

                return self.decide_term_contains_nullish(term, nullish);
            }
            TypeTerm::Union { elements: _ } => {
                let TypeTerm::Union { elements } = self.inference.term(term) else {
                    return Ok(Answer::Ready(false));
                };
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut decision = Answer::Ready(false);

                for element in elements {
                    let Some(term) = self.type_operand_term_id(element)? else {
                        decision = decision.or(Answer::pending(element.dependencies(self)));
                        continue;
                    };
                    let element = self.decide_term_contains_nullish(term, nullish)?;

                    decision = decision.or(element);
                }

                decision
            }
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Decide whether one type carries scalar or reference identity.
    fn decide_strict_identity(&mut self, term: TermId<TypeTerm>) -> CompilerResult<Answer<bool>> {
        let decision = match self.inference.term(term) {
            TypeTerm::Literal(literal) => Answer::from(literal.has_strict_identity()),
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments: _,
            } => Answer::from(self.symbol_kind(*symbol) == dir::SymbolKind::Class),
            TypeTerm::Form { payload, .. } => {
                let payload = *payload;
                let Some(term) = self.type_operand_term_id(payload)? else {
                    return Ok(Answer::pending(payload.dependencies(self)));
                };

                return self.decide_strict_identity(term);
            }
            TypeTerm::Union { elements: _ } => {
                let TypeTerm::Union { elements } = self.inference.term(term) else {
                    return Ok(Answer::Ready(false));
                };
                let elements = elements.iter().copied().collect::<SmallVec<[_; 4]>>();
                let mut decision = Answer::Ready(true);

                for element in elements {
                    let Some(element) = self.type_operand_term_id(element)? else {
                        decision = decision.and(Answer::pending(element.dependencies(self)));
                        continue;
                    };
                    let element = self.decide_strict_identity(element)?;

                    decision = decision.and(element);
                }

                decision
            }
            TypeTerm::Function(_) => Answer::Ready(true),
            _ => Answer::Ready(false),
        };

        Ok(decision)
    }

    /// Return the builtin boolean type term.
    pub(in crate::check) fn boolean_type_term() -> TypeTerm {
        TypeTerm::Literal(TypeLiteralTerm::boolean())
    }

    /// Return the builtin boolean type operand.
    fn boolean_type_operand(&mut self) -> TypeOperand {
        self.type_term_operand(Self::boolean_type_term())
    }
}
