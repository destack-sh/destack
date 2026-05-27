use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CallDecision, CallFailure, CallResolution, CallResolutionTarget, CheckState, ConstraintOrigin,
    ConstructCandidates, FunctionParameter, FunctionTerm, GenericArgument, GenericArgumentKey,
    GenericInstance, GenericParameter, GenericSlot, GenericSubstitution, GenericSubstitutionEntry,
    Progress, Reduction, ShapeMember, StaticTerm, TermId, TypeLiteralTerm, TypeOperand,
    TypeRelation, TypeTerm, VariableId, VariableKind, VariableOutput,
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
    pub(in crate::check) member: Option<TermId<MemberCallTerm>>,
    /// The symbol-backed candidates visible at the call site.
    pub(in crate::check) candidates: Vec<CallCandidate>,
    /// The explicit call generic arguments.
    pub(in crate::check) generic_arguments: SmallVec<[GenericArgument; 4]>,
    /// The argument expression types.
    pub(in crate::check) arguments: SmallVec<[TypeOperand; 4]>,
}

/// Symbol-backed callable candidate.
///
/// ```ts
/// value.toString()
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CallCandidate {
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
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 4]>,
    /// The protocol that must own the resolved method.
    pub(in crate::check) protocol: Option<MemberProtocol>,
}

/// Protocol contract required for a member resolution.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MemberProtocol {
    /// The protocol language item.
    pub(in crate::check) item: dir::LanguageItem,
    /// The required protocol arguments.
    pub(in crate::check) arguments: SmallVec<[GenericArgument; 4]>,
}

impl CallTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();

        if self.member.is_none() && self.candidates.is_empty() {
            variables.push(self.callee);
        }
        if !self.candidates.is_empty() {
            variables.extend(self.candidates.iter().map(|candidate| candidate.ty));
        }
        if let Some(member) = self.member {
            let member = state.terms.get(member);

            variables.push(member.receiver);
            variables.extend(
                member
                    .arguments
                    .iter()
                    .flat_map(|argument| state.argument_variables(argument)),
            );
        }
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| state.argument_variables(argument)),
        );
        variables.extend(
            self.arguments
                .iter()
                .flat_map(|argument| argument.referenced_variables(state)),
        );

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
    Pending {
        /// The variable changes made before resolution became pending.
        progress: Progress,
    },
    /// Call resolution failed.
    Rejected(CallFailure),
    /// One callable candidate resolved.
    Resolved {
        /// The resolved call target.
        target: CallResolutionTarget,
        /// The resolved function signature.
        function: FunctionTerm,
        /// The variable changes made while resolving.
        progress: Progress,
    },
}

impl CallableSelection {
    /// Return a pending selection with no side progress.
    pub(in crate::check) fn pending() -> Self {
        Self::Pending {
            progress: Progress::Unchanged,
        }
    }

    /// Return a resolved selection with side progress.
    pub(in crate::check) fn resolved(
        target: CallResolutionTarget,
        function: FunctionTerm,
        progress: Progress,
    ) -> Self {
        Self::Resolved {
            target,
            function,
            progress,
        }
    }

    /// Return a rejected selection.
    pub(in crate::check) fn rejected(failure: CallFailure) -> Self {
        Self::Rejected(failure)
    }

    /// Return progress made while selecting.
    pub(in crate::check) fn progress(&self) -> Progress {
        match self {
            Self::Pending { progress } | Self::Resolved { progress, .. } => progress.clone(),
            Self::Rejected(_) => Progress::Unchanged,
        }
    }
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
    /// Constructor selected through call syntax.
    Constructor {
        /// The resolved constructor symbol.
        symbol: dir::GlobalSymbolId,
    },
}

impl CallTarget {
    /// Convert this selected target into a resolved call target.
    pub(in crate::check) fn into_resolution_target(
        self,
        instance: Option<GenericInstance>,
    ) -> CallResolutionTarget {
        match self {
            Self::Value => CallResolutionTarget::Value,
            Self::Symbol { symbol, receiver } => CallResolutionTarget::Symbol {
                symbol,
                instance,
                receiver,
            },
            Self::Constructor { symbol } => CallResolutionTarget::Constructor { symbol, instance },
        }
    }
}

