use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    ArgumentTerm, CallFailure, CallOutcome, CallResolution, CallResolutionTarget,
    CheckComponentState, ConstructCandidates, FunctionTerm, GenericArgumentKey, GenericInstance,
    GenericSlot, GenericSubstitution, GenericSubstitutionEntry, Progress, ShapeMemberTerm,
    TypeLiteralTerm, TypeRelation, TypeTerm, VariableId, VariableKind, VariableOrigin,
};
use crate::{CompilerError, CompilerResult};

use crate::check::Decision;

/// Runtime call expression term.
///
/// ```ts
/// format(value)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct CallTerm {
    /// The source call expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The called expression type.
    pub(in crate::check) callee: VariableId,
    /// The member projection that produced this callee.
    pub(in crate::check) member: Option<MemberCallTerm>,
    /// The symbol-backed candidates visible at the call site.
    pub(in crate::check) candidates: Vec<CallCandidateTerm>,
    /// The explicit call generic arguments.
    pub(in crate::check) generic_arguments: Vec<ArgumentTerm>,
    /// The argument expression types.
    pub(in crate::check) arguments: Vec<VariableId>,
}

/// Symbol-backed callable candidate.
///
/// ```ts
/// value.toString()
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CallCandidateTerm {
    /// The selected callable target.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The candidate callable type.
    pub(in crate::check) ty: VariableId,
}

/// Member projection used as a runtime call callee.
///
/// ```ts
/// value.toString()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberCallTerm {
    /// The receiver type.
    pub(in crate::check) receiver: VariableId,
    /// The selected member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The applied static arguments.
    pub(in crate::check) arguments: Vec<ArgumentTerm>,
    /// The protocol that must own the resolved method.
    pub(in crate::check) protocol: Option<MemberProtocol>,
}

/// Protocol contract required for a member resolution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberProtocol {
    /// The protocol language item.
    pub(in crate::check) item: dir::LanguageItem,
    /// The required protocol arguments.
    pub(in crate::check) arguments: Vec<ArgumentTerm>,
}

impl CallTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        if self.member.is_none() && self.candidates.is_empty() {
            variables.push(self.callee);
        }
        if !self.candidates.is_empty() {
            variables.extend(self.candidates.iter().map(|candidate| candidate.ty));
        }
        if let Some(member) = &self.member {
            variables.push(member.receiver);
            variables.extend(member.arguments.iter().map(ArgumentTerm::variable));
        }
        variables.extend(self.generic_arguments.iter().map(ArgumentTerm::variable));
        variables.extend(self.arguments.iter().copied());

        variables
    }
}

/// Callable function signature extracted from a type term.
pub(in crate::check) enum CallableSignature {
    /// Signature extraction is waiting for solver input.
    Pending,
    /// The term is not callable.
    Absent,
    /// The term has one function type.
    Present(FunctionTerm),
}

/// Transient callable selection while reducing call-like syntax.
pub(in crate::check) enum CallableSelection {
    /// Call resolution is waiting for solver input.
    Pending,
    /// The callee has no callable candidates.
    NotCallable,
    /// No callable candidate accepts the arguments.
    NoMatch,
    /// One callable candidate resolved.
    Resolved {
        /// The resolved call target.
        target: CallResolutionTarget,
        /// The resolved function signature.
        function: FunctionTerm,
    },
}

/// Callable target selected before generic instantiation.
pub(in crate::check) enum CallTarget {
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

impl CallTarget {
    /// Convert this selected target into a resolved call target.
    fn into_resolution_target(self, instance: Option<GenericInstance>) -> CallResolutionTarget {
        match self {
            Self::Value => CallResolutionTarget::Value,
            Self::Symbol { symbol, receiver } => CallResolutionTarget::Symbol {
                symbol,
                instance,
                receiver,
            },
            Self::Construct { symbol } => CallResolutionTarget::Construct { symbol, instance },
        }
    }
}

/// Constructible callee selected through call syntax.
struct ConstructibleCallee {
    /// The constructed nominal type.
    ty: VariableId,
    /// The selected type symbol.
    symbol: dir::GlobalSymbolId,
}

impl CheckComponentState<'_> {
    /// Reduce one runtime call to its return type.
    pub(in crate::check) fn reduce_call_type(
        &mut self,
        module: ModuleId,
        call: &CallTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let result = self.resolve_call(call, None)?;
        let function = match &result {
            CallableSelection::Resolved { target, function } => {
                self.record_call_resolution(call, target.clone(), function)?;

                function
            }
            CallableSelection::NotCallable => {
                self.record_call_rejection(call, CallFailure::NotCallable)?;

                return Ok(None);
            }
            CallableSelection::NoMatch => {
                self.record_call_rejection(call, CallFailure::NoMatch)?;

                return Ok(None);
            }
            CallableSelection::Pending => return Ok(None),
        };

        let term = match function.return_type {
            Some(return_type) => TypeTerm::Variable(return_type),
            None => TypeTerm::Literal(TypeLiteralTerm::Void),
        };
        let term = self.push_solved_type_variable(module, term)?;

        Ok(Some(TypeTerm::Variable(term)))
    }

