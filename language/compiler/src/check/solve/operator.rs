use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    ArgumentTerm, CheckComponentState, FunctionTerm, MemberProtocol, OperatorFailure,
    OperatorFailureReason, OperatorOutcome, OperatorProtocol, OperatorProtocolArgument,
    OperatorResolution, OperatorTerm, OperatorTermKind, OperatorType, TypeRelation, TypeTerm,
    VariableId, binary_operator_protocols, unary_operator_protocols,
};
use crate::{CompilerError, CompilerResult};
use smallvec::SmallVec;

use super::Decision;
use super::call::CallSignature;
use super::queue::Progress;

/// Result of resolving a runtime operator.
pub(in crate::check) enum OperatorResult {
    /// Operator resolution is waiting for solver input.
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

impl CheckComponentState<'_> {
    /// Reduce one runtime operator to its result type.
    pub(super) fn reduce_operator_type(
        &mut self,
        operator: &OperatorTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let result = self.resolve_operator(operator, None)?;
        match &result {
            OperatorResult::Builtin { return_type } => {
                let result =
                    self.push_solved_type_variable(operator.receiver.module, return_type.clone())?;
                self.record_builtin_operator_resolution(operator, result)?;

                return Ok(Some(TypeTerm::Variable(result)));
            }
            OperatorResult::Method {
                symbol,
                function,
                return_type,
            } => {
                let result =
                    self.push_solved_type_variable(operator.receiver.module, return_type.clone())?;
                self.record_operator_method_resolution(operator, *symbol, function)?;

                return Ok(Some(TypeTerm::Variable(result)));
            }
            OperatorResult::NoMatch { reason } => {
                self.record_operator_rejection(operator, *reason)?;

                return Ok(None);
            }
            OperatorResult::Pending => return Ok(None),
        }
    }

