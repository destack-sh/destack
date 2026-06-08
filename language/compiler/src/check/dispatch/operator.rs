use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallableDispatch, CallableSignature, CallableTarget, CheckState, Decision, FunctionTerm,
    GenericArgument, MemberCandidate, MemberLookup, MemberProtocol, OperatorExpressionResult,
    OperatorFailureReason, OperatorProtocol, OperatorProtocolArgument, OperatorTerm,
    OperatorTermKind, Origin, StaticTerm, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
    binary_operator_protocols, unary_operator_protocols,
};

/// Transient operator selection while reducing an operator term.
pub(in crate::check) enum OperatorCandidateDispatch {
    /// OperatorTerm resolution is waiting for solver input.
    Pending,
    /// No operator candidate accepts the operands.
    NoMatch {
        /// Why operator resolution failed.
        reason: OperatorFailureReason,
    },
    /// Builtin operator behavior resolved.
    Builtin {
        /// The resolved builtin return type.
        return_type: TypeTerm,
    },
    /// One symbol-backed operator method resolved.
    Method {
        /// The resolved operator method symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved function signature.
        function: FunctionTerm,
        /// The operator expression result.
        return_type: TypeTerm,
    },
}

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
    return_type: TypeTerm,
}

/// Builtin operator candidate.
struct BuiltinOperatorMatch {
    /// The operator result.
    return_type: TypeTerm,
}

/// Builtin operator decision before committing operand expectations.
enum BuiltinOperatorDecision {
    /// Builtin operator match is waiting for solver input.
    Pending,
    /// Builtin operator match rejected the operands.
    Rejected(OperatorFailureReason),
    /// Builtin operator match accepted the operands.
    Accepted(BuiltinOperatorMatch),
}