    /// Apply an expected call result to resolved call candidates.
    pub(in crate::check) fn expect_call_result(
        &mut self,
        call: &CallTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.resolve_call(call, Some(result))?;
        let progress = match &resolved {
            CallableSelection::Resolved { target, function } => {
                self.record_call_resolution(call, target.clone(), function)?;

                match function.return_type {
                    Some(return_type) => self.relate_type_assignable(return_type, result)?,
                    None => Progress::Unchanged,
                }
            }
            CallableSelection::NotCallable => {
                self.record_call_rejection(call, CallFailure::NotCallable)?;

                Progress::Unchanged
            }
            CallableSelection::NoMatch => {
                self.record_call_rejection(call, CallFailure::NoMatch)?;

                Progress::Unchanged
            }
            CallableSelection::Pending => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Record one rejected call for diagnostics.
    pub(in crate::check) fn record_call_rejection(
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
    pub(in crate::check) fn record_call_resolution(
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
    ) -> CompilerResult<CallableSelection> {
        if let Some(result) = self.resolve_member_call(call, expected)? {
            return Ok(result);
        }
        if let Some(result) = self.resolve_construct_call(call, expected)? {
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
                CallableSignature::Pending => saw_pending = true,
                CallableSignature::Absent => {}
                CallableSignature::Present(function) => {
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
                        CallTarget::Symbol {
                            symbol: candidate.symbol,
                            receiver: None,
                        },
                    )?;
                    match result {
                        CallableSelection::Resolved { .. } => return Ok(result),
                        CallableSelection::Pending => saw_pending = true,
                        CallableSelection::NoMatch => {}
                        CallableSelection::NotCallable => {}
                    }
                }
            }
        }

        if saw_pending {
            Ok(CallableSelection::Pending)
        } else if saw_callable {
            Ok(CallableSelection::NoMatch)
        } else {
            Ok(CallableSelection::NotCallable)
        }
    }

    /// Select a call whose callee is an arbitrary value expression.
    fn resolve_value_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableSelection> {
        let Some(term) = self.solved_type_term(call.callee)? else {
            return Ok(CallableSelection::Pending);
        };
        let function = match self.call_signature(call.callee.module, &term)? {
            CallableSignature::Pending => return Ok(CallableSelection::Pending),
            CallableSignature::Absent => return Ok(CallableSelection::NotCallable),
            CallableSignature::Present(function) => function,
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
            CallTarget::Value,
        )
    }

    /// Select a nominal constructor exposed through call syntax.
    fn resolve_construct_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableSelection>> {
        let Some(callee) = self.constructible_callee(call)? else {
            return Ok(None);
        };
        let Some(term) = self.solved_type_term(callee.ty)? else {
            return Ok(Some(CallableSelection::Pending));
        };
        let candidates = match self.construct_candidates(call.callee.module, callee.ty, &term)? {
            ConstructCandidates::Pending => return Ok(Some(CallableSelection::Pending)),
            ConstructCandidates::Absent => return Ok(None),
            ConstructCandidates::Present(candidates) => candidates,
        };
        let mut saw_pending = false;

        // choose the first compatible declaration-order construct candidate
        for candidate in candidates {
            let target_symbol = match candidate.symbol {
                Some(symbol) => symbol,
                None => callee.symbol,
            };
            let result = self.resolve_call_signature(
                call.callee.module,
                call.source,
                Some(callee.symbol),
                candidate.instance,
                candidate.function,
                &call.generic_arguments,
                &call.arguments,
                expected,
                CallTarget::Construct {
                    symbol: target_symbol,
                },
            )?;
            match result {
                CallableSelection::Resolved { .. } => return Ok(Some(result)),
                CallableSelection::Pending => saw_pending = true,
                CallableSelection::NoMatch | CallableSelection::NotCallable => {}
            }
        }

        let result = if saw_pending {
            CallableSelection::Pending
        } else {
            CallableSelection::NoMatch
        };

        Ok(Some(result))
    }

    /// Return a constructible type selected by one call callee.
    fn constructible_callee(&self, call: &CallTerm) -> CompilerResult<Option<ConstructibleCallee>> {
        let candidates = call
            .candidates
            .iter()
            .filter_map(|candidate| {
                let symbol = self.module(candidate.symbol.module_id).ok()?;
                let binding = symbol.binding_table();
                let symbol = binding.get_symbol(candidate.symbol.local_id);

                Self::symbol_constructible(symbol.form).then_some(ConstructibleCallee {
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
    ) -> CompilerResult<Option<CallableSelection>> {
        let Some(member) = &call.member else {
            return Ok(None);
        };
        let Some(receiver) = self.solved_type_term(member.receiver)? else {
            return Ok(Some(CallableSelection::Pending));
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
            return Ok(Some(CallableSelection::Pending));
        };
        let Some(term) = self.solved_type_term(member_match.ty)? else {
            return Ok(Some(CallableSelection::Pending));
        };
        let function = match self.call_signature(member_match.ty.module, &term)? {
            CallableSignature::Pending => return Ok(Some(CallableSelection::Pending)),
            CallableSignature::Absent => return Ok(Some(CallableSelection::NotCallable)),
            CallableSignature::Present(function) => function,
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
            CallTarget::Symbol {
                symbol: member_match.symbol,
                receiver: Some(member.receiver),
            },
        )?;

        Ok(Some(result))
    }

    /// Resolve one callable signature for actual call arguments.
    pub(in crate::check) fn resolve_call_signature(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[ArgumentTerm],
        arguments: &[VariableId],
        expected: Option<VariableId>,
        target: CallTarget,
    ) -> CompilerResult<CallableSelection> {
        let is_generic = !function.generic_parameters.is_empty();
        if generic_arguments.len() > function.generic_parameters.len()
            || (!is_generic && !generic_arguments.is_empty())
        {
            return Ok(CallableSelection::NoMatch);
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
            self.expect_call_return(&function, expected)?;
        }

        let argument_decision = self.decide_call_arguments(arguments, &function.parameters)?;
        let return_type = self.decide_call_return(&function, expected)?;
        match argument_decision.and(return_type) {
            Decision::Yes => {
                self.expect_call_arguments(arguments, &function.parameters)?;
                self.expect_call_return(&function, expected)?;
                let target = target.into_resolution_target(instance);

                Ok(CallableSelection::Resolved { target, function })
            }
            Decision::Undecidable => Ok(CallableSelection::Pending),
            Decision::No => Ok(CallableSelection::NoMatch),
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

        let mut function = function.substitute(module, &substitution, self)?;
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
    pub(in crate::check) fn call_signature(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
    ) -> CompilerResult<CallableSignature> {
        let callable = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(CallableSignature::Pending);
                };

                return self.call_signature(variable.module, &term);
            }
            TypeTerm::Function(function) => CallableSignature::Present(function.clone()),
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.named_call_signature(*symbol, arguments)?,
            TypeTerm::Shape { members } => self.shape_call_signature(module, members)?,
            _ => CallableSignature::Absent,
        };

        Ok(callable)
    }

    /// Return a callable signature for a named language item term.
    fn named_call_signature(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
    ) -> CompilerResult<CallableSignature> {
        if self.environment.language.item(symbol) != Some(dir::LanguageItem::Function) {
            return Ok(CallableSignature::Absent);
        }
        let Some(parameters) = Self::type_argument(arguments, 0) else {
            return Ok(CallableSignature::Pending);
        };
        let Some(return_type) = Self::type_argument(arguments, 1) else {
            return Ok(CallableSignature::Pending);
        };

        self.function_language_item_signature(parameters, return_type)
    }

    /// Return the call signature for one Function language item.
    fn function_language_item_signature(
        &mut self,
        parameters: VariableId,
        return_type: VariableId,
    ) -> CompilerResult<CallableSignature> {
        let Some(parameters) = self.call_parameters_from_tuple(parameters)? else {
            return Ok(CallableSignature::Pending);
        };

        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters,
            return_type: Some(return_type),
            is_generator: false,
        };

        Ok(CallableSignature::Present(function))
    }

    /// Return the call signature for a shape term.
    fn shape_call_signature(
        &mut self,
        module: ModuleId,
        members: &[ShapeMemberTerm],
    ) -> CompilerResult<CallableSignature> {
        for member in members {
            let ShapeMemberTerm::CallSignature { ty } = member else {
                continue;
            };
            let Some(term) = self.solved_type_term(*ty)? else {
                return Ok(CallableSignature::Pending);
            };

            return self.call_signature(module, &term);
        }

        Ok(CallableSignature::Absent)
    }

    /// Return function parameters from a tuple type variable.
    fn call_parameters_from_tuple(
        &mut self,
        variable: VariableId,
    ) -> CompilerResult<Option<Vec<VariableId>>> {
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(None);
        };
        let parameters = match term {
            TypeTerm::Tuple { elements, .. } => elements.iter().map(|element| element.ty).collect(),
            TypeTerm::Literal(TypeLiteralTerm::Void) => Vec::new(),
            _ => return Ok(None),
        };

        Ok(Some(parameters))
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
            let void = TypeTerm::Literal(TypeLiteralTerm::Void);

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
            self.relate_type_assignable(*argument, *parameter)?;
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
            let void = TypeTerm::Literal(TypeLiteralTerm::Void);
            let void = self.push_solved_type_variable(expected.module, void)?;

            return self.relate_type_assignable(void, expected);
        };

        self.relate_type_assignable(return_type, expected)
    }
}
