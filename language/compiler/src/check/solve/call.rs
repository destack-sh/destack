use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    ArgumentTerm, CallFailure, CallOutcome, CallResolution, CallResolutionTarget, CallTerm,
    CheckComponentState, FunctionTerm, GenericArgumentKey, GenericInstance, GenericSlot,
    ShapeMemberTerm, TypeRelation, TypeTerm, VariableId, VariableKind, VariableOrigin,
};
use crate::{CompilerError, CompilerResult};

use super::Decision;
use super::construct::ConstructCandidates;
use super::queue::Progress;
use super::substitute::{GenericSubstitution, GenericSubstitutionEntry};

/// Call signature extracted from a type term.
pub(super) enum CallSignature {
    /// Signature extraction is waiting for solver input.
    Pending,
    /// The term is not callable.
    Absent,
    /// The term has one function type.
    Present(FunctionTerm),
}

/// Result of resolving a runtime call target.
pub(in crate::check) enum CallResult {
    /// Call resolution is waiting for solver input.
    Pending,
    /// The callee has no callable candidates.
    NotCallable,
    /// No callable candidate accepts the arguments.
    NoMatch,
    /// One callable candidate resolved.
    Resolution {
        /// The resolved call target.
        target: CallResolutionTarget,
        /// The resolved function signature.
        function: FunctionTerm,
    },
}

/// Callable target selected before function generic instantiation.
pub(super) enum CallTargetTerm {
    /// Callable value without a declaration symbol.
    Value,
    /// Symbol-backed callable selected at compile time.
    Symbol {
        /// The resolved callable symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved receiver type for method calls.
        receiver: Option<VariableId>,
    },
    /// Constructible nominal selected through call syntax.
    Construct {
        /// The resolved constructible symbol.
        symbol: dir::GlobalSymbolId,
    },
}

/// Constructible nominal target selected through call syntax.
struct CalledConstruct {
    /// The constructed nominal type.
    ty: VariableId,
    /// The selected type symbol.
    symbol: dir::GlobalSymbolId,
}