impl CheckState<'_> {
    /// Select one runtime operator from builtin and method candidates.
    pub(in crate::check) fn select_operator_candidate(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<OperatorCandidateDispatch> {
        let Some(receiver) = self.reduce_type_operand(origin, operator.receiver)? else {
            return Ok(OperatorCandidateDispatch::Pending);
        };

        // choose primitive operators
        if let Some(result) =
            self.select_builtin_operator_candidate(origin, operator, receiver, expected)?
        {
            return Ok(result);
        }

        // choose operator method overload
        if let Some(result) =
            self.select_operator_method_candidate(origin, operator, receiver, expected)?
        {
            return Ok(result);
        }

        Ok(Self::operator_no_match())
    }

    /// Return the generic no-match operator result.
    fn operator_no_match() -> OperatorCandidateDispatch {
        OperatorCandidateDispatch::NoMatch {
            reason: OperatorFailureReason::NoMatch,
        }
    }

    /// Select one builtin operator case.
    fn select_builtin_operator_candidate(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<OperatorCandidateDispatch>> {
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
            BuiltinOperatorDecision::Pending => {
                return Ok(Some(OperatorCandidateDispatch::Pending));
            }
            BuiltinOperatorDecision::Rejected(reason) => {
                return Ok(Some(OperatorCandidateDispatch::NoMatch { reason }));
            }
            BuiltinOperatorDecision::Accepted(selection) => selection,
        };

        // reject result mismatch before committing operand expectations
        if self.decide_expected_result(&selection.return_type, expected)? == Decision::No {
            return Ok(Some(Self::operator_no_match()));
        }
        Ok(Some(OperatorCandidateDispatch::Builtin {
            return_type: selection.return_type,
        }))
    }

    /// Decide one builtin unary operator case.
    fn decide_builtin_unary(
        &self,
        kind: dir::UnaryOperator,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<BuiltinOperatorDecision>> {
        let selection = match kind {
            dir::UnaryOperator::Not => {
                let target = Self::boolean_type_term();

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
                return_type: TypeTerm::Literal(TypeLiteralTerm::Void),
            },
            dir::UnaryOperator::Typeof => BuiltinOperatorMatch {
                return_type: TypeTerm::Literal(TypeLiteralTerm::Primitive(
                    dir::PrimitiveType::String,
                )),
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
        let Some(argument_type) = self.reduce_type_operand(origin, argument)? else {
            return Ok(Some(BuiltinOperatorDecision::Pending));
        };

        // choose strict identity comparison
        if kind.is_strict_equality() {
            let is_compatible = self.strict_equality_compatible(receiver, argument_type)?;
            let target = Self::boolean_type_term();

            if is_compatible && self.decide_expected_result(&target, expected)? != Decision::No {
                let selection = BuiltinOperatorMatch {
                    return_type: target,
                };

                return Ok(Some(BuiltinOperatorDecision::Accepted(selection)));
            }

            return Ok(Some(BuiltinOperatorDecision::Rejected(
                OperatorFailureReason::InvalidStrictEquality,
            )));
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
            let target = Self::boolean_type_term();
            let selection = BuiltinOperatorMatch {
                return_type: target,
            };

            return Ok(Some(BuiltinOperatorDecision::Accepted(selection)));
        }

        Ok(None)
    }

    /// Select one operator method.
    fn select_operator_method_candidate(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<OperatorCandidateDispatch>> {
        let protocols = operator.protocols();
        if protocols.is_empty() {
            return Ok(None);
        }
        let mut pending_protocol = None;
        let mut pending_count = 0;
        // choose the first protocol candidate that resolves
        for (index, protocol) in protocols.iter().cloned().enumerate() {
            let probe = self.inference.begin_probe();
            let result = self.select_operator_protocol_method_candidate(
                origin, operator, receiver, expected, protocol,
            )?;
            match result {
                OperatorCandidateDispatch::Method { .. }
                | OperatorCandidateDispatch::Builtin { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(Some(result));
                }
                OperatorCandidateDispatch::Pending => {
                    self.inference.drop_probe(probe);
                    pending_count += 1;
                    pending_protocol = Some(index);
                }
                OperatorCandidateDispatch::NoMatch { reason: _ } => {
                    self.inference.drop_probe(probe);
                }
            }
        }

        // commit the unique unresolved protocol candidate
        if let (1, Some(index)) = (pending_count, pending_protocol) {
            let probe = self.inference.begin_probe();
            let result = self.select_operator_protocol_method_candidate(
                origin,
                operator,
                receiver,
                expected,
                protocols[index].clone(),
            )?;

            self.inference.commit_probe(probe)?;

            return Ok(Some(result));
        }

        if pending_count > 1 {
            Ok(Some(OperatorCandidateDispatch::Pending))
        } else {
            Ok(Some(Self::operator_no_match()))
        }
    }

    /// Select one operator method protocol candidate.
    fn select_operator_protocol_method_candidate(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        receiver: TypeOperand,
        expected: Option<TypeOperand>,
        protocol: OperatorProtocol,
    ) -> CompilerResult<OperatorCandidateDispatch> {
        // resolve protocol member on the receiver
        let module = operator.source.module_id;
        let key = protocol.method.key(&self.module(module).strings);
        let member_protocol = self.operator_member_protocol(&protocol)?;
        let members =
            match self.resolve_protocol_member(origin, module, receiver, &key, &member_protocol)? {
                MemberLookup::Pending => return Ok(OperatorCandidateDispatch::Pending),
                MemberLookup::Missing => return Ok(Self::operator_no_match()),
                MemberLookup::Field(_) => return Ok(Self::operator_no_match()),
                MemberLookup::Found(members) => members,
            };

        self.select_operator_member_candidate(origin, operator, expected, protocol, members)
    }

    /// Select one operator method member candidate.
    fn select_operator_member_candidate(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        expected: Option<TypeOperand>,
        protocol: OperatorProtocol,
        members: Vec<MemberCandidate>,
    ) -> CompilerResult<OperatorCandidateDispatch> {
        let mut pending_member = None;
        let mut pending_count = 0;
        // choose the first member candidate that accepts this operator
        for (index, member) in members.iter().enumerate() {
            let probe = self.inference.begin_probe();
            let result = self
                .select_operator_member_signature(origin, operator, expected, &protocol, member)?;

            match result {
                OperatorCandidateDispatch::Method { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                OperatorCandidateDispatch::Pending => {
                    self.inference.drop_probe(probe);
                    pending_count += 1;
                    pending_member = Some(index);
                }
                OperatorCandidateDispatch::Builtin { .. } => {
                    unreachable!("operator member selection produced a builtin");
                }
                OperatorCandidateDispatch::NoMatch { reason: _ } => {
                    self.inference.drop_probe(probe);
                }
            }
        }

        // commit the unique unresolved member candidate
        if let (1, Some(index)) = (pending_count, pending_member) {
            let probe = self.inference.begin_probe();
            let result = self.select_operator_member_signature(
                origin,
                operator,
                expected,
                &protocol,
                &members[index],
            )?;

            self.inference.commit_probe(probe)?;

            return Ok(result);
        }

        if pending_count > 1 {
            Ok(OperatorCandidateDispatch::Pending)
        } else {
            Ok(Self::operator_no_match())
        }
    }

    /// Select one operator method signature.
    fn select_operator_member_signature(
        &mut self,
        origin: Origin,
        operator: &OperatorTerm,
        expected: Option<TypeOperand>,
        protocol: &OperatorProtocol,
        member: &MemberCandidate,
    ) -> CompilerResult<OperatorCandidateDispatch> {
        let module = operator.source.module_id;

        // reduce member type to one callable signature
        let reduction = self.reduce_type_operand(origin, member.ty)?;
        let Some(term) = reduction else {
            return Ok(OperatorCandidateDispatch::Pending);
        };
        let function = match self.callable_signature(module, term)? {
            CallableSignature::Pending => return Ok(OperatorCandidateDispatch::Pending),
            CallableSignature::Absent => return Ok(Self::operator_no_match()),
            CallableSignature::Present(function) => function,
        };

        // select the method as a callable signature
        let argument_types = operator
            .argument
            .into_iter()
            .collect::<SmallVec<[TypeOperand; 4]>>();
        let dispatch = self.select_callable_signature(
            origin,
            module,
            operator.source,
            Some(member.symbol),
            member.instance.clone(),
            function,
            &[],
            &argument_types,
            &[],
            None,
            CallableTarget::Symbol {
                symbol: member.symbol,
                receiver: Some(operator.receiver),
            },
        )?;
        let function = match dispatch {
            CallableDispatch::CallSelected { function, .. } => function,
            CallableDispatch::Pending => return Ok(OperatorCandidateDispatch::Pending),
            CallableDispatch::CallRejected(_) => return Ok(Self::operator_no_match()),
            CallableDispatch::Invalid => return Ok(OperatorCandidateDispatch::Pending),
            CallableDispatch::ConstructSelected { .. } | CallableDispatch::ConstructRejected(_) => {
                unreachable!("operator method produced construct dispatch");
            }
        };

        // decide expression output
        let Some(expression_result) =
            self.operator_expression_result_term(&function, protocol.expression_result)?
        else {
            return Ok(OperatorCandidateDispatch::Pending);
        };
        let expected = self.decide_expected_result(&expression_result, expected)?;

        // commit selected method
        match expected {
            Decision::Yes => Ok(OperatorCandidateDispatch::Method {
                symbol: member.symbol,
                function,
                return_type: expression_result,
            }),
            Decision::Undecidable => Ok(OperatorCandidateDispatch::Pending),
            Decision::No => Ok(Self::operator_no_match()),
        }
    }

    /// Decide whether one type term can flow into an expected result.
    fn decide_expected_result(
        &mut self,
        source: &TypeTerm,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Decision> {
        let Some(expected) = expected else {
            return Ok(Decision::Yes);
        };
        let source = self.inference.push_term(source.clone());

        self.decide_type_relation(TypeRelation::Assignable, source, expected)
    }

    /// Return one operator expression result term.
    fn operator_expression_result_term(
        &mut self,
        function: &FunctionTerm,
        expression_result: OperatorExpressionResult,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match expression_result {
            OperatorExpressionResult::MethodReturn => match function.return_type {
                Some(return_type) => {
                    let Some(term) = self.type_operand_term(return_type)? else {
                        return Ok(None);
                    };

                    term
                }
                None => TypeTerm::Literal(TypeLiteralTerm::Void),
            },
            OperatorExpressionResult::Boolean => TypeTerm::Literal(TypeLiteralTerm::boolean()),
        };

        Ok(Some(term))
    }

    /// Return whether one type can flow into boolean.
    fn is_boolean_assignable(&mut self, operand: TypeOperand) -> CompilerResult<bool> {
        let target = Self::boolean_type_term();
        let target = self.inference.push_term(target);

        Ok(self.decide_type_relation(TypeRelation::Assignable, operand, target)? == Decision::Yes)
    }

    /// Classify one solved type operand as a builtin numeric operand.
    fn numeric_operand(&self, operand: TypeOperand) -> CompilerResult<Option<NumericOperand>> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(None);
        };
        let operand = Self::numeric_type_term(&term);

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
        &self,
        operator: dir::UnaryOperator,
        operand: &NumericOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<TypeTerm> {
        if let Some(primitive) = operand.primitive {
            return Ok(TypeTerm::Literal(TypeLiteralTerm::Primitive(primitive)));
        }
        if let Some(expected) = self.expected_numeric_type(expected)?
            && (operator != dir::UnaryOperator::ElementwiseNot
                || Self::numeric_type_term(&expected)
                    .is_some_and(|numeric| numeric.kind != NumericKind::Float))
        {
            return Ok(expected);
        }
        let literal = if operator == dir::UnaryOperator::ElementwiseNot {
            TypeLiteralTerm::integer()
        } else {
            TypeLiteralTerm::number()
        };

        Ok(TypeTerm::Literal(literal))
    }

    /// Select one primitive numeric binary operator.
    fn select_numeric_binary(
        &self,
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
            Self::boolean_type_term()
        } else {
            operand_type.clone()
        };

        Ok(Some(NumericBinaryMatch { return_type }))
    }

    /// Return the operand type for one numeric binary operator.
    fn numeric_binary_operand_type(
        &self,
        operator: dir::BinaryOperator,
        left: &NumericOperand,
        right: &NumericOperand,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<TypeTerm>> {
        if let Some(primitive) = Self::selected_numeric_primitive(left, right) {
            return Ok(Some(TypeTerm::Literal(TypeLiteralTerm::Primitive(
                primitive,
            ))));
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

        Ok(Some(TypeTerm::Literal(literal)))
    }

    /// Return an expected type when it is primitive numeric.
    fn expected_numeric_type(
        &self,
        expected: Option<TypeOperand>,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(expected) = expected else {
            return Ok(None);
        };
        let Some(term) = self.type_operand_term(expected)? else {
            return Ok(None);
        };
        if Self::numeric_type_term(&term).is_none() {
            return Ok(None);
        }

        Ok(Some(term))
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

    /// Return whether strict equality can compare both operands.
    fn strict_equality_compatible(
        &self,
        left: TypeOperand,
        right: TypeOperand,
    ) -> CompilerResult<bool> {
        let left = self.supports_strict_identity(left)?;
        let right = self.supports_strict_identity(right)?;

        Ok(left && right)
    }

    /// Return whether one type carries scalar or reference identity.
    fn supports_strict_identity(&self, operand: TypeOperand) -> CompilerResult<bool> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(false);
        };
        let is_supported = match term {
            TypeTerm::Literal(literal) => Self::literal_supports_strict_identity(&literal),
            TypeTerm::Reference { .. } => false,
            TypeTerm::Form { payload, .. } => {
                return self.supports_strict_identity(payload);
            }
            TypeTerm::Union { elements } => {
                for element in elements {
                    if !self.supports_strict_identity(element)? {
                        return Ok(false);
                    }
                }

                true
            }
            TypeTerm::Function(_) => true,
            _ => false,
        };

        Ok(is_supported)
    }

    /// Return whether one literal type carries scalar or reference identity.
    fn literal_supports_strict_identity(literal: &TypeLiteralTerm) -> bool {
        match literal {
            TypeLiteralTerm::Null | TypeLiteralTerm::Undefined | TypeLiteralTerm::Object => true,
            TypeLiteralTerm::Scalar(scalar) => Self::scalar_supports_strict_identity(scalar),
            TypeLiteralTerm::Primitive(primitive) => {
                Self::primitive_supports_strict_identity(*primitive)
            }
            _ => false,
        }
    }

    /// Return whether one scalar literal has builtin strict identity.
    fn scalar_supports_strict_identity(scalar: &dir::ScalarLiteral) -> bool {
        matches!(
            scalar,
            dir::ScalarLiteral::Boolean(_)
                | dir::ScalarLiteral::Integer(_)
                | dir::ScalarLiteral::Float(_)
                | dir::ScalarLiteral::Character(_)
        )
    }

    /// Return whether one primitive has builtin strict identity.
    fn primitive_supports_strict_identity(primitive: dir::PrimitiveType) -> bool {
        matches!(
            primitive,
            dir::PrimitiveType::Boolean
                | dir::PrimitiveType::Character
                | dir::PrimitiveType::Integer(_)
                | dir::PrimitiveType::Float(_)
                | dir::PrimitiveType::Symbol
                | dir::PrimitiveType::UniqueSymbol
        )
    }

    /// Return the builtin boolean type term.
    fn boolean_type_term() -> TypeTerm {
        TypeTerm::Literal(TypeLiteralTerm::boolean())
    }

    /// Lower one operator protocol into member protocol arguments.
    fn operator_member_protocol(
        &mut self,
        protocol: &OperatorProtocol,
    ) -> CompilerResult<MemberProtocol> {
        let mut arguments = Vec::with_capacity(protocol.arguments.len());

        for argument in &protocol.arguments {
            let argument = match argument {
                OperatorProtocolArgument::Access(access) => {
                    let value = dir::StaticTerm::Access { access: *access };
                    let value = self.inference.push_term(StaticTerm::Literal(value));

                    GenericArgument::Static(value.into())
                }
            };

            arguments.push(argument);
        }

        Ok(MemberProtocol {
            item: protocol.item,
            arguments: arguments.into(),
        })
    }
}

impl OperatorTerm {
    /// Return the operator protocols used by this term.
    pub(in crate::check) fn protocols(&self) -> SmallVec<[OperatorProtocol; 2]> {
        match self.kind {
            OperatorTermKind::Unary(operator) => unary_operator_protocols(operator),
            OperatorTermKind::Binary(operator) => binary_operator_protocols(operator),
        }
    }
}