    /// Apply an expected operator result to resolved operator candidates.
    pub(super) fn expect_operator_result(
        &mut self,
        operator: &OperatorTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.resolve_operator(operator, Some(result))?;
        let progress = match &resolved {
            OperatorResult::Builtin { return_type } => {
                let return_type =
                    self.push_solved_type_variable(operator.receiver.module, return_type.clone())?;
                self.record_builtin_operator_resolution(operator, return_type)?;

                self.relate_type_assignable(return_type, result)?
            }
            OperatorResult::Method {
                symbol,
                function,
                return_type,
            } => {
                let return_type =
                    self.push_solved_type_variable(operator.receiver.module, return_type.clone())?;
                self.record_operator_method_resolution(operator, *symbol, function)?;

                self.relate_type_assignable(return_type, result)?
            }
            OperatorResult::NoMatch { reason } => {
                self.record_operator_rejection(operator, *reason)?;

                Progress::Unchanged
            }
            OperatorResult::Pending => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Record one rejected operator for diagnostics.
    pub(super) fn record_operator_rejection(
        &mut self,
        operator: &OperatorTerm,
        reason: OperatorFailureReason,
    ) -> CompilerResult<()> {
        let failure = OperatorFailure {
            source: operator.source,
            kind: operator.kind,
            reason,
        };
        let outcome = OperatorOutcome::Rejected(failure);

        self.module_mut(operator.source.module_id)?
            .record_operator_outcome(outcome);

        Ok(())
    }

    /// Record one resolved operator for commit.
    pub(super) fn record_builtin_operator_resolution(
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

        let outcome = OperatorOutcome::Resolved(resolution);

        self.module_mut(operator.source.module_id)?
            .record_operator_outcome(outcome);

        Ok(())
    }

    /// Record one resolved operator method for commit.
    pub(super) fn record_operator_method_resolution(
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

        let outcome = OperatorOutcome::Resolved(resolution);

        self.module_mut(operator.source.module_id)?
            .record_operator_outcome(outcome);

        Ok(())
    }

    /// Resolve one runtime operator from builtin and method candidates.
    pub(in crate::check) fn resolve_operator(
        &mut self,
        operator: &OperatorTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<OperatorResult> {
        let Some(receiver) = self.solved_type_term(operator.receiver)? else {
            return Ok(OperatorResult::Pending);
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
    fn operator_no_match() -> OperatorResult {
        OperatorResult::NoMatch {
            reason: OperatorFailureReason::NoMatch,
        }
    }

    /// Resolve one builtin operator case.
    fn resolve_builtin_operator(
        &mut self,
        operator: &OperatorTerm,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<OperatorResult>> {
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
    ) -> CompilerResult<Option<OperatorResult>> {
        let result = match kind {
            dir::UnaryOperator::Not => {
                let target = Self::boolean_type_term();
                let _ = self.expect_type(operator.receiver, receiver, &target)?;

                Some(target)
            }
            dir::UnaryOperator::PostIncrement
            | dir::UnaryOperator::PostDecrement
            | dir::UnaryOperator::PreIncrement
            | dir::UnaryOperator::PreDecrement
            | dir::UnaryOperator::Plus
            | dir::UnaryOperator::Negate
            | dir::UnaryOperator::ElementwiseNot
                if self.is_int32_assignable(receiver)? =>
            {
                let target = TypeTerm::Literal(Self::int32_type());
                let _ = self.expect_type(operator.receiver, receiver, &target)?;

                Some(target)
            }
            dir::UnaryOperator::Void => Some(TypeTerm::Literal(dir::Type::Void)),
            dir::UnaryOperator::Typeof => Some(TypeTerm::Literal(dir::Type::Primitive(
                dir::PrimitiveType::String,
            ))),
            _ => None,
        };

        if let Some(result) = result {
            if self.decide_expected_type(&result, expected)? == Decision::No {
                return Ok(Some(Self::operator_no_match()));
            }

            return Ok(Some(OperatorResult::Builtin {
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
    ) -> CompilerResult<Option<OperatorResult>> {
        let Some(argument) = operator.argument else {
            return Ok(Some(Self::operator_no_match()));
        };
        let Some(argument_type) = self.solved_type_term(argument)? else {
            return Ok(Some(OperatorResult::Pending));
        };

        // choose strict identity comparison
        if Self::strict_equality_operator(kind) {
            let is_compatible = self.strict_equality_compatible(
                operator.receiver.module,
                receiver,
                &argument_type,
            )?;
            let target = Self::boolean_type_term();

            if is_compatible && self.decide_expected_type(&target, expected)? != Decision::No {
                return Ok(Some(OperatorResult::Builtin {
                    return_type: target,
                }));
            }

            return Ok(Some(OperatorResult::NoMatch {
                reason: OperatorFailureReason::InvalidStrictEquality,
            }));
        }

        // choose primitive numeric operators
        if self.is_int32_assignable(receiver)? && self.is_int32_assignable(&argument_type)? {
            let mut target = TypeTerm::Literal(Self::int32_type());

            let _ = self.expect_type(operator.receiver, receiver, &target)?;
            let _ = self.expect_type(argument, &argument_type, &target)?;

            if Self::numeric_boolean_operator(kind) {
                target = Self::boolean_type_term();
            }
            if self.decide_expected_type(&target, expected)? == Decision::No {
                return Ok(Some(Self::operator_no_match()));
            }

            return Ok(Some(OperatorResult::Builtin {
                return_type: target,
            }));
        }

        // choose boolean logical operators
        if Self::logical_boolean_operator(kind)
            && self.is_boolean_assignable(receiver)?
            && self.is_boolean_assignable(&argument_type)?
        {
            let target = Self::boolean_type_term();

            let _ = self.expect_type(operator.receiver, receiver, &target)?;
            let _ = self.expect_type(argument, &argument_type, &target)?;
            if self.decide_expected_type(&target, expected)? == Decision::No {
                return Ok(Some(Self::operator_no_match()));
            }

            return Ok(Some(OperatorResult::Builtin {
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

    /// Return the default checked int32 type.
    pub(in crate::check) fn int32_type() -> dir::Type {
        dir::Type::Primitive(dir::PrimitiveType::Integer(dir::IntegerType::Fixed {
            width: 32,
            is_signed: true,
        }))
    }

    /// Resolve one operator method.
    fn resolve_operator_method(
        &mut self,
        operator: &OperatorTerm,
        receiver: &TypeTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<OperatorResult>> {
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
                OperatorResult::Method { .. } => return Ok(Some(result)),
                OperatorResult::Pending => is_pending = true,
                OperatorResult::NoMatch { reason: _ } => {}
                OperatorResult::Builtin { .. } => return Ok(Some(result)),
            }
        }

        if is_pending {
            Ok(Some(OperatorResult::Pending))
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
    ) -> CompilerResult<OperatorResult> {
        let key = protocol
            .method
            .key(&self.module(operator.receiver.module)?.input.strings);
        let member_protocol = self.operator_member_protocol(operator.receiver.module, &protocol)?;
        let Some(member) = self.member_type_candidate_for_protocol(
            operator.receiver.module,
            receiver,
            &key,
            &member_protocol,
        )?
        else {
            return Ok(Self::operator_no_match());
        };
        let Some(term) = self.solved_type_term(member.ty)? else {
            return Ok(OperatorResult::Pending);
        };
        let function = match self.call_signature(member.ty.module, &term)? {
            CallSignature::Pending => return Ok(OperatorResult::Pending),
            CallSignature::Absent => return Ok(Self::operator_no_match()),
            CallSignature::Present(function) => function,
        };

        let arguments = self.decide_operator_method_arguments(operator, &function)?;
        let method_return = self.decide_operator_type(&function, protocol.method_return)?;
        let expression_type = self.operator_type_term(&function, protocol.expression_type)?;
        let expected = self.decide_expected_type(&expression_type, expected)?;
        match arguments.and(method_return).and(expected) {
            Decision::Yes => {
                self.expect_operator_method_arguments(operator, &function)?;
                let _ = self.expect_operator_type(&function, protocol.method_return)?;

                Ok(OperatorResult::Method {
                    symbol: member.symbol,
                    function,
                    return_type: expression_type,
                })
            }
            Decision::Undecidable => Ok(OperatorResult::Pending),
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
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                argument,
                function.parameters[0],
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
                None => TypeTerm::Literal(dir::Type::Void),
            },
            OperatorType::Boolean => {
                TypeTerm::Literal(dir::Type::Primitive(dir::PrimitiveType::Boolean))
            }
            OperatorType::LanguageItem(item) => self.language_item_type_term(item)?,
            OperatorType::NullableLanguageItem(item) => {
                self.nullable_operator_type_term(function, self.language_item_type_term(item)?)?
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
        let target = self.push_solved_type_variable(return_type.module, target)?;

        self.relate_type_assignable(return_type, target)
    }

    /// Apply expected operator method types to accepted operands.
    fn expect_operator_method_arguments(
        &mut self,
        operator: &OperatorTerm,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        if let Some(this_parameter) = function.this_parameter
            && let Some(source) = self.solved_type_term(operator.receiver)?
            && let Some(target) = self.solved_type_term(this_parameter)?
        {
            let _ = self.expect_type(operator.receiver, &source, &target)?;
        }
        if let Some(argument) = operator.argument
            && let Some(source) = self.solved_type_term(argument)?
            && let Some(target) = self.solved_type_term(function.parameters[0])?
        {
            let _ = self.expect_type(argument, &source, &target)?;
        }

        Ok(())
    }

    /// Return whether one type can flow into int32.
    fn is_int32_assignable(&self, term: &TypeTerm) -> CompilerResult<bool> {
        let target = TypeTerm::Literal(Self::int32_type());

        Ok(
            self.decide_type_term_relation(TypeRelation::Assignable, term, &target)?
                == Decision::Yes,
        )
    }

    /// Return whether one type can flow into string.
    fn is_string_assignable(&self, term: &TypeTerm) -> CompilerResult<bool> {
        let target = TypeTerm::Literal(dir::Type::Primitive(dir::PrimitiveType::String));

        Ok(
            self.decide_type_term_relation(TypeRelation::Assignable, term, &target)?
                == Decision::Yes,
        )
    }

    /// Return whether one type can flow into boolean.
    fn is_boolean_assignable(&self, term: &TypeTerm) -> CompilerResult<bool> {
        let target = Self::boolean_type_term();

        Ok(
            self.decide_type_term_relation(TypeRelation::Assignable, term, &target)?
                == Decision::Yes,
        )
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
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                return self.supports_strict_identity(variable.module, &term);
            }
            TypeTerm::Literal(ty) => self.type_supports_strict_identity(module, ty)?,
            TypeTerm::Reference { symbol, .. } => self.symbol_supports_strict_identity(*symbol)?,
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.solved_type_term(*payload)? else {
                    return Ok(false);
                };

                return self.supports_strict_identity(payload.module, &term);
            }
            TypeTerm::Union { elements } => {
                for element in elements {
                    let Some(term) = self.solved_type_term(*element)? else {
                        return Ok(false);
                    };
                    if !self.supports_strict_identity(element.module, &term)? {
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

    /// Return whether one committed type carries scalar or reference identity.
    fn type_supports_strict_identity(
        &self,
        module: ModuleId,
        ty: &dir::Type,
    ) -> CompilerResult<bool> {
        let is_supported = match ty {
            dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Object
            | dir::Type::Literal(_)
            | dir::Type::Primitive(_)
            | dir::Type::Function(_)
            | dir::Type::Closure(_) => true,
            dir::Type::Named(named) => self.symbol_supports_strict_identity(named.symbol)?,
            dir::Type::Form(form) => {
                let ty = self.module(module)?.get_type(form.value);

                return self.type_supports_strict_identity(module, &ty);
            }
            dir::Type::Union(union) => {
                for element in &union.elements {
                    let ty = self.module(module)?.get_type(*element);
                    if !self.type_supports_strict_identity(module, &ty)? {
                        return Ok(false);
                    }
                }

                true
            }
            _ => false,
        };

        Ok(is_supported)
    }

    /// Return whether one nominal symbol carries reference identity.
    fn symbol_supports_strict_identity(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        let module = self.module(symbol.module_id)?;
        let binding_table = module.binding_table();
        let symbol = binding_table.get_symbol(symbol.local_id);

        Ok(matches!(
            symbol.form,
            dir::SymbolForm::Class | dir::SymbolForm::Function
        ))
    }

    /// Return the builtin boolean type term.
    fn boolean_type_term() -> TypeTerm {
        TypeTerm::Literal(dir::Type::Primitive(dir::PrimitiveType::Boolean))
    }

    /// Return a nominal language item type term.
    fn language_item_type_term(&self, item: dir::LanguageItem) -> CompilerResult<TypeTerm> {
        let Some(symbol) = self.environment.language.symbol(item) else {
            return Err(CompilerError::Internal {
                message: format!("missing operator language item: {item}"),
            });
        };

        Ok(TypeTerm::Reference {
            source: None,
            symbol,
            arguments: Vec::new(),
        })
    }

    /// Return a nullable protocol return type.
    fn nullable_operator_type_term(
        &mut self,
        function: &FunctionTerm,
        value: TypeTerm,
    ) -> CompilerResult<TypeTerm> {
        let module = Self::operator_type_module(function)?;
        let value = self.push_solved_type_variable(module, value)?;
        let null = self.push_solved_type_variable(module, TypeTerm::Literal(dir::Type::Null))?;

        Ok(TypeTerm::Union {
            elements: vec![value, null],
        })
    }

    /// Return the module used for synthetic operator type pieces.
    fn operator_type_module(function: &FunctionTerm) -> CompilerResult<ModuleId> {
        if let Some(return_type) = function.return_type {
            return Ok(return_type.module);
        }
        if let Some(parameter) = function.this_parameter {
            return Ok(parameter.module);
        }
        if let Some(parameter) = function.parameters.first() {
            return Ok(parameter.module);
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
        protocol: &OperatorProtocol,
    ) -> CompilerResult<MemberProtocol> {
        let mut arguments = Vec::with_capacity(protocol.arguments.len());

        for argument in &protocol.arguments {
            let argument = match argument {
                OperatorProtocolArgument::Access(access) => {
                    let value = dir::StaticTerm::Access { access: *access };
                    let variable = self.push_solved_static_value_variable(module, value)?;

                    ArgumentTerm::Static(variable)
                }
            };

            arguments.push(argument);
        }

        Ok(MemberProtocol {
            item: protocol.item,
            arguments,
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
