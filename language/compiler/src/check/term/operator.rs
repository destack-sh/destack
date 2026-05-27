use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CallableSignature, CheckState, ConstraintOrigin, FunctionTerm, GenericArgument, MemberProtocol,
    OperatorDecision, OperatorFailure, OperatorFailureReason, OperatorProtocol,
    OperatorProtocolArgument, OperatorResolution, OperatorType, Progress, StaticTerm,
    TypeLiteralTerm, TypeRelation, TypeTerm, VariableId, VariableKind, binary_operator_protocols,
    unary_operator_protocols,
};
use crate::{CompilerError, CompilerResult};
use smallvec::SmallVec;

use crate::check::Decision;

/// Runtime operator expression term.
///
/// ```ts
/// -value
/// left + right
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct OperatorTerm {
    /// The source operator expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The source operator.
    pub(in crate::check) kind: OperatorTermKind,
    /// The receiver operand type.
    pub(in crate::check) receiver: VariableId,
    /// The remaining operand type.
    pub(in crate::check) argument: Option<VariableId>,
}

impl OperatorTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.push(self.receiver);
        variables.extend(self.argument);

        variables
    }
}

/// Source operator represented by an operator term.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum OperatorTermKind {
    /// Unary source operator.
    Unary(dir::UnaryOperator),
    /// Binary source operator.
    Binary(dir::BinaryOperator),
}

/// Transient operator selection while reducing operator syntax.
pub(in crate::check) enum OperatorSelection {
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

/// Selected builtin numeric binary operation.
#[derive(Debug, Clone, PartialEq)]
struct NumericBinarySelection {
    /// The operand type applied to exact numeric literals.
    operand_type: TypeTerm,
    /// The operator result type.
    return_type: TypeTerm,
}

impl CheckState<'_> {
    /// Reduce one runtime operator to its result type.
    pub(in crate::check) fn reduce_operator_term(
        &mut self,
        operator: &OperatorTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let result = self.resolve_operator(operator, None)?;
        match &result {
            OperatorSelection::Builtin { return_type } => {
                return Ok(Some(return_type.clone()));
            }
            OperatorSelection::Method {
                symbol,
                function,
                return_type,
            } => {
                self.record_operator_method_resolution(operator, *symbol, function)?;

                return Ok(Some(return_type.clone()));
            }
            OperatorSelection::NoMatch { reason } => {
                self.record_operator_rejection(operator, *reason)?;

                return Ok(None);
            }
            OperatorSelection::Pending => return Ok(None),
        }
    }