impl CheckComponentState<'_> {
    /// Reduce one runtime call to its return type.
    pub(super) fn reduce_call_type(
        &mut self,
        module: ModuleId,
        call: &CallTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let result = self.resolve_call(call, None)?;
        let function = match &result {
            CallResult::Resolution { target, function } => {
                self.record_call_resolution(call, target.clone(), function)?;

                function
            }
            CallResult::NotCallable => {
                self.record_call_rejection(call, CallFailure::NotCallable)?;

                return Ok(None);
            }
            CallResult::NoMatch => {
                self.record_call_rejection(call, CallFailure::NoMatch)?;

                return Ok(None);
            }
            CallResult::Pending => return Ok(None),
        };

        let term = match function.return_type {
            Some(return_type) => TypeTerm::Variable(return_type),
            None => TypeTerm::Literal(dir::Type::Void),
        };
        let term = self.push_solved_type_variable(module, term)?;

        Ok(Some(TypeTerm::Variable(term)))
    }

    /// Apply an expected call result to resolved call candidates.
    pub(super) fn expect_call_result(
        &mut self,
        call: &CallTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.resolve_call(call, Some(result))?;
        let progress = match &resolved {
            CallResult::Resolution { target, function } => {
                self.record_call_resolution(call, target.clone(), function)?;

                match function.return_type {
                    Some(return_type) => self.relate_type_assignable(return_type, result)?,
                    None => Progress::Unchanged,
                }
            }
            CallResult::NotCallable => {
                self.record_call_rejection(call, CallFailure::NotCallable)?;

                Progress::Unchanged
            }
            CallResult::NoMatch => {
                self.record_call_rejection(call, CallFailure::NoMatch)?;

                Progress::Unchanged
            }
            CallResult::Pending => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Record one rejected call for diagnostics.
    pub(super) fn record_call_rejection(
        &mut self,
        call: &CallTerm,
        failure: CallFailure,
    ) -> CompilerResult<()> {
        let outcome = CallOutcome::Rejected(failure);

        self.module_mut(call.source.module_id)?
            .record_call_outcome(call.source, outcome);

        Ok(())
    }

    /// Record one resolved call for commit.
    pub(super) fn record_call_resolution(
        &mut self,
        call: &CallTerm,
        target: CallResolutionTarget,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let member_source = match self
            .module(call.callee.module)?
            .variable(call.callee)
            .origin
        {
            VariableOrigin::Node(node) if call.member.is_some() => Some(node),
            _ => None,
        };
        let resolution = CallResolution {
            source: call.source,
            member_source,
            target,
            function: function.clone(),
        };

        let outcome = CallOutcome::Resolved(resolution);

        self.module_mut(call.source.module_id)?
            .record_call_outcome(call.source, outcome);

        Ok(())
    }

    /// Resolve one runtime call target from callable candidates.
    pub(in crate::check) fn resolve_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallResult> {
        if let Some(result) = self.resolve_member_call(call, expected)? {
            return Ok(result);
        }
        if let Some(result) = self.resolve_called_construct(call, expected)? {
            return Ok(result);
        }
        if call.candidates.is_empty() {
            return self.resolve_value_call(call, expected);
        }

        let mut saw_callable = false;
        let mut saw_pending = false;

        // choose the first compatible declaration-order candidate
        for candidate in &call.candidates {
            let Some(term) = self.solved_type_term(candidate.ty)? else {
                saw_pending = true;
                continue;
            };
            match self.call_signature(candidate.ty.module, &term)? {
                CallSignature::Pending => saw_pending = true,
                CallSignature::Absent => {}
                CallSignature::Present(function) => {
                    saw_callable = true;
                    let result = self.resolve_call_signature(
                        call.callee.module,
                        call.source,
                        Some(candidate.symbol),
                        None,
                        function,
                        &call.generic_arguments,
                        &call.arguments,
                        expected,
                        CallTargetTerm::Symbol {
                            symbol: candidate.symbol,
                            receiver: None,
                        },
                    )?;
                    match result {
                        CallResult::Resolution { .. } => return Ok(result),
                        CallResult::Pending => saw_pending = true,
                        CallResult::NoMatch => {}
                        CallResult::NotCallable => {}
                    }
                }
            }
        }

        if saw_pending {
            Ok(CallResult::Pending)
        } else if saw_callable {
            Ok(CallResult::NoMatch)
        } else {
            Ok(CallResult::NotCallable)
        }
    }

    /// Select a call whose callee is an arbitrary value expression.
    fn resolve_value_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallResult> {
        let Some(term) = self.solved_type_term(call.callee)? else {
            return Ok(CallResult::Pending);
        };
        let function = match self.call_signature(call.callee.module, &term)? {
            CallSignature::Pending => return Ok(CallResult::Pending),
            CallSignature::Absent => return Ok(CallResult::NotCallable),
            CallSignature::Present(function) => function,
        };
        self.resolve_call_signature(
            call.callee.module,
            call.source,
            None,
            None,
            function,
            &call.generic_arguments,
            &call.arguments,
            expected,
            CallTargetTerm::Value,
        )
    }

    /// Select a nominal constructor exposed through call syntax.
    fn resolve_called_construct(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallResult>> {
        let Some(callee) = self.called_construct(call)? else {
            return Ok(None);
        };
        let Some(term) = self.solved_type_term(callee.ty)? else {
            return Ok(Some(CallResult::Pending));
        };
        let candidates = match self.construct_candidates(call.callee.module, callee.ty, &term)? {
            ConstructCandidates::Pending => return Ok(Some(CallResult::Pending)),
            ConstructCandidates::Absent => return Ok(None),
            ConstructCandidates::Present(candidates) => candidates,
        };
        let mut saw_pending = false;

        // choose the first compatible declaration-order construct candidate
        for candidate in candidates {
            let result = self.resolve_call_signature(
                call.callee.module,
                call.source,
                Some(callee.symbol),
                candidate.instance,
                candidate.function,
                &call.generic_arguments,
                &call.arguments,
                expected,
                CallTargetTerm::Construct {
                    symbol: candidate.symbol.unwrap_or(callee.symbol),
                },
            )?;
            match result {
                CallResult::Resolution { .. } => return Ok(Some(result)),
                CallResult::Pending => saw_pending = true,
                CallResult::NoMatch | CallResult::NotCallable => {}
            }
        }

        let result = if saw_pending {
            CallResult::Pending
        } else {
            CallResult::NoMatch
        };

        Ok(Some(result))
    }

    /// Return a constructible type selected by one call callee.
    fn called_construct(&self, call: &CallTerm) -> CompilerResult<Option<CalledConstruct>> {
        let candidates = call
            .candidates
            .iter()
            .filter_map(|candidate| {
                let symbol = self.module(candidate.symbol.module_id).ok()?;
                let binding = symbol.binding_table();
                let symbol = binding.get_symbol(candidate.symbol.local_id);

                Self::symbol_constructible(symbol.form).then_some(CalledConstruct {
                    ty: candidate.ty,
                    symbol: candidate.symbol,
                })
            })
            .collect::<Vec<_>>();

        let construct = if candidates.len() == 1 {
            candidates.into_iter().next()
        } else {
            None
        };

        Ok(construct)
    }

    /// Return whether one symbol form can construct through call syntax.
    fn symbol_constructible(form: dir::SymbolForm) -> bool {
        matches!(form, dir::SymbolForm::Newtype)
    }

    /// Select a call whose callee is a member projection.
    fn resolve_member_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallResult>> {
        let Some(member) = &call.member else {
            return Ok(None);
        };
        let Some(receiver) = self.solved_type_term(member.receiver)? else {
            return Ok(Some(CallResult::Pending));
        };
        let member_match = if let Some(protocol) = &member.protocol {
            self.member_type_candidate_for_protocol(
                member.receiver.module,
                &receiver,
                &member.key,
                protocol,
            )?
        } else {
            self.member_type_candidate(member.receiver.module, &receiver, &member.key)?
        };
        let Some(member_match) = member_match else {
            return Ok(Some(CallResult::Pending));
        };
        let Some(term) = self.solved_type_term(member_match.ty)? else {
            return Ok(Some(CallResult::Pending));
        };
        let function = match self.call_signature(member_match.ty.module, &term)? {
            CallSignature::Pending => return Ok(Some(CallResult::Pending)),
            CallSignature::Absent => return Ok(Some(CallResult::NotCallable)),
            CallSignature::Present(function) => function,
        };
        let result = self.resolve_call_signature(
            member.receiver.module,
            call.source,
            None,
            member_match.instance,
            function,
            &call.generic_arguments,
            &call.arguments,
            expected,
            CallTargetTerm::Symbol {
                symbol: member_match.symbol,
                receiver: Some(member.receiver),
            },
        )?;

        Ok(Some(result))
    }

    /// Resolve one callable signature for actual call arguments.
    pub(super) fn resolve_call_signature(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[ArgumentTerm],
        arguments: &[VariableId],
        expected: Option<VariableId>,
        target: CallTargetTerm,
    ) -> CompilerResult<CallResult> {
        let is_generic = !function.generic_parameters.is_empty();
        if generic_arguments.len() > function.generic_parameters.len()
            || (!is_generic && !generic_arguments.is_empty())
        {
            return Ok(CallResult::NoMatch);
        }
        let (function, instance) = self.instantiate_call_signature(
            module,
            source,
            owner,
            instance,
            function,
            generic_arguments,
        )?;

        // push argument types into fresh call generic variables
        if is_generic {
            self.expect_call_arguments(arguments, &function.parameters)?;
            let _ = self.expect_call_return(&function, expected)?;
        }

        let argument_decision = self.decide_call_arguments(arguments, &function.parameters)?;
        let return_type = self.decide_call_return(&function, expected)?;
        match argument_decision.and(return_type) {
            Decision::Yes => {
                self.expect_call_arguments(arguments, &function.parameters)?;
                let _ = self.expect_call_return(&function, expected)?;
                let target = Self::call_selection_target(target, instance);

                Ok(CallResult::Resolution { target, function })
            }
            Decision::Undecidable => Ok(CallResult::Pending),
            Decision::No => Ok(CallResult::NoMatch),
        }
    }

    /// Return a committed call target after generic instantiation.
    fn call_selection_target(
        target: CallTargetTerm,
        instance: Option<GenericInstance>,
    ) -> CallResolutionTarget {
        match target {
            CallTargetTerm::Value => CallResolutionTarget::Value,
            CallTargetTerm::Symbol { symbol, receiver } => CallResolutionTarget::Symbol {
                symbol,
                instance,
                receiver,
            },
            CallTargetTerm::Construct { symbol } => {
                CallResolutionTarget::Construct { symbol, instance }
            }
        }
    }

    /// Instantiate function generics as call-local inference variables.
    fn instantiate_call_signature(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[ArgumentTerm],
    ) -> CompilerResult<(FunctionTerm, Option<GenericInstance>)> {
        if function.generic_parameters.is_empty() {
            return Ok((function, instance));
        }
        let mut substitution = GenericSubstitution::empty();
        let mut arguments = Vec::with_capacity(function.generic_parameters.len());

        // use explicit arguments first, then infer the remaining call generics
        for (index, parameter) in function.generic_parameters.iter().enumerate() {
            let argument = if let Some(argument) = generic_arguments.get(index) {
                argument.clone()
            } else {
                self.instantiation_argument(module, source, *parameter)?
            };
            let symbol = self.generic_parameter_symbol(*parameter)?;

            arguments.push(argument.clone());
            substitution.entries.push(GenericSubstitutionEntry {
                variable: *parameter,
                symbol,
                argument,
            });
        }

        let mut function = self.substitute_function_term(module, &substitution, &function)?;
        function.generic_parameters.clear();
        let instance =
            instance.or_else(|| owner.map(|symbol| GenericInstance { symbol, arguments }));

        Ok((function, instance))
    }

    /// Create one inferred generic argument variable.
    fn instantiation_argument(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: VariableId,
    ) -> CompilerResult<ArgumentTerm> {
        let variable = self.instantiation_variable(module, source, parameter)?;
        let kind = self.module(variable.module)?.variable(variable).kind;
        let argument = match kind {
            VariableKind::Type => ArgumentTerm::Type(variable),
            VariableKind::Static => ArgumentTerm::Static(variable),
        };

        Ok(argument)
    }

    /// Return one stable inferred generic argument variable.
    fn instantiation_variable(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: VariableId,
    ) -> CompilerResult<VariableId> {
        let slot = self.generic_parameter_slot(parameter)?;
        let key = GenericArgumentKey {
            source,
            owner: slot.owner,
            index: slot.index,
        };
        let kind = self.module(parameter.module)?.variable(parameter).kind;
        let module = self.module_mut(module)?;

        if let Some(variable) = module.work.variables.generic_argument.get(&key).copied() {
            return Ok(variable);
        }
        let variable = module.push_variable(kind, VariableOrigin::generated());

        module.work.variables.generic_argument.insert(key, variable);

        Ok(variable)
    }

    /// Return the generic slot for one generic parameter.
    fn generic_parameter_slot(&self, parameter: VariableId) -> CompilerResult<GenericSlot> {
        let variable = self.module(parameter.module)?.variable(parameter);
        let VariableOrigin::Generic(generic) = &variable.origin else {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter:?} is not a generic slot"),
            });
        };

        Ok(generic.slot().clone())
    }

    /// Return the source symbol for one explicit generic parameter.
    fn generic_parameter_symbol(
        &self,
        parameter: VariableId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let variable = self.module(parameter.module)?.variable(parameter);
        let symbol = match &variable.origin {
            VariableOrigin::Generic(generic) => match generic.slot().key {
                dir::GenericSlotKey::Symbol(symbol) => Some(symbol),
                dir::GenericSlotKey::Generated(_) => None,
            },
            _ => None,
        };

        Ok(symbol)
    }

    /// Return the callable signature represented by one type term.
    pub(super) fn call_signature(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
    ) -> CompilerResult<CallSignature> {
        let callable = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(CallSignature::Pending);
                };

                return self.call_signature(variable.module, &term);
            }
            TypeTerm::Function(function) => CallSignature::Present(function.clone()),
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.named_call_signature(module, *symbol, arguments)?,
            TypeTerm::Literal(dir::Type::Function(function)) => {
                CallSignature::Present(self.committed_function_signature(module, function)?)
            }
            TypeTerm::Literal(dir::Type::Named(named)) => {
                self.committed_named_call_signature(module, named)?
            }
            TypeTerm::Literal(dir::Type::Shape(shape)) => {
                self.committed_shape_call_signature(module, shape)?
            }
            TypeTerm::Shape { members } => self.shape_call_signature(module, members)?,
            _ => CallSignature::Absent,
        };

        Ok(callable)
    }

    /// Return a callable signature for a named language item term.
    fn named_call_signature(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
    ) -> CompilerResult<CallSignature> {
        if self.environment.language.item(symbol) != Some(dir::LanguageItem::Function) {
            return Ok(CallSignature::Absent);
        }
        let Some(parameters) = Self::type_argument(arguments, 0) else {
            return Ok(CallSignature::Pending);
        };
        let Some(return_type) = Self::type_argument(arguments, 1) else {
            return Ok(CallSignature::Pending);
        };

        self.function_language_item_signature(module, parameters, return_type)
    }

    /// Return a callable signature for a committed named language item.
    fn committed_named_call_signature(
        &mut self,
        module: ModuleId,
        named: &dir::NamedType,
    ) -> CompilerResult<CallSignature> {
        if self.environment.language.item(named.symbol) != Some(dir::LanguageItem::Function) {
            return Ok(CallSignature::Absent);
        }
        let Some(parameters) = named.arguments.first() else {
            return Ok(CallSignature::Pending);
        };
        let Some(return_type) = named.arguments.get(1) else {
            return Ok(CallSignature::Pending);
        };
        let parameters = self.type_static_argument_variable(module, parameters)?;
        let return_type = self.type_static_argument_variable(module, return_type)?;

        self.function_language_item_signature(module, parameters, return_type)
    }

    /// Return the call signature for one Function language item.
    fn function_language_item_signature(
        &mut self,
        module: ModuleId,
        parameters: VariableId,
        return_type: VariableId,
    ) -> CompilerResult<CallSignature> {
        let Some(parameters) = self.call_parameters_from_tuple(module, parameters)? else {
            return Ok(CallSignature::Pending);
        };

        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters,
            return_type: Some(return_type),
            is_generator: false,
        };

        Ok(CallSignature::Present(function))
    }

    /// Return the call signature for a committed shape.
    fn committed_shape_call_signature(
        &mut self,
        module: ModuleId,
        shape: &dir::ShapeType,
    ) -> CompilerResult<CallSignature> {
        let Some(signature) = shape.call_signatures.first().copied() else {
            return Ok(CallSignature::Absent);
        };
        let variable = self.type_id_variable(module, signature)?;
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(CallSignature::Pending);
        };

        self.call_signature(module, &term)
    }

    /// Return the call signature for a shape term.
    fn shape_call_signature(
        &mut self,
        module: ModuleId,
        members: &[ShapeMemberTerm],
    ) -> CompilerResult<CallSignature> {
        for member in members {
            let ShapeMemberTerm::CallSignature { ty } = member else {
                continue;
            };
            let Some(term) = self.solved_type_term(*ty)? else {
                return Ok(CallSignature::Pending);
            };

            return self.call_signature(module, &term);
        }

        Ok(CallSignature::Absent)
    }

    /// Return function parameters from a tuple type variable.
    fn call_parameters_from_tuple(
        &mut self,
        module: ModuleId,
        variable: VariableId,
    ) -> CompilerResult<Option<Vec<VariableId>>> {
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(None);
        };
        let parameters = match term {
            TypeTerm::Tuple { elements, .. } => elements.iter().map(|element| element.ty).collect(),
            TypeTerm::Literal(dir::Type::Tuple(tuple)) => tuple
                .elements
                .iter()
                .map(|element| self.type_id_variable(module, element.ty))
                .collect::<CompilerResult<Vec<_>>>()?,
            TypeTerm::Literal(dir::Type::Void) => Vec::new(),
            _ => return Ok(None),
        };

        Ok(Some(parameters))
    }

    /// Return one type variable from a committed static type argument.
    pub(super) fn type_static_argument_variable(
        &mut self,
        module: ModuleId,
        argument: &dir::StaticArgument,
    ) -> CompilerResult<VariableId> {
        let value = self.module(module)?.get_static(argument.value);
        let dir::StaticTerm::Type { ty } = value else {
            return self.push_solved_type_variable(module, TypeTerm::Literal(dir::Type::Error));
        };

        self.type_id_variable(module, ty)
    }

    /// Return one function term from a committed DIR function type.
    fn committed_function_signature(
        &mut self,
        module: ModuleId,
        function: &dir::FunctionType,
    ) -> CompilerResult<FunctionTerm> {
        Ok(FunctionTerm {
            asynchrony: function.asynchrony,
            generic_parameters: function
                .generic_parameters
                .iter()
                .map(|ty| self.type_id_variable(module, *ty))
                .collect::<CompilerResult<Vec<_>>>()?,
            this_parameter: function
                .this_parameter
                .map(|ty| self.type_id_variable(module, ty))
                .transpose()?,
            parameters: function
                .parameters
                .iter()
                .map(|ty| self.type_id_variable(module, *ty))
                .collect::<CompilerResult<Vec<_>>>()?,
            return_type: function
                .return_type
                .map(|ty| self.type_id_variable(module, ty))
                .transpose()?,
            is_generator: function.is_generator,
        })
    }

    /// Decide whether arguments are assignable to parameters.
    fn decide_call_arguments(
        &self,
        arguments: &[VariableId],
        parameters: &[VariableId],
    ) -> CompilerResult<Decision> {
        if arguments.len() != parameters.len() {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // every argument must be assignable to the corresponding parameter
        for (argument, parameter) in arguments.iter().zip(parameters) {
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                *argument,
                *parameter,
            )?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether the return type can flow into the expected result.
    fn decide_call_return(
        &self,
        function: &FunctionTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Decision> {
        let Some(expected) = expected else {
            return Ok(Decision::Yes);
        };
        let Some(return_type) = function.return_type else {
            let Some(expected) = self.solved_type_term(expected)? else {
                return Ok(Decision::Undecidable);
            };
            let void = TypeTerm::Literal(dir::Type::Void);

            return self.decide_type_term_relation(TypeRelation::Assignable, &void, &expected);
        };

        self.decide_type_relation(TypeRelation::Assignable, return_type, expected)
    }

    /// Apply expected parameter types to accepted call arguments.
    fn expect_call_arguments(
        &mut self,
        arguments: &[VariableId],
        parameters: &[VariableId],
    ) -> CompilerResult<()> {
        for (argument, parameter) in arguments.iter().zip(parameters) {
            let _ = self.relate_type_assignable(*argument, *parameter)?;
        }

        Ok(())
    }

    /// Apply expected result type to an accepted call return.
    fn expect_call_return(
        &mut self,
        function: &FunctionTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Progress> {
        let Some(expected) = expected else {
            return Ok(Progress::Unchanged);
        };
        let Some(return_type) = function.return_type else {
            let void = TypeTerm::Literal(dir::Type::Void);
            let void = self.push_solved_type_variable(expected.module, void)?;

            return self.relate_type_assignable(void, expected);
        };

        self.relate_type_assignable(return_type, expected)
    }
}
