use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallableSignature, CheckState, Decision, FunctionTerm, GenericArgument, MemberProtocol,
    OperatorFailureReason, OperatorProtocol, OperatorProtocolArgument, OperatorTerm,
    OperatorTermKind, OperatorType, Origin, Progress, StaticTerm, TypeLiteralTerm, TypeRelation,
    TypeTerm, VariableId, binary_operator_protocols, unary_operator_protocols,
};

use super::CandidateSet;

/// Transient operator selection while reducing operator syntax.
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
        /// The operator expression result type.
        return_type: TypeTerm,
    },
}

/// Primitive numeric operand shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericShape {
    /// Integer literal or primitive.
    Integer,
    /// Floating-point literal or primitive.
    Float,
}

/// Solved numeric operand classification.
#[derive(Debug, Clone, PartialEq)]
struct NumericOperand {
    /// The operand shape.
    shape: NumericShape,
    /// The concrete primitive type, when the operand already has one.
    primitive: Option<dir::PrimitiveType>,
}

/// Matched builtin numeric binary operation.
#[derive(Debug, Clone, PartialEq)]
struct NumericBinaryMatch {
    /// The operator result type.
    return_type: TypeTerm,
}

/// Builtin operator candidate.
struct BuiltinOperatorMatch {
    /// The operator result type.
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
        expected: Option<VariableId>,
    ) -> CompilerResult<OperatorCandidateDispatch> {
        let Some(receiver) = self.type_operand_term(operator.receiver)? else {
            return Ok(OperatorCandidateDispatch::Pending);
        };

        // choose primitive operators
        if let Some(result) =
            self.select_builtin_operator_candidate(operator, &receiver, expected)?
        {
            return Ok(result);
        }

        // choose operator method overload
        if let Some(result) =
            self.select_operator_method_candidate(origin, operator, &receiver, expected)?
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
        operator: &OperatorTerm,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<OperatorCandidateDispatch>> {
        let decision = match operator.kind {
            OperatorTermKind::Unary(kind) => self.decide_builtin_unary(kind, receiver, expected)?,
            OperatorTermKind::Binary(kind) => {
                self.decide_builtin_binary(operator, kind, receiver, expected)?
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
        if self.decide_expected_type(&selection.return_type, expected)? == Decision::No {
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
        receiver: &TypeTerm,
        expected: Option<VariableId>,
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
                if kind == dir::UnaryOperator::ElementwiseNot
                    && numeric.shape == NumericShape::Float
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
        &self,
        operator: &OperatorTerm,
        kind: dir::BinaryOperator,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<BuiltinOperatorDecision>> {
        let Some(argument) = operator.argument else {
            return Ok(Some(BuiltinOperatorDecision::Rejected(
                OperatorFailureReason::NoMatch,
            )));
        };
        let Some(argument_type) = self.type_operand_term(argument)? else {
            return Ok(Some(BuiltinOperatorDecision::Pending));
        };

        // choose strict identity comparison
        if kind.is_strict_equality() {
            let module = operator.source.module_id;
            let is_compatible =
                self.strict_equality_compatible(module, receiver, &argument_type)?;
            let target = Self::boolean_type_term();

            if is_compatible && self.decide_expected_type(&target, expected)? != Decision::No {
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
            && let Some(right) = self.numeric_operand(&argument_type)?
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
            && self.is_boolean_assignable(&argument_type)?
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
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<OperatorCandidateDispatch>> {
        let protocols = operator.protocols();
        if protocols.is_empty() {
            return Ok(None);
        }
        let mut is_pending = false;
        let candidate_set = CandidateSet::from_len(protocols.len());

        // choose the first protocol candidate that resolves
        for protocol in protocols {
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
                    if candidate_set.keeps_pending_probe() {
                        self.inference.commit_probe(probe)?;

                        return Ok(Some(result));
                    }

                    self.inference.drop_probe(probe);
                    is_pending = true;
                }
                OperatorCandidateDispatch::NoMatch { reason: _ } => {
                    self.inference.drop_probe(probe);
                }
            }
        }

        if is_pending {
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
        receiver: &TypeTerm,
        expected: Option<VariableId>,
        protocol: OperatorProtocol,
    ) -> CompilerResult<OperatorCandidateDispatch> {
        // resolve protocol member on the receiver
        let module = operator.source.module_id;
        let key = protocol.method.key(&self.module(module).strings);
        let member_protocol = self.operator_member_protocol(&protocol)?;
        let Some(member) = self.member_type_candidate_for_protocol(
            origin,
            module,
            receiver,
            &key,
            &member_protocol,
        )?
        else {
            return Ok(Self::operator_no_match());
        };

        // reduce member type to one callable signature
        let reduction = self.reduce_type_term(origin, &member.ty)?;
        let Some(term) = reduction.value else {
            return Ok(OperatorCandidateDispatch::Pending);
        };
        let function = match self.call_signature(module, &term)? {
            CallableSignature::Pending => return Ok(OperatorCandidateDispatch::Pending),
            CallableSignature::Absent => return Ok(Self::operator_no_match()),
            CallableSignature::Present(function) => function,
        };

        // decide operator method inputs and protocol output
        let arguments = self.decide_operator_method_arguments(operator, &function)?;
        let method_return = self.reduce_operator_type(module, &function, protocol.method_return)?;
        let Some(expression_type) =
            self.operator_type_term(module, &function, protocol.expression_type)?
        else {
            return Ok(OperatorCandidateDispatch::Pending);
        };
        let expected = self.decide_expected_type(&expression_type, expected)?;

        // commit selected method expectations
        match arguments.and(method_return).and(expected) {
            Decision::Yes => {
                self.expect_operator_type(origin, module, &function, protocol.method_return)?;

                Ok(OperatorCandidateDispatch::Method {
                    symbol: member.symbol,
                    function,
                    return_type: expression_type,
                })
            }
            Decision::Undecidable => Ok(OperatorCandidateDispatch::Pending),
            Decision::No => Ok(Self::operator_no_match()),
        }
    }

    /// Decide whether operands are assignable to an operator method.
    fn decide_operator_method_arguments(
        &self,
        operator: &OperatorTerm,
        function: &FunctionTerm,
    ) -> CompilerResult<Decision> {
        if function.parameters.len() != operator.argument_count() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // match receiver when the method has an explicit this parameter
        if let Some(this_parameter) = function.this_parameter {
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                operator.receiver,
                this_parameter,
            )?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        if let Some(argument) = operator.argument {
            let parameter = &function.parameters[0];

            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                argument,
                parameter.ty,
            )?);
        }

        Ok(decision)
    }

    /// Decide whether one type term can flow into an expected result.
    fn decide_expected_type(
        &self,
        source: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Decision> {
        let Some(expected) = expected else {
            return Ok(Decision::Yes);
        };
        let Some(expected) = self.type_solution(expected)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(TypeRelation::Assignable, source, &expected)
    }

    /// Decide whether one operator method return matches the protocol.
    fn reduce_operator_type(
        &mut self,
        module: ModuleId,
        function: &FunctionTerm,
        operator_type: OperatorType,
    ) -> CompilerResult<Decision> {
        let Some(source) = self.operator_type_term(module, function, OperatorType::MethodReturn)?
        else {
            return Ok(Decision::Undecidable);
        };
        let Some(target) = self.operator_type_term(module, function, operator_type)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(TypeRelation::Assignable, &source, &target)
    }

    /// Return one operator protocol type term.
    fn operator_type_term(
        &mut self,
        module: ModuleId,
        function: &FunctionTerm,
        operator_type: OperatorType,
    ) -> CompilerResult<Option<TypeTerm>> {
        let term = match operator_type {
            OperatorType::MethodReturn => match function.return_type {
                Some(return_type) => {
                    let Some(term) = self.type_operand_term(return_type)? else {
                        return Ok(None);
                    };

                    term
                }
                None => TypeTerm::Literal(TypeLiteralTerm::Void),
            },
            OperatorType::Boolean => TypeTerm::Literal(TypeLiteralTerm::boolean()),
            OperatorType::LanguageItem(item) => self.language_item_type_term(module, item)?,
            OperatorType::NullableLanguageItem(item) => {
                let value = self.language_item_type_term(module, item)?;

                self.nullable_operator_type_term(value)?
            }
        };

        Ok(Some(term))
    }

    /// Apply one operator protocol type to the selected method return.
    fn expect_operator_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        function: &FunctionTerm,
        operator_type: OperatorType,
    ) -> CompilerResult<Progress> {
        if operator_type == OperatorType::MethodReturn {
            return Ok(Progress::Unchanged);
        }
        let Some(return_type) = function.return_type else {
            return Ok(Progress::Unchanged);
        };
        let Some(target) = self.operator_type_term(module, function, operator_type)? else {
            return Ok(Progress::Unchanged);
        };
        let target = self.inference.push_term(target);

        self.relate_contextual_type_assignability(origin, return_type, target)
    }

    /// Return whether one type can flow into boolean.
    fn is_boolean_assignable(&self, term: &TypeTerm) -> CompilerResult<bool> {
        let target = Self::boolean_type_term();

        Ok(
            self.decide_type_term_relation(TypeRelation::Assignable, term, &target)?
                == Decision::Yes,
        )
    }

    /// Classify one solved type as a builtin numeric operand.
    fn numeric_operand(&self, term: &TypeTerm) -> CompilerResult<Option<NumericOperand>> {
        let operand = match term {
            TypeTerm::Literal(TypeLiteralTerm::Scalar(dir::ScalarLiteral::Integer(_))) => {
                NumericOperand {
                    shape: NumericShape::Integer,
                    primitive: None,
                }
            }
            TypeTerm::Literal(TypeLiteralTerm::Scalar(dir::ScalarLiteral::Float(_))) => {
                NumericOperand {
                    shape: NumericShape::Float,
                    primitive: None,
                }
            }
            TypeTerm::Literal(TypeLiteralTerm::Primitive(primitive)) => {
                let Some(shape) = Self::numeric_primitive(*primitive) else {
                    return Ok(None);
                };

                NumericOperand {
                    shape,
                    primitive: Some(*primitive),
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(operand))
    }

    /// Classify one primitive as a builtin numeric primitive.
    fn numeric_primitive(primitive: dir::PrimitiveType) -> Option<NumericShape> {
        let numeric = match primitive {
            dir::PrimitiveType::Integer(_) => NumericShape::Integer,
            dir::PrimitiveType::Float(_) => NumericShape::Float,
            _ => return None,
        };

        Some(numeric)
    }

    /// Return the builtin result type for one unary numeric operator.
    fn unary_numeric_result_type(
        &self,
        operator: dir::UnaryOperator,
        operand: &NumericOperand,
        expected: Option<VariableId>,
    ) -> CompilerResult<TypeTerm> {
        if let Some(primitive) = operand.primitive {
            return Ok(TypeTerm::Literal(TypeLiteralTerm::Primitive(primitive)));
        }
        if let Some(expected) = self.expected_numeric_type(expected)?
            && (operator != dir::UnaryOperator::ElementwiseNot
                || self
                    .numeric_operand(&expected)?
                    .is_some_and(|numeric| numeric.shape != NumericShape::Float))
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
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<NumericBinaryMatch>> {
        if !operator.has_numeric_builtin() {
            return Ok(None);
        }
        if operator.requires_integer_numeric_operands()
            && (left.shape == NumericShape::Float || right.shape == NumericShape::Float)
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
        expected: Option<VariableId>,
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
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(expected) = expected else {
            return Ok(None);
        };
        let Some(term) = self.type_solution(expected)? else {
            return Ok(None);
        };
        if self.numeric_operand(&term)?.is_none() {
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
        module: ModuleId,
        left: &TypeTerm,
        right: &TypeTerm,
    ) -> CompilerResult<bool> {
        let left = self.supports_strict_identity(module, left)?;
        let right = self.supports_strict_identity(module, right)?;

        Ok(left && right)
    }

    /// Return whether one type carries scalar or reference identity.
    fn supports_strict_identity(&self, module: ModuleId, term: &TypeTerm) -> CompilerResult<bool> {
        let is_supported = match term {
            TypeTerm::Literal(literal) => Self::literal_supports_strict_identity(literal),
            TypeTerm::Reference { symbol, .. } => self.symbol_supports_strict_identity(*symbol)?,
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(false);
                };

                return self.supports_strict_identity(module, &term);
            }
            TypeTerm::Union { elements } => {
                for element in elements {
                    let Some(term) = self.type_operand_term(*element)? else {
                        return Ok(false);
                    };
                    if !self.supports_strict_identity(module, &term)? {
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

    /// Return whether one nominal symbol carries reference identity.
    fn symbol_supports_strict_identity(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let kind = self.symbol_kind(symbol);

        Ok(matches!(
            kind,
            dir::SymbolKind::Class | dir::SymbolKind::Function
        ))
    }

    /// Return the builtin boolean type term.
    fn boolean_type_term() -> TypeTerm {
        TypeTerm::Literal(TypeLiteralTerm::boolean())
    }

    /// Return a nominal language item type term.
    fn language_item_type_term(
        &self,
        module: ModuleId,
        item: dir::LanguageItem,
    ) -> CompilerResult<TypeTerm> {
        let symbol = self.language_symbol(module, item);

        Ok(TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments: Vec::new().into(),
        })
    }

    /// Return a nullable protocol return type.
    fn nullable_operator_type_term(&mut self, value: TypeTerm) -> CompilerResult<TypeTerm> {
        let value = self.inference.push_term(value);
        let null = self
            .inference
            .push_term(TypeTerm::Literal(TypeLiteralTerm::Null));

        Ok(TypeTerm::Union {
            elements: vec![value.into(), null.into()],
        })
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

    /// Return the number of method arguments after the receiver.
    pub(in crate::check) fn argument_count(&self) -> usize {
        usize::from(self.argument.is_some())
    }
}