/// Newtype callee selected through call syntax.
struct NewtypeCallee {
    /// The constructed nominal type.
    ty: VariableId,
    /// The selected type symbol.
    symbol: dir::GlobalSymbolId,
}

/// Function signature after call generic substitution.
struct CallInstantiation {
    /// The instantiated function signature.
    function: FunctionTerm,
    /// The resolved generic instance.
    instance: Option<GenericInstance>,
    /// The substitution used for this call.
    substitution: GenericSubstitution,
    /// The original generic parameters.
    generic_parameters: Vec<VariableId>,
}

impl CheckState<'_> {
    /// Reduce one generic function value reference to its specialized function type.
    pub(in crate::check) fn reduce_function_reference_term(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[GenericArgument],
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let value = self.symbol_type_variable(module, symbol)?;
        let Some(term) = self.solved_type_term(value)? else {
            return Ok(Reduction::pending());
        };
        let function = match self.call_signature(module, &term)? {
            CallableSignature::Pending => return Ok(Reduction::pending()),
            CallableSignature::Absent => {
                return Ok(Reduction::value(TypeTerm::Reference {
                    source: Some(source),
                    symbol,
                    arguments: generic_arguments.to_vec().into(),
                }));
            }
            CallableSignature::Present(function) => function,
        };
        let instantiation = self.instantiate_call_signature(
            module,
            source,
            Some(symbol),
            None,
            function,
            generic_arguments,
        )?;

        let function = self.terms.push(instantiation.function);

        Ok(Reduction::value(TypeTerm::Function(function)))
    }

    /// Reduce one runtime call to its return type.
    pub(in crate::check) fn reduce_call_term(
        &mut self,
        _module: ModuleId,
        call: &CallTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let result = self.resolve_call(call, None)?;
        let progress = result.progress();
        let function = match &result {
            CallableSelection::Resolved {
                target, function, ..
            } => {
                self.record_call_resolution(call, target.clone(), function)?;

                function
            }
            CallableSelection::Rejected(failure) => {
                self.record_call_rejection(call, failure.clone())?;

                return Ok(Reduction::progress(progress));
            }
            CallableSelection::Pending { .. } => return Ok(Reduction::progress(progress)),
        };

        let term = match function.return_type {
            Some(return_type) => TypeTerm::Variable(return_type),
            None => TypeTerm::Literal(TypeLiteralTerm::Void),
        };
        Ok(Reduction {
            value: Some(term),
            progress,
        })
    }

    /// Expect resolved call candidates to produce the expected result.
    pub(in crate::check) fn expect_call_term(
        &mut self,
        call: &CallTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.resolve_call(call, Some(result))?;
        let progress = resolved.progress();
        let progress = match &resolved {
            CallableSelection::Resolved {
                target, function, ..
            } => {
                self.record_call_resolution(call, target.clone(), function)?;

                match function.return_type {
                    Some(return_type) => {
                        progress.merge(self.solve_type_assignability(return_type, result)?)
                    }
                    None => progress,
                }
            }
            CallableSelection::Rejected(failure) => {
                self.record_call_rejection(call, failure.clone())?;

                progress
            }
            CallableSelection::Pending { .. } => progress,
        };

        Ok(progress)
    }

    /// Record one rejected call for diagnostics.
    pub(in crate::check) fn record_call_rejection(
        &mut self,
        call: &CallTerm,
        failure: CallFailure,
    ) -> CompilerResult<()> {
        let decision = CallDecision::Rejected(failure);

        self.record_call_decision(call.source, decision);

        Ok(())
    }

    /// Record one resolved call for commit.
    pub(in crate::check) fn record_call_resolution(
        &mut self,
        call: &CallTerm,
        target: CallResolutionTarget,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let member_source = match self.variable(call.callee).output {
            Some(VariableOutput::Node(node)) if call.member.is_some() => Some(node),
            _ => None,
        };
        let resolution = CallResolution {
            source: call.source,
            member_source,
            target,
            function: function.clone(),
        };

        let decision = CallDecision::Resolved(resolution);

        self.record_call_decision(call.source, decision);

        Ok(())
    }

    /// Resolve one runtime call target from callable candidates.
    pub(in crate::check) fn resolve_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableSelection> {
        if let Some(selection) = self.recorded_call_selection(call)? {
            return Ok(selection);
        }
        if let Some(result) = self.resolve_member_call(call, expected)? {
            return Ok(result);
        }
        if let Some(result) = self.resolve_newtype_call(call, expected)? {
            return Ok(result);
        }
        if call.candidates.is_empty() {
            return self.resolve_value_call(call, expected);
        }

        let mut saw_callable = false;
        let mut saw_pending = false;
        let mut argument_failure = None;

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
                        CallableSelection::Pending { .. } => saw_pending = true,
                        CallableSelection::Rejected(CallFailure::ArgumentType {
                            argument,
                            parameter,
                        }) => {
                            argument_failure.get_or_insert((argument, parameter));
                        }
                        CallableSelection::Rejected(CallFailure::NoMatch) => {}
                        CallableSelection::Rejected(CallFailure::NotCallable) => {}
                    }
                }
            }
        }

        if saw_pending {
            Ok(CallableSelection::pending())
        } else if let Some((argument, parameter)) = argument_failure {
            Ok(CallableSelection::rejected(CallFailure::ArgumentType {
                argument,
                parameter,
            }))
        } else if saw_callable {
            Ok(CallableSelection::rejected(CallFailure::NoMatch))
        } else {
            Ok(CallableSelection::rejected(CallFailure::NotCallable))
        }
    }

    /// Return the already chosen decision for one call.
    fn recorded_call_selection(
        &self,
        call: &CallTerm,
    ) -> CompilerResult<Option<CallableSelection>> {
        let Some(decision) = self.solutions.call.get(&call.source).cloned() else {
            return Ok(None);
        };

        let selection = match decision {
            CallDecision::Resolved(resolution) => CallableSelection::resolved(
                resolution.target,
                resolution.function,
                Progress::Unchanged,
            ),
            CallDecision::Rejected(failure) => CallableSelection::rejected(failure),
        };

        Ok(Some(selection))
    }

    /// Select a call whose callee is an arbitrary value expression.
    fn resolve_value_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableSelection> {
        let Some(term) = self.solved_type_term(call.callee)? else {
            return Ok(CallableSelection::pending());
        };
        let function = match self.call_signature(call.callee.module, &term)? {
            CallableSignature::Pending => return Ok(CallableSelection::pending()),
            CallableSignature::Absent => {
                return Ok(CallableSelection::rejected(CallFailure::NotCallable));
            }
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

    /// Select a newtype constructor exposed through call syntax.
    fn resolve_newtype_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableSelection>> {
        let Some(callee) = self.newtype_callee(call)? else {
            return Ok(None);
        };
        let Some(term) = self.solved_type_term(callee.ty)? else {
            return Ok(Some(CallableSelection::pending()));
        };
        let candidates = match self.construct_candidates(call.callee.module, callee.ty, &term)? {
            ConstructCandidates::Pending => return Ok(Some(CallableSelection::pending())),
            ConstructCandidates::Absent => return Ok(None),
            ConstructCandidates::Present(candidates) => candidates,
        };
        let mut saw_pending = false;

        // choose the first compatible declaration-order newtype candidate
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
                CallTarget::Constructor {
                    symbol: target_symbol,
                },
            )?;
            match result {
                CallableSelection::Resolved { .. } => return Ok(Some(result)),
                CallableSelection::Pending { .. } => saw_pending = true,
                CallableSelection::Rejected(_) => {}
            }
        }

        let result = if saw_pending {
            CallableSelection::pending()
        } else {
            CallableSelection::rejected(CallFailure::NoMatch)
        };

        Ok(Some(result))
    }

    /// Return the newtype selected by one call callee.
    fn newtype_callee(&self, call: &CallTerm) -> CompilerResult<Option<NewtypeCallee>> {
        let candidates = call
            .candidates
            .iter()
            .filter_map(|candidate| {
                let input = self.inputs.get(&candidate.symbol.module_id)?;
                let binding = input.binding_table();
                let symbol = binding.get_symbol(candidate.symbol.local_id);

                matches!(symbol.kind, dir::SymbolKind::Newtype).then_some(NewtypeCallee {
                    ty: candidate.ty,
                    symbol: candidate.symbol,
                })
            })
            .collect::<Vec<_>>();

        let callee = if candidates.len() == 1 {
            candidates.into_iter().next()
        } else {
            None
        };

        Ok(callee)
    }

    /// Select a call whose callee is a member projection.
    fn resolve_member_call(
        &mut self,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableSelection>> {
        let Some(member) = call.member else {
            return Ok(None);
        };
        let member = self.terms.get(member).clone();
        let Some(receiver) = self.solved_type_term(member.receiver)? else {
            return Ok(Some(CallableSelection::pending()));
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
            return Ok(Some(CallableSelection::pending()));
        };
        let reduction = self.reduce_type_term(member.receiver.module, &member_match.ty)?;
        let Some(term) = reduction.value else {
            return Ok(Some(CallableSelection::pending()));
        };
        let function = match self.call_signature(member.receiver.module, &term)? {
            CallableSignature::Pending => return Ok(Some(CallableSelection::pending())),
            CallableSignature::Absent => {
                return Ok(Some(CallableSelection::rejected(CallFailure::NotCallable)));
            }
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
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        expected: Option<VariableId>,
        target: CallTarget,
    ) -> CompilerResult<CallableSelection> {
        let is_generic = !function.generic_parameters.is_empty();
        if generic_arguments.len() > function.generic_parameters.len()
            || (!is_generic && !generic_arguments.is_empty())
        {
            return Ok(CallableSelection::rejected(CallFailure::NoMatch));
        }
        let instantiation = self.instantiate_call_signature(
            module,
            source,
            owner,
            instance,
            function,
            generic_arguments,
        )?;
        let function = instantiation.function;
        let mut progress = Progress::Unchanged;

        // push argument types into fresh call generic variables
        if is_generic {
            progress = progress.merge(self.expect_call_arguments(arguments, &function.parameters)?);
            progress = progress.merge(self.expect_call_return(&function, expected)?);
            self.solve_defaulted_generic_arguments(
                module,
                &instantiation.substitution,
                &instantiation.generic_parameters,
                generic_arguments.len(),
            )?;
        }

        let argument_decision = self.decide_call_arguments(arguments, &function.parameters)?;
        let return_type = self.decide_call_return(&function, expected)?;
        match argument_decision.and(return_type) {
            Decision::Yes => {
                progress =
                    progress.merge(self.expect_call_arguments(arguments, &function.parameters)?);
                progress = progress.merge(self.expect_call_return(&function, expected)?);
                let target = target.into_resolution_target(instantiation.instance);

                Ok(CallableSelection::resolved(target, function, progress))
            }
            Decision::Undecidable => Ok(CallableSelection::Pending { progress }),
            Decision::No if !progress.is_unchanged() => Ok(CallableSelection::Pending { progress }),
            Decision::No => {
                if let Some((argument, parameter)) =
                    self.call_argument_type_failure(arguments, &function.parameters)?
                {
                    Ok(CallableSelection::rejected(CallFailure::ArgumentType {
                        argument,
                        parameter,
                    }))
                } else {
                    Ok(CallableSelection::rejected(CallFailure::NoMatch))
                }
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
        generic_arguments: &[GenericArgument],
    ) -> CompilerResult<CallInstantiation> {
        if function.generic_parameters.is_empty() {
            return Ok(CallInstantiation {
                function,
                instance,
                substitution: GenericSubstitution::empty(),
                generic_parameters: Vec::new(),
            });
        }
        let mut substitution = GenericSubstitution::empty();
        let mut arguments = SmallVec::with_capacity(function.generic_parameters.len());
        let generic_parameters = function.generic_parameters.clone();

        // use explicit arguments first, then infer the remaining call generics
        for (index, parameter) in function.generic_parameters.iter().enumerate() {
            let slot = self.generic_parameter_slot(*parameter)?;
            let argument = if let Some(argument) = generic_arguments.get(index) {
                self.select_argument_for_static_slot(
                    argument,
                    self.generic_parameter_is_static(*parameter)?,
                )
            } else {
                self.instantiation_argument(module, source, *parameter)?
            };
            let slot = slot.id();

            arguments.push(argument.clone());
            substitution.entries.push(GenericSubstitutionEntry {
                variable: *parameter,
                slot,
                argument,
            });
        }

        let mut function = function.substitute(module, &substitution, self)?;
        function.generic_parameters.clear();
        let instance =
            instance.or_else(|| owner.map(|symbol| GenericInstance { symbol, arguments }));

        Ok(CallInstantiation {
            function,
            instance,
            substitution,
            generic_parameters: generic_parameters.to_vec(),
        })
    }

    /// Solve omitted generic arguments from their defaults when inference has no input.
    fn solve_defaulted_generic_arguments(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        parameters: &[VariableId],
        explicit_count: usize,
    ) -> CompilerResult<()> {
        for parameter in parameters.iter().skip(explicit_count) {
            self.solve_defaulted_generic_argument(module, substitution, *parameter)?;
        }

        Ok(())
    }

    /// Solve one omitted generic argument from its default.
    fn solve_defaulted_generic_argument(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        parameter: VariableId,
    ) -> CompilerResult<()> {
        let variable = self.variable(parameter);
        let Some(VariableOutput::Generic(generic)) = &variable.output else {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter:?} is not a generic slot"),
            });
        };

        match generic {
            GenericParameter::Type {
                default: Some(default),
                ..
            }
            | GenericParameter::VariadicType {
                default: Some(default),
                ..
            } => {
                let Some(argument) = self.substitution_type_variable(substitution, parameter)
                else {
                    return Ok(());
                };
                if !self.lower_bounds(argument).is_empty()
                    || self.variable_solution(argument)?.is_some()
                {
                    return Ok(());
                }
                let default = self.substitute_type_variable(module, substitution, *default)?;
                let term = TypeTerm::Variable(default);

                self.solve_type_variable(argument, term)?;
            }
            GenericParameter::Static {
                default: Some(default),
                ..
            }
            | GenericParameter::VariadicStatic {
                default: Some(default),
                ..
            } => {
                let Some(argument) = self.substitution_static_variable(substitution, parameter)
                else {
                    return Ok(());
                };
                if !self.lower_bounds(argument).is_empty()
                    || self.variable_solution(argument)?.is_some()
                {
                    return Ok(());
                }
                let default = self.substitute_static_variable(module, substitution, *default)?;
                let term = StaticTerm::Variable(default);

                self.solve_static_variable(argument, term)?;
            }
            GenericParameter::Type { .. }
            | GenericParameter::VariadicType { .. }
            | GenericParameter::Static { .. }
            | GenericParameter::VariadicStatic { .. } => {}
        };

        Ok(())
    }

    /// Create one inferred generic argument variable.
    fn instantiation_argument(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: VariableId,
    ) -> CompilerResult<GenericArgument> {
        let variable = self.instantiation_variable(module, source, parameter)?;
        let kind = self.variable(variable).kind;
        let argument = match kind {
            VariableKind::Type => GenericArgument::Type(variable.into()),
            VariableKind::Static => GenericArgument::Static(variable.into()),
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
        let kind = self.variable(parameter).kind;

        if let Some(variable) = self.variables.generic_argument.get(&key).copied() {
            return Ok(variable);
        }
        let origin = ConstraintOrigin::Node(source);
        let variable = self.allocate_intermediate_variable(module, kind, origin);

        self.variables.generic_argument.insert(key, variable);

        Ok(variable)
    }

    /// Return the generic slot for one generic parameter.
    fn generic_parameter_slot(&self, parameter: VariableId) -> CompilerResult<GenericSlot> {
        let variable = self.variable(parameter);
        let Some(VariableOutput::Generic(generic)) = &variable.output else {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter:?} is not a generic slot"),
            });
        };

        Ok(generic.slot().clone())
    }

    /// Return whether one generic parameter expects a static argument.
    fn generic_parameter_is_static(&self, parameter: VariableId) -> CompilerResult<bool> {
        let variable = self.variable(parameter);
        let Some(VariableOutput::Generic(generic)) = &variable.output else {
            return Err(CompilerError::Internal {
                message: format!("generic parameter {parameter:?} is not a generic slot"),
            });
        };

        Ok(generic.is_static())
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
            TypeTerm::Function(function) => {
                CallableSignature::Present(self.terms.get(*function).clone())
            }
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
        arguments: &[GenericArgument],
    ) -> CompilerResult<CallableSignature> {
        if self.environment.language.item(symbol) != Some(dir::LanguageItem::Function) {
            return Ok(CallableSignature::Absent);
        }
        let Some(parameters) = self.generic_argument_type_variable(arguments, 0) else {
            return Ok(CallableSignature::Pending);
        };
        let Some(return_type) = self.generic_argument_type_variable(arguments, 1) else {
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
            generic_parameters: SmallVec::new(),
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
        members: &[ShapeMember],
    ) -> CompilerResult<CallableSignature> {
        for member in members {
            let member = member;
            let ShapeMember::CallSignature { ty } = member else {
                continue;
            };
            let Some(term) = self.type_operand_term(*ty)? else {
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
    ) -> CompilerResult<Option<SmallVec<[FunctionParameter; 4]>>> {
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(None);
        };
        let parameters = match term {
            TypeTerm::Tuple { elements, .. } => elements
                .iter()
                .map(|element| {
                    let element = element;
                    let parameter = FunctionParameter {
                        ty: element.ty,
                        is_optional: element.is_optional,
                        is_rest: element.is_rest,
                    };

                    parameter
                })
                .collect(),
            TypeTerm::Literal(TypeLiteralTerm::Void) => SmallVec::new(),
            _ => return Ok(None),
        };

        Ok(Some(parameters))
    }

    /// Decide whether arguments are assignable to parameters.
    fn decide_call_arguments(
        &self,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<Decision> {
        if !self.call_arity_accepts(arguments, parameters) {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // every argument must be assignable to the corresponding parameter
        for (argument, parameter) in arguments.iter().zip(parameters) {
            let parameter = parameter.ty;

            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                *argument,
                parameter,
            )?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the first solved argument type failure.
    fn call_argument_type_failure(
        &self,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<Option<(TypeOperand, TypeOperand)>> {
        if !self.call_arity_accepts(arguments, parameters) {
            return Ok(None);
        }

        for (argument, parameter) in arguments.iter().zip(parameters) {
            let parameter = parameter.ty;

            let decision =
                self.decide_type_relation(TypeRelation::Assignable, *argument, parameter)?;
            if decision == Decision::No {
                return Ok(Some((*argument, parameter)));
            }
        }

        Ok(None)
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

    /// Expect accepted call arguments to satisfy parameter types.
    fn expect_call_arguments(
        &mut self,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        for (argument, parameter) in arguments.iter().zip(parameters) {
            let parameter = parameter.ty;

            progress = progress.merge(self.solve_type_assignability(*argument, parameter)?);
        }

        Ok(progress)
    }

    /// Return whether runtime arguments fit one function parameter list.
    fn call_arity_accepts(
        &self,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
    ) -> bool {
        let required = parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = parameters.iter().any(|parameter| parameter.is_rest);

        arguments.len() >= required && (has_rest || arguments.len() <= parameters.len())
    }

    /// Expect an accepted call return to satisfy the result type.
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
            let void = self.terms.push(void);

            return self.solve_type_assignability(void, expected);
        };

        self.solve_type_assignability(return_type, expected)
    }
}