    /// Expect resolved operator candidates to produce the expected result.
    pub(in crate::check) fn expect_operator_term(
        &mut self,
        operator: &OperatorTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.resolve_operator(operator, Some(result))?;
        let progress = match &resolved {
            OperatorSelection::Builtin { return_type } => {
                self.record_builtin_operator_resolution(operator, result)?;

                self.expect_operator_return_type(return_type, result)?
            }
            OperatorSelection::Method {
                symbol,
                function,
                return_type,
            } => {
                self.record_operator_method_resolution(operator, *symbol, function)?;

                self.expect_operator_return_type(return_type, result)?
            }
            OperatorSelection::NoMatch { reason } => {
                self.record_operator_rejection(operator, *reason)?;

                Progress::Unchanged
            }
            OperatorSelection::Pending => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Expect one selected operator return type to satisfy the result variable.
    fn expect_operator_return_type(
        &mut self,
        return_type: &TypeTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(expected) = self.solved_type_term(result)? else {
            return Ok(Progress::Unchanged);
        };

        self.constrain_solved_type_assignable(return_type, &expected)
    }

    /// Record one rejected operator for diagnostics.
    pub(in crate::check) fn record_operator_rejection(
        &mut self,
        operator: &OperatorTerm,
        reason: OperatorFailureReason,
    ) -> CompilerResult<()> {
        let failure = OperatorFailure {
            source: operator.source,
            kind: operator.kind,
            reason,
        };
        let decision = OperatorDecision::Rejected(failure);

        self.record_operator_decision(decision);

        Ok(())
    }

    /// Record one resolved operator for commit.
    pub(in crate::check) fn record_builtin_operator_resolution(
        &mut self,
        operator: &OperatorTerm,
        result: VariableId,
    ) -> CompilerResult<()> {
        let resolution = OperatorResolution::Builtin {
            source: operator.source,
            kind: operator.kind,
            receiver: operator.receiver,
            argument: operator.argument,
            result,
        };

        let decision = OperatorDecision::Resolved(resolution);

        self.record_operator_decision(decision);

        Ok(())
    }

    /// Record one resolved operator method for commit.
    pub(in crate::check) fn record_operator_method_resolution(
        &mut self,
        operator: &OperatorTerm,
        symbol: dir::GlobalSymbolId,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let resolution = OperatorResolution::Method {
            source: operator.source,
            symbol,
            receiver: operator.receiver,
            function: function.clone(),
        };

        let decision = OperatorDecision::Resolved(resolution);

        self.record_operator_decision(decision);

        Ok(())
    }

    /// Resolve one runtime operator from builtin and method candidates.
    pub(in crate::check) fn resolve_operator(
        &mut self,
        operator: &OperatorTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<OperatorSelection> {
        let Some(receiver) = self.solved_type_term(operator.receiver)? else {
            return Ok(OperatorSelection::Pending);
        };

        // choose primitive operators
        if let Some(result) = self.resolve_builtin_operator(operator, &receiver, expected)? {
            return Ok(result);
        }

        // choose operator method overload
        if let Some(result) = self.resolve_operator_method(operator, &receiver, expected)? {
            return Ok(result);
        }

        Ok(Self::operator_no_match())
    }

    /// Return the generic no-match operator result.
    fn operator_no_match() -> OperatorSelection {
        OperatorSelection::NoMatch {
            reason: OperatorFailureReason::NoMatch,
        }
    }

    /// Resolve one builtin operator case.
    fn resolve_builtin_operator(
        &mut self,
        operator: &OperatorTerm,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<OperatorSelection>> {
        let result = match operator.kind {
            OperatorTermKind::Unary(kind) => {
                self.resolve_builtin_unary(operator, kind, receiver, expected)?
            }
            OperatorTermKind::Binary(kind) => {
                self.resolve_builtin_binary(operator, kind, receiver, expected)?
            }
        };

        Ok(result)
    }

    /// Resolve one builtin unary operator case.
    fn resolve_builtin_unary(
        &mut self,
        operator: &OperatorTerm,
        kind: dir::UnaryOperator,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<OperatorSelection>> {
        let result = match kind {
            dir::UnaryOperator::Not => {
                let target = Self::boolean_type_term();
                self.expect_literal_term(operator.receiver, receiver, &target)?;

                Some(target)
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
                self.expect_literal_term(operator.receiver, receiver, &target)?;

                Some(target)
            }
            dir::UnaryOperator::Void => Some(TypeTerm::Literal(TypeLiteralTerm::Void)),
            dir::UnaryOperator::Typeof => Some(TypeTerm::Literal(TypeLiteralTerm::Primitive(
                dir::PrimitiveType::String,
            ))),
            _ => None,
        };

        if let Some(result) = result {
            if self.decide_expected_type(&result, expected)? == Decision::No {
                return Ok(Some(Self::operator_no_match()));
            }

            return Ok(Some(OperatorSelection::Builtin {
                return_type: result,
            }));
        }

        Ok(None)
    }

    /// Resolve one builtin binary operator case.
    fn resolve_builtin_binary(
        &mut self,
        operator: &OperatorTerm,
        kind: dir::BinaryOperator,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<OperatorSelection>> {
        let Some(argument) = operator.argument else {
            return Ok(Some(Self::operator_no_match()));
        };
        let Some(argument_type) = self.solved_type_term(argument)? else {
            return Ok(Some(OperatorSelection::Pending));
        };

        // choose strict identity comparison
        if Self::strict_equality_operator(kind) {
            let is_compatible = self.strict_equality_compatible(receiver, &argument_type)?;
            let target = Self::boolean_type_term();

            if is_compatible && self.decide_expected_type(&target, expected)? != Decision::No {
                return Ok(Some(OperatorSelection::Builtin {
                    return_type: target,
                }));
            }

            return Ok(Some(OperatorSelection::NoMatch {
                reason: OperatorFailureReason::InvalidStrictEquality,
            }));
        }

        // choose primitive numeric operators
        if let Some(left) = self.numeric_operand(receiver)?
            && let Some(right) = self.numeric_operand(&argument_type)?
            && let Some(selection) = self.resolve_numeric_binary(kind, &left, &right, expected)?
        {
            self.expect_literal_term(operator.receiver, receiver, &selection.operand_type)?;
            self.expect_literal_term(argument, &argument_type, &selection.operand_type)?;

            if self.decide_expected_type(&selection.return_type, expected)? == Decision::No {
                return Ok(Some(Self::operator_no_match()));
            }

            return Ok(Some(OperatorSelection::Builtin {
                return_type: selection.return_type,
            }));
        }

        // choose boolean logical operators
        if Self::logical_boolean_operator(kind)
            && self.is_boolean_assignable(receiver)?
            && self.is_boolean_assignable(&argument_type)?
        {
            let target = Self::boolean_type_term();

            self.expect_literal_term(operator.receiver, receiver, &target)?;
            self.expect_literal_term(argument, &argument_type, &target)?;
            if self.decide_expected_type(&target, expected)? == Decision::No {
                return Ok(Some(Self::operator_no_match()));
            }

            return Ok(Some(OperatorSelection::Builtin {
                return_type: target,
            }));
        }

        Ok(None)
    }

    /// Return whether one numeric operator returns boolean.
    pub(in crate::check) fn numeric_boolean_operator(operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::Equal
                | dir::BinaryOperator::NotEqual
                | dir::BinaryOperator::LessThan
                | dir::BinaryOperator::LessThanOrEqual
                | dir::BinaryOperator::GreaterThan
                | dir::BinaryOperator::GreaterThanOrEqual
        )
    }

    /// Return whether one operator can use primitive numeric rules.
    fn primitive_numeric_operator(operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::Exponent
                | dir::BinaryOperator::Multiply
                | dir::BinaryOperator::Divide
                | dir::BinaryOperator::Remainder
                | dir::BinaryOperator::Add
                | dir::BinaryOperator::Subtract
                | dir::BinaryOperator::ShiftLeft
                | dir::BinaryOperator::ShiftRight
                | dir::BinaryOperator::UnsignedShiftRight
                | dir::BinaryOperator::ElementwiseAnd
                | dir::BinaryOperator::ElementwiseXor
                | dir::BinaryOperator::ElementwiseOr
                | dir::BinaryOperator::Equal
                | dir::BinaryOperator::NotEqual
                | dir::BinaryOperator::LessThan
                | dir::BinaryOperator::LessThanOrEqual
                | dir::BinaryOperator::GreaterThan
                | dir::BinaryOperator::GreaterThanOrEqual
        )
    }

    /// Return whether one operator is boolean short-circuit logic.
    pub(in crate::check) fn logical_boolean_operator(operator: dir::BinaryOperator) -> bool {
        matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or)
    }

    /// Return whether one operator is strict identity equality.
    pub(in crate::check) fn strict_equality_operator(operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict
        )
    }

    /// Resolve one operator method.
    fn resolve_operator_method(
        &mut self,
        operator: &OperatorTerm,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<OperatorSelection>> {
        let protocols = operator.protocols();
        if protocols.is_empty() {
            return Ok(None);
        }
        let mut is_pending = false;

        // choose the first protocol candidate that resolves
        for protocol in protocols {
            let result =
                self.resolve_operator_protocol_method(operator, receiver, expected, protocol)?;
            match result {
                OperatorSelection::Method { .. } => return Ok(Some(result)),
                OperatorSelection::Pending => is_pending = true,
                OperatorSelection::NoMatch { reason: _ } => {}
                OperatorSelection::Builtin { .. } => return Ok(Some(result)),
            }
        }

        if is_pending {
            Ok(Some(OperatorSelection::Pending))
        } else {
            Ok(Some(Self::operator_no_match()))
        }
    }

    /// Resolve one operator method protocol candidate.
    fn resolve_operator_protocol_method(
        &mut self,
        operator: &OperatorTerm,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
        protocol: OperatorProtocol,
    ) -> CompilerResult<OperatorSelection> {
        let key = protocol
            .method
            .key(&self.input(operator.receiver.module).strings);
        let origin = ConstraintOrigin::Node(operator.source);
        let member_protocol =
            self.operator_member_protocol(operator.receiver.module, origin, &protocol)?;
        let Some(member) = self.member_type_candidate_for_protocol(
            operator.receiver.module,
            receiver,
            &key,
            &member_protocol,
        )?
        else {
            return Ok(Self::operator_no_match());
        };
        let reduction = self.reduce_type_term(operator.receiver.module, &member.ty)?;
        let Some(term) = reduction.value else {
            return Ok(OperatorSelection::Pending);
        };
        let function = match self.call_signature(operator.receiver.module, &term)? {
            CallableSignature::Pending => return Ok(OperatorSelection::Pending),
            CallableSignature::Absent => return Ok(Self::operator_no_match()),
            CallableSignature::Present(function) => function,
        };

        let arguments = self.decide_operator_method_arguments(operator, &function)?;
        let method_return = self.decide_operator_type(&function, protocol.method_return)?;
        let expression_type = self.operator_type_term(&function, protocol.expression_type)?;
        let expected = self.decide_expected_type(&expression_type, expected)?;
        match arguments.and(method_return).and(expected) {
            Decision::Yes => {
                self.expect_operator_method_arguments(operator, &function)?;
                self.expect_operator_type(&function, protocol.method_return)?;

                Ok(OperatorSelection::Method {
                    symbol: member.symbol,
                    function,
                    return_type: expression_type,
                })
            }
            Decision::Undecidable => Ok(OperatorSelection::Pending),
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
        let Some(expected) = self.solved_type_term(expected)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_type_term_relation(TypeRelation::Assignable, source, &expected)
    }

    /// Decide whether one operator method return matches the protocol.
    fn decide_operator_type(
        &mut self,
        function: &FunctionTerm,
        operator_type: OperatorType,
    ) -> CompilerResult<Decision> {
        let source = self.operator_type_term(function, OperatorType::MethodReturn)?;
        let target = self.operator_type_term(function, operator_type)?;

        self.decide_type_term_relation(TypeRelation::Assignable, &source, &target)
    }

    /// Return one operator protocol type term.
    fn operator_type_term(
        &mut self,
        function: &FunctionTerm,
        operator_type: OperatorType,
    ) -> CompilerResult<TypeTerm> {
        let term = match operator_type {
            OperatorType::MethodReturn => match function.return_type {
                Some(return_type) => TypeTerm::Variable(return_type),
                None => TypeTerm::Literal(TypeLiteralTerm::Void),
            },
            OperatorType::Boolean => TypeTerm::Literal(TypeLiteralTerm::boolean()),
            OperatorType::LanguageItem(item) => {
                let module = self.operator_type_module(function)?;
                self.language_item_type_term(module, item)?
            }
            OperatorType::NullableLanguageItem(item) => {
                let module = self.operator_type_module(function)?;
                let value = self.language_item_type_term(module, item)?;

                self.nullable_operator_type_term(value)?
            }
        };

        Ok(term)
    }

    /// Apply one operator protocol type to the selected method return.
    fn expect_operator_type(
        &mut self,
        function: &FunctionTerm,
        operator_type: OperatorType,
    ) -> CompilerResult<Progress> {
        if operator_type == OperatorType::MethodReturn {
            return Ok(Progress::Unchanged);
        }
        let Some(return_type) = function.return_type else {
            return Ok(Progress::Unchanged);
        };
        let target = self.operator_type_term(function, operator_type)?;
        let target = self.terms.push(target);

        self.solve_type_assignability(return_type, target)
    }

    /// Expect accepted operands to satisfy operator method types.
    fn expect_operator_method_arguments(
        &mut self,
        operator: &OperatorTerm,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        if let Some(this_parameter) = function.this_parameter
            && let Some(source) = self.solved_type_term(operator.receiver)?
            && let Some(target) = self.solved_type_term(this_parameter)?
        {
            self.expect_literal_term(operator.receiver, &source, &target)?;
        }
        if let Some(argument) = operator.argument
            && let Some(source) = self.solved_type_term(argument)?
        {
            let parameter = &function.parameters[0];
            let Some(target) = self.type_operand_term(parameter.ty)? else {
                return Ok(());
            };

            self.expect_literal_term(argument, &source, &target)?;
        }

        Ok(())
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
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                return self.numeric_operand(&term);
            }
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
        if let Some(expected) = self.expected_numeric_type(expected)? {
            if operator != dir::UnaryOperator::ElementwiseNot
                || self
                    .numeric_operand(&expected)?
                    .is_some_and(|numeric| numeric.shape != NumericShape::Float)
            {
                return Ok(expected);
            }
        }
        let literal = if operator == dir::UnaryOperator::ElementwiseNot {
            TypeLiteralTerm::integer()
        } else {
            TypeLiteralTerm::number()
        };

        Ok(TypeTerm::Literal(literal))
    }

    /// Resolve one primitive numeric binary operator.
    fn resolve_numeric_binary(
        &self,
        operator: dir::BinaryOperator,
        left: &NumericOperand,
        right: &NumericOperand,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<NumericBinarySelection>> {
        if !Self::primitive_numeric_operator(operator) {
            return Ok(None);
        }
        if Self::integer_operator(operator)
            && (left.shape == NumericShape::Float || right.shape == NumericShape::Float)
        {
            return Ok(None);
        }
        let Some(operand_type) =
            self.numeric_binary_operand_type(operator, left, right, expected)?
        else {
            return Ok(None);
        };
        let return_type = if Self::numeric_boolean_operator(operator) {
            Self::boolean_type_term()
        } else {
            operand_type.clone()
        };

        Ok(Some(NumericBinarySelection {
            operand_type,
            return_type,
        }))
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
        if !Self::numeric_boolean_operator(operator)
            && let Some(expected) = self.expected_numeric_type(expected)?
        {
            return Ok(Some(expected));
        }
        let literal = if Self::integer_operator(operator) {
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
        let Some(term) = self.solved_type_term(expected)? else {
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

    /// Return whether one operator is limited to integer numeric operands.
    fn integer_operator(operator: dir::BinaryOperator) -> bool {
        matches!(
            operator,
            dir::BinaryOperator::ShiftLeft
                | dir::BinaryOperator::ShiftRight
                | dir::BinaryOperator::UnsignedShiftRight
                | dir::BinaryOperator::ElementwiseAnd
                | dir::BinaryOperator::ElementwiseXor
                | dir::BinaryOperator::ElementwiseOr
        )
    }

    /// Return whether strict equality can compare both operands.
    fn strict_equality_compatible(
        &self,
        left: &TypeTerm,
        right: &TypeTerm,
    ) -> CompilerResult<bool> {
        let left = self.supports_strict_identity(left)?;
        let right = self.supports_strict_identity(right)?;

        Ok(left && right)
    }

    /// Return whether one type carries scalar or reference identity.
    fn supports_strict_identity(&self, term: &TypeTerm) -> CompilerResult<bool> {
        let is_supported = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                return self.supports_strict_identity(&term);
            }
            TypeTerm::Literal(literal) => Self::literal_supports_strict_identity(literal),
            TypeTerm::Reference { symbol, .. } => self.symbol_supports_strict_identity(*symbol)?,
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(false);
                };

                return self.supports_strict_identity(&term);
            }
            TypeTerm::Union { elements } => {
                for element in elements {
                    let Some(term) = self.type_operand_term(*element)? else {
                        return Ok(false);
                    };
                    if !self.supports_strict_identity(&term)? {
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
        matches!(
            literal,
            TypeLiteralTerm::Null
                | TypeLiteralTerm::Undefined
                | TypeLiteralTerm::Object
                | TypeLiteralTerm::Scalar(_)
                | TypeLiteralTerm::Primitive(_)
        )
    }

    /// Return whether one nominal symbol carries reference identity.
    fn symbol_supports_strict_identity(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let binding_table = self.input(symbol.module_id).binding_table();
        let symbol = binding_table.get_symbol(symbol.local_id);

        Ok(matches!(
            symbol.kind,
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
        let symbol = self.language_symbol(module, item)?;

        Ok(TypeTerm::Reference {
            source: None,
            symbol,
            arguments: Vec::new().into(),
        })
    }

    /// Return a nullable protocol return type.
    fn nullable_operator_type_term(&mut self, value: TypeTerm) -> CompilerResult<TypeTerm> {
        let value = self.terms.push(value);
        let null = self.terms.push(TypeTerm::Literal(TypeLiteralTerm::Null));

        Ok(TypeTerm::Union {
            elements: vec![value.into(), null.into()],
        })
    }

    /// Return the module used for synthetic operator type pieces.
    fn operator_type_module(&self, function: &FunctionTerm) -> CompilerResult<ModuleId> {
        if let Some(return_type) = function.return_type {
            return Ok(return_type.module);
        }
        if let Some(parameter) = function.this_parameter {
            return Ok(parameter.module);
        }
        if let Some(parameter) = function.parameters.first() {
            let Some(variable) = parameter.ty.variable() else {
                return Err(CompilerError::Internal {
                    message: "operator parameter type has no variable anchor".to_owned(),
                });
            };

            return Ok(variable.module);
        }

        let Some(parameter) = function.generic_parameters.first() else {
            return Err(CompilerError::Internal {
                message: "operator function has no typed variable anchor".to_owned(),
            });
        };

        Ok(parameter.module)
    }

    /// Lower one operator protocol into member protocol arguments.
    fn operator_member_protocol(
        &mut self,
        module: ModuleId,
        origin: ConstraintOrigin,
        protocol: &OperatorProtocol,
    ) -> CompilerResult<MemberProtocol> {
        let mut arguments = Vec::with_capacity(protocol.arguments.len());

        for argument in &protocol.arguments {
            let argument = match argument {
                OperatorProtocolArgument::Access(access) => {
                    let value = dir::StaticTerm::Access { access: *access };
                    let variable =
                        self.allocate_intermediate_variable(module, VariableKind::Static, origin);
                    self.define_static(module, variable, StaticTerm::Literal(value));

                    GenericArgument::Static(variable.into())
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
    fn protocols(&self) -> SmallVec<[OperatorProtocol; 2]> {
        match self.kind {
            OperatorTermKind::Unary(operator) => unary_operator_protocols(operator),
            OperatorTermKind::Binary(operator) => binary_operator_protocols(operator),
        }
    }

    /// Return the number of method arguments after the receiver.
    fn argument_count(&self) -> usize {
        usize::from(self.argument.is_some())
    }
}
