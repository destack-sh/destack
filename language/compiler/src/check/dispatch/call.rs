use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallCallee, CallDecision, CallFailure, CallTargetResolution, CallTerm, CandidateResolution,
    CheckState, Condition, ConstructCandidates, ConstructDecision, ConstructFailure,
    ConstructTargetResolution, Decision, FunctionParameter, FunctionTerm, GenericArgument,
    GenericInstance, GenericInstanceKey, GenericParameterBinding, GenericParameterId,
    GenericSubstitution, GenericSubstitutionEntry, MemberDecision, MemberFailure, MemberLookup,
    MemberProjectionOrigin, Origin, Progress, ReceiverSubstitution, ShapeMember, StaticTerm,
    Substitution, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm, VariableId,
};
use smallvec::SmallVec;

use super::CandidateCardinality;

/// Callable candidate considered by dispatch selection.
pub(in crate::check) struct CallableCandidate {
    /// The module whose type context owns the candidate.
    pub(in crate::check) module: ModuleId,
    /// The resolved declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The callable type term to inspect.
    pub(in crate::check) ty: TypeTerm,
    /// The already resolved generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The final callable target if this candidate is selected.
    pub(in crate::check) target: CallableTarget,
}

/// Function signature after applying generic arguments.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionTermApplication {
    /// The instantiated function signature.
    pub(in crate::check) function: FunctionTerm,
    /// The resolved generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The substitution used for this instance.
    pub(in crate::check) substitution: GenericSubstitution,
    /// The original generic parameters.
    pub(in crate::check) generic_parameters: Vec<GenericParameterId>,
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

/// Transient callable dispatch while reducing call-like syntax.
pub(in crate::check) enum CallableDispatch {
    /// Dispatch is waiting for solver input.
    Pending {
        /// The variable changes made before dispatch became pending.
        progress: Progress,
    },
    /// Dispatch is invalid because a sub-selection already failed.
    Invalid {
        /// The variable changes made before dispatch became invalid.
        progress: Progress,
    },
    /// Call dispatch failed.
    CallRejected(CallFailure),
    /// Construct dispatch failed.
    ConstructRejected(ConstructFailure),
    /// One call candidate resolved.
    CallSelected {
        /// The resolved call target.
        target: CallTargetResolution,
        /// The resolved function signature.
        function: FunctionTerm,
        /// The variable changes made while resolving.
        progress: Progress,
    },
    /// One construct candidate resolved.
    ConstructSelected {
        /// The resolved construct target.
        target: ConstructTargetResolution,
        /// The resolved constructor signature.
        function: FunctionTerm,
        /// The variable changes made while resolving.
        progress: Progress,
    },
}

impl CallableDispatch {
    /// Return a pending dispatch with no side progress.
    pub(in crate::check) fn pending() -> Self {
        Self::Pending {
            progress: Progress::Unchanged,
        }
    }

    /// Return an invalid dispatch with no side progress.
    pub(in crate::check) fn invalid() -> Self {
        Self::Invalid {
            progress: Progress::Unchanged,
        }
    }

    /// Return a selected call dispatch with side progress.
    pub(in crate::check) fn call_selected(
        target: CallTargetResolution,
        function: FunctionTerm,
        progress: Progress,
    ) -> Self {
        Self::CallSelected {
            target,
            function,
            progress,
        }
    }

    /// Return a selected construct dispatch with side progress.
    pub(in crate::check) fn construct_selected(
        target: ConstructTargetResolution,
        function: FunctionTerm,
        progress: Progress,
    ) -> Self {
        Self::ConstructSelected {
            target,
            function,
            progress,
        }
    }

    /// Return a rejected call dispatch.
    pub(in crate::check) fn call_rejected(failure: CallFailure) -> Self {
        Self::CallRejected(failure)
    }

    /// Return a rejected construct dispatch.
    pub(in crate::check) fn construct_rejected(failure: ConstructFailure) -> Self {
        Self::ConstructRejected(failure)
    }

    /// Return progress made while dispatching.
    pub(in crate::check) fn progress(&self) -> Progress {
        match self {
            Self::Pending { progress }
            | Self::Invalid { progress }
            | Self::CallSelected { progress, .. }
            | Self::ConstructSelected { progress, .. } => progress.clone(),
            Self::CallRejected(_) | Self::ConstructRejected(_) => Progress::Unchanged,
        }
    }
}

/// Callable target selected before generic instantiation.
#[derive(Clone)]
pub(in crate::check) enum CallableTarget {
    /// Callable expression without a declaration symbol.
    Expression,
    /// Symbol-backed callable selected at compile time.
    Symbol {
        /// The resolved callable symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved receiver type for method calls.
        receiver: Option<TypeOperand>,
    },
    /// Symbol-backed callable variants selected from a union receiver.
    Union {
        /// The resolved callable candidates.
        candidates: Vec<CandidateResolution>,
        /// The resolved receiver type for method calls.
        receiver: Option<TypeOperand>,
    },
    /// Construct target exposed through call-like syntax.
    Construct(ConstructTargetResolution),
}

impl CallableTarget {
    /// Return the selected receiver type when this target has one.
    fn receiver(&self) -> Option<TypeOperand> {
        match self {
            Self::Symbol { receiver, .. } | Self::Union { receiver, .. } => *receiver,
            Self::Expression | Self::Construct(_) => None,
        }
    }

    /// Return this selected target as a callable dispatch.
    pub(in crate::check) fn into_dispatch(
        self,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        progress: Progress,
    ) -> CallableDispatch {
        match self {
            Self::Expression => {
                let target = CallTargetResolution::Expression { instance };

                CallableDispatch::call_selected(target, function, progress)
            }
            Self::Symbol { symbol, receiver } => {
                let target = CallTargetResolution::Symbol {
                    symbol,
                    instance,
                    receiver,
                };

                CallableDispatch::call_selected(target, function, progress)
            }
            Self::Union {
                candidates,
                receiver,
            } => {
                let target = CallTargetResolution::Union {
                    candidates,
                    receiver,
                };

                CallableDispatch::call_selected(target, function, progress)
            }
            Self::Construct(target) => {
                let target = target.with_application(instance);

                CallableDispatch::construct_selected(target, function, progress)
            }
        }
    }
}

impl CheckState<'_> {
    /// Select one runtime call target from callable candidates.
    pub(in crate::check) fn select_call_target(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        // return cached call decision
        if let Some(selection) = self.selected_call(call) {
            return Ok(selection);
        }

        // try member call
        if let Some(result) = self.select_member_call(origin, call, expected)? {
            Ok(result)
        }
        // try reference call
        else if let Some(result) = self.select_reference_call(origin, call, expected)? {
            Ok(result)
        } else {
            self.select_expression_call(origin, call, expected)
        }
    }

    /// Return the already chosen decision for one call.
    fn selected_call(&self, call: &CallTerm) -> Option<CallableDispatch> {
        // return cached construct decision
        if let Some(decision) = self.inference.construct(call.source) {
            let selection = match decision {
                ConstructDecision::Resolved(selection) => CallableDispatch::construct_selected(
                    selection.target,
                    selection.function,
                    Progress::Unchanged,
                ),
                ConstructDecision::Rejected(failure) => {
                    CallableDispatch::construct_rejected(failure)
                }
            };

            return Some(selection);
        }

        // return cached call decision
        let Some(decision) = self.inference.call(call.source) else {
            return None;
        };
        let selection = match decision {
            CallDecision::Resolved(selection) => CallableDispatch::call_selected(
                selection.target,
                selection.function,
                Progress::Unchanged,
            ),
            CallDecision::Rejected(failure) => CallableDispatch::call_rejected(failure),
        };

        Some(selection)
    }

    /// Select a call whose callee is an arbitrary callable expression.
    fn select_expression_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let CallCallee::Expression(callee) = call.callee else {
            unreachable!("non-expression call reached expression dispatch");
        };
        let Some(term) = self.reduce_type_operand(origin, callee)? else {
            return Ok(CallableDispatch::pending());
        };
        let module = call.source.module_id;
        let function = match self.call_signature(module, &term)? {
            CallableSignature::Pending => return Ok(CallableDispatch::pending()),
            CallableSignature::Absent => {
                return Ok(CallableDispatch::call_rejected(CallFailure::NotCallable));
            }
            CallableSignature::Present(function) => function,
        };

        self.select_call_signature(
            origin,
            module,
            call.source,
            None,
            None,
            function,
            &call.generic_arguments,
            &call.arguments,
            &call.argument_values,
            expected,
            CallableTarget::Expression,
            CandidateCardinality::One,
        )
    }

    /// Select a call whose callee is a symbol-backed reference.
    fn select_reference_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let CallCallee::Reference {
            value: callee,
            symbol,
        } = call.callee
        else {
            return Ok(None);
        };

        let module = call.source.module_id;
        let Some(term) = self.reduce_type_operand(origin, callee)? else {
            return Ok(Some(CallableDispatch::pending()));
        };
        let function = match self.call_signature(module, &term)? {
            CallableSignature::Pending => return Ok(Some(CallableDispatch::pending())),
            CallableSignature::Absent => {
                return match self.select_construct_call(origin, call, expected, &term)? {
                    Some(result) => Ok(Some(result)),
                    None => Ok(Some(CallableDispatch::call_rejected(
                        CallFailure::NotCallable,
                    ))),
                };
            }
            CallableSignature::Present(function) => function,
        };
        let result = self.select_call_signature(
            origin,
            module,
            call.source,
            Some(symbol),
            None,
            function,
            &call.generic_arguments,
            &call.arguments,
            &call.argument_values,
            expected,
            CallableTarget::Symbol {
                symbol,
                receiver: None,
            },
            CandidateCardinality::One,
        )?;

        Ok(Some(result))
    }

    /// Select a construct target exposed through call syntax.
    fn select_construct_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
        term: &TypeTerm,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let module = call.source.module_id;
        let candidates = match self.construct_candidates(module, term)? {
            ConstructCandidates::Pending => return Ok(Some(CallableDispatch::pending())),
            ConstructCandidates::Absent => return Ok(None),
            ConstructCandidates::Present(candidates) => candidates,
        };
        let result = self.select_construct_candidate(
            origin,
            module,
            call.source,
            candidates,
            &call.generic_arguments,
            &call.arguments,
            &call.argument_values,
            expected,
        )?;

        Ok(Some(result))
    }

    /// Select a call whose callee is a member projection.
    fn select_member_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let CallCallee::Member(member) = call.callee else {
            return Ok(None);
        };
        let member = self.inference.term(member).clone();
        let Some(receiver) = self.reduce_type_operand(origin, member.receiver)? else {
            return Ok(Some(CallableDispatch::pending()));
        };
        let module = call.source.module_id;
        let member_match = match &member.origin {
            MemberProjectionOrigin::Expression { .. } => {
                self.lookup_type_member(origin, module, &receiver, &member.key)?
            }
            MemberProjectionOrigin::Protocol { protocol } => {
                self.lookup_protocol_member(origin, module, &receiver, &member.key, &protocol)?
            }
        };
        let member_matches = match member_match {
            MemberLookup::Found(member_matches) => member_matches,
            MemberLookup::Pending => {
                return Ok(Some(CallableDispatch::pending()));
            }
            MemberLookup::Missing => {
                if let MemberProjectionOrigin::Expression { source } = member.origin {
                    self.select_member(
                        source,
                        MemberDecision::Rejected(MemberFailure::Missing { key: member.key }),
                    )?;
                }

                return Ok(Some(CallableDispatch::invalid()));
            }
        };
        let candidate_cardinality = CandidateCardinality::from_len(member_matches.len());
        let union_target = if candidate_cardinality.is_many() {
            let candidates = member_matches
                .iter()
                .map(|candidate| CandidateResolution {
                    symbol: candidate.symbol,
                    instance: candidate.instance.clone(),
                })
                .collect();

            Some(CallableTarget::Union {
                candidates,
                receiver: Some(member.receiver),
            })
        } else {
            None
        };
        let dispatch_candidates = member_matches
            .iter()
            .map(|member_match| {
                let target = if let Some(target) = &union_target {
                    target.clone()
                } else {
                    CallableTarget::Symbol {
                        symbol: member_match.symbol,
                        receiver: Some(member.receiver),
                    }
                };

                CallableCandidate {
                    module,
                    symbol: member_match.symbol,
                    ty: member_match.ty.clone(),
                    instance: member_match.instance.clone(),
                    target,
                }
            })
            .collect::<Vec<_>>();
        let result = self.select_callable_candidate(
            origin,
            call.source,
            &dispatch_candidates,
            &call.generic_arguments,
            &call.arguments,
            &call.argument_values,
            expected,
        )?;

        Ok(Some(result))
    }

    /// Select one callable signature for actual call arguments.
    pub(in crate::check) fn select_call_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
        target: CallableTarget,
        cardinality: CandidateCardinality,
    ) -> CompilerResult<CallableDispatch> {
        // probe one selected signature
        let probe = self.inference.begin_probe();
        let result = self.reduce_call_signature(
            origin,
            module,
            source,
            owner,
            instance,
            function,
            generic_arguments,
            arguments,
            argument_values,
            expected,
            target,
            cardinality,
        )?;

        // keep inference writes for selected or uniquely pending signatures
        match result {
            CallableDispatch::CallSelected { .. }
            | CallableDispatch::ConstructSelected { .. }
            | CallableDispatch::Invalid { .. }
            | CallableDispatch::Pending { .. } => self.inference.commit_probe(probe)?,
            CallableDispatch::CallRejected(_) | CallableDispatch::ConstructRejected(_) => {
                self.inference.drop_probe(probe);
            }
        }

        Ok(result)
    }

    /// Reduce one selected callable signature.
    fn reduce_call_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
        target: CallableTarget,
        candidate_cardinality: CandidateCardinality,
    ) -> CompilerResult<CallableDispatch> {
        // reject impossible explicit generic arity
        let is_generic = !function.generic_parameters.is_empty();
        if generic_arguments.len() > function.generic_parameters.len()
            || (!is_generic && !generic_arguments.is_empty())
        {
            return Ok(CallableDispatch::call_rejected(CallFailure::NoMatch));
        }

        // instantiate receiver and call generics
        let instantiation = self.instantiate_function_signature(
            module,
            source,
            owner,
            target.receiver(),
            instance,
            function,
            generic_arguments,
            argument_values,
        )?;
        let function = instantiation.function;
        let mut progress = Progress::Unchanged;

        // constrain call inputs against the selected signature
        progress =
            progress.merge(self.expect_call_arguments(origin, arguments, &function.parameters)?);
        progress = progress.merge(self.expect_call_return(origin, &function, expected)?);

        // constrain inferred call generics
        if is_generic {
            progress = progress.merge(self.solve_omitted_generic_defaults(
                module,
                (&instantiation.substitution).into(),
                &instantiation.generic_parameters,
                generic_arguments.len(),
            )?);
            progress = progress.merge(self.expect_call_generic_constraints(
                origin,
                module,
                (&instantiation.substitution).into(),
                &instantiation.generic_parameters,
            )?);
        }

        // reduce final argument and result decisions
        let argument_decision = self.decide_call_arguments(arguments, &function.parameters)?;
        let return_decision = self.decide_call_return(&function, expected)?;
        let generic_constraints = self.reduce_call_generic_constraints(
            module,
            (&instantiation.substitution).into(),
            &instantiation.generic_parameters,
        )?;
        let input_decision = argument_decision.and(generic_constraints);
        match input_decision {
            Decision::Yes => match return_decision {
                Decision::Yes | Decision::Undecidable => {
                    Ok(target.into_dispatch(instantiation.instance, function, progress))
                }
                Decision::No if !progress.is_unchanged() => {
                    Ok(CallableDispatch::Pending { progress })
                }
                Decision::No => self.call_signature_rejected(arguments, &function.parameters),
            },
            Decision::Undecidable if candidate_cardinality.keeps_pending_probe() => {
                Ok(target.into_dispatch(instantiation.instance, function, progress))
            }
            Decision::Undecidable => Ok(CallableDispatch::Pending { progress }),
            Decision::No if !progress.is_unchanged() => Ok(CallableDispatch::Pending { progress }),
            Decision::No => self.call_signature_rejected(arguments, &function.parameters),
        }
    }

    /// Return the precise rejection for a failed call signature.
    fn call_signature_rejected(
        &mut self,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<CallableDispatch> {
        if let Some((argument, parameter)) =
            self.call_argument_type_failure(arguments, parameters)?
        {
            Ok(CallableDispatch::call_rejected(CallFailure::ArgumentType {
                argument,
                parameter,
            }))
        } else {
            Ok(CallableDispatch::call_rejected(CallFailure::NoMatch))
        }
    }

    /// Select the first applicable callable candidate.
    pub(in crate::check) fn select_callable_candidate(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        candidates: &[CallableCandidate],
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let mut callable_signature_count = 0;
        let mut saw_pending = false;
        let candidate_cardinality = CandidateCardinality::from_len(candidates.len());

        // choose the first compatible declaration order candidate
        for candidate in candidates {
            let probe = self.inference.begin_probe();
            let reduction = self.reduce_type_term(origin, &candidate.ty)?;
            let Some(term) = reduction.value else {
                if candidate_cardinality.keeps_pending_probe() {
                    self.inference.commit_probe(probe)?;

                    return Ok(CallableDispatch::Pending {
                        progress: reduction.progress,
                    });
                }

                saw_pending = true;
                self.inference.drop_probe(probe);

                continue;
            };
            let function = match self.call_signature(candidate.module, &term)? {
                CallableSignature::Pending => {
                    self.inference.drop_probe(probe);
                    saw_pending = true;
                    continue;
                }
                CallableSignature::Absent => {
                    self.inference.drop_probe(probe);

                    continue;
                }
                CallableSignature::Present(function) => function,
            };
            callable_signature_count += 1;
            let result = self.select_call_signature(
                origin,
                candidate.module,
                source,
                Some(candidate.symbol),
                candidate.instance.clone(),
                function,
                generic_arguments,
                arguments,
                argument_values,
                expected,
                candidate.target.clone(),
                candidate_cardinality,
            )?;

            match result {
                CallableDispatch::CallSelected { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                CallableDispatch::ConstructSelected { .. } => {
                    unreachable!(
                        "internal invariant: callable candidate dispatch produced a construct selection"
                    );
                }
                CallableDispatch::Pending { .. } => {
                    if candidate_cardinality.keeps_pending_probe() {
                        self.inference.commit_probe(probe)?;

                        return Ok(result);
                    }
                    self.inference.drop_probe(probe);

                    return Ok(CallableDispatch::pending());
                }
                CallableDispatch::CallRejected(CallFailure::ArgumentType { .. }) => {
                    self.inference.drop_probe(probe);
                }
                CallableDispatch::CallRejected(CallFailure::NoMatch) => {
                    self.inference.drop_probe(probe);
                }
                CallableDispatch::CallRejected(CallFailure::NotCallable) => {
                    self.inference.drop_probe(probe);
                }
                CallableDispatch::ConstructRejected(_) => {
                    self.inference.drop_probe(probe);
                }
                CallableDispatch::Invalid { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
            }
        }

        // wait for unresolved candidate type input
        if saw_pending {
            return Ok(CallableDispatch::pending());
        }

        // distinguish rejected callable overloads from non callable values
        if callable_signature_count > 0 {
            Ok(CallableDispatch::call_rejected(CallFailure::NoMatch))
        } else {
            Ok(CallableDispatch::call_rejected(CallFailure::NotCallable))
        }
    }

    /// Return the callable signature represented by one type term.
    pub(in crate::check) fn call_signature(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
    ) -> CompilerResult<CallableSignature> {
        let callable = match term {
            TypeTerm::Function(function) => {
                CallableSignature::Present(self.inference.term(*function).clone())
            }
            TypeTerm::Type(ty) => match self.r#type(*ty) {
                dir::Type::Function(function) => {
                    CallableSignature::Present(self.function_type_term(module, function.clone())?)
                }
                _ => CallableSignature::Absent,
            },
            TypeTerm::Reference {
                origin: _,
                symbol: _,
                arguments: _,
            } => CallableSignature::Absent,
            TypeTerm::Shape(shape) => {
                let members = self.inference.term(*shape).members.clone();

                self.shape_call_signature(module, &members)?
            }
            _ => CallableSignature::Absent,
        };

        Ok(callable)
    }

    /// Return the call signature for a shape term.
    fn shape_call_signature(
        &mut self,
        module: ModuleId,
        members: &[ShapeMember],
    ) -> CompilerResult<CallableSignature> {
        for member in members {
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

    /// Instantiate function generics as call-local inference variables.
    pub(in crate::check) fn instantiate_function_signature(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        receiver: Option<TypeOperand>,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
    ) -> CompilerResult<FunctionTermApplication> {
        if function.generic_parameters.is_empty() {
            let function = if let Some(receiver) = receiver {
                let substitution = ReceiverSubstitution::new(receiver);

                function.substitute_receiver(module, substitution, self)?
            } else {
                function
            };

            return Ok(FunctionTermApplication {
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
            let generic = self.inference.generic_parameter(*parameter);
            let owner = generic.identity().owner;
            let instance_key = GenericInstanceKey { source, owner };
            let argument = if let Some(argument) = generic_arguments.get(index) {
                argument.select_for_static_parameter(generic.is_static())
            } else if let Some(argument) = instance
                .as_ref()
                .filter(|instance| instance.owner == owner)
                .and_then(|instance| instance.arguments.get(index))
            {
                argument.select_for_static_parameter(generic.is_static())
            } else if let Some(argument) = self
                .inference
                .generic_instance(instance_key)
                .and_then(|instance| instance.arguments.get(index))
            {
                argument.clone()
            } else if let Some(argument) = self.static_parameter_argument(
                module,
                *parameter,
                &function.parameters,
                argument_values,
            )? {
                argument
            } else {
                self.instantiation_argument(module, source, *parameter)?
            };

            arguments.push(argument.clone());
            substitution.entries.push(GenericSubstitutionEntry {
                parameter: *parameter,
                argument,
            });
        }

        let mut function = function.substitute(module, &substitution, self)?;
        if let Some(receiver) = receiver {
            let substitution = ReceiverSubstitution::new(receiver);

            function = function.substitute_receiver(module, substitution, self)?;
        }
        function.generic_parameters.clear();
        let instance = instance
            .or_else(|| owner.map(|owner| self.insert_generic_instance(source, owner, arguments)));

        let instantiation = FunctionTermApplication {
            function,
            instance,
            substitution,
            generic_parameters: generic_parameters.to_vec(),
        };

        Ok(instantiation)
    }

    /// Create one omitted call instantiation argument.
    fn instantiation_argument(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
    ) -> CompilerResult<GenericArgument> {
        let generic = self.inference.generic_parameter(parameter);
        let is_static = generic.is_static();
        let variable = self.instantiation_variable(module, source, parameter)?;
        let argument = if is_static {
            GenericArgument::Static(variable.into())
        } else {
            GenericArgument::Type(variable.into())
        };

        Ok(argument)
    }

    /// Return the runtime argument supplied to one static parameter parameter.
    fn static_parameter_argument(
        &mut self,
        module: ModuleId,
        parameter: GenericParameterId,
        parameters: &[FunctionParameter],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
    ) -> CompilerResult<Option<GenericArgument>> {
        let Some(index) = parameters
            .iter()
            .position(|candidate| candidate.static_parameter == Some(parameter))
        else {
            return Ok(None);
        };
        let Some(value) = argument_values.get(index).cloned() else {
            return Ok(None);
        };
        let origin = Origin::Node(value.clone().into_any());
        let variable = self.create_static_variable(module, origin);
        let term = StaticTerm::Expression(value);
        let term = self.inference.push_term(term);

        self.equate_static(origin, variable, term, Condition::Always);

        Ok(Some(GenericArgument::Static(variable.into())))
    }

    /// Return one stable omitted call instantiation argument variable.
    fn instantiation_variable(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: GenericParameterId,
    ) -> CompilerResult<VariableId> {
        let generic = self.inference.generic_parameter(parameter);
        let origin = Origin::Node(source);

        let variable = if generic.is_static() {
            self.create_static_variable(module, origin)
        } else {
            self.create_type_variable(module, origin)
        };

        Ok(variable)
    }

    /// Expect call instantiation arguments to satisfy declared constraints.
    fn expect_call_generic_constraints(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameters: &[GenericParameterId],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // constrain each substituted generic argument by its declared parameter
        for parameter in parameters {
            let generic = self.inference.generic_parameter(*parameter).clone();

            progress = progress.merge(self.expect_call_generic_constraint(
                origin,
                module,
                substitution,
                *parameter,
                &generic,
            )?);
        }

        Ok(progress)
    }

    /// Expect one call instantiation argument to satisfy its declared constraint.
    fn expect_call_generic_constraint(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameter: GenericParameterId,
        generic: &GenericParameterBinding,
    ) -> CompilerResult<Progress> {
        match generic {
            GenericParameterBinding::Type {
                constraint: Some(constraint),
                ..
            }
            | GenericParameterBinding::VariadicType {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_type_operand(substitution, parameter) else {
                    return Ok(Progress::Unchanged);
                };
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.relate_contextual_type_assignability(origin, argument, constraint)
            }
            GenericParameterBinding::Static {
                constraint: Some(constraint),
                ..
            }
            | GenericParameterBinding::VariadicStatic {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_static_operand(substitution, parameter)
                else {
                    return Ok(Progress::Unchanged);
                };
                let source = self
                    .inference
                    .push_term(TypeTerm::StaticValue { value: argument });
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.relate_contextual_type_assignability(origin, source, constraint)
            }
            GenericParameterBinding::Type {
                constraint: None, ..
            }
            | GenericParameterBinding::VariadicType {
                constraint: None, ..
            }
            | GenericParameterBinding::Static {
                constraint: None, ..
            }
            | GenericParameterBinding::VariadicStatic {
                constraint: None, ..
            } => Ok(Progress::Unchanged),
        }
    }

    /// Decide whether call generic arguments satisfy declared constraints.
    fn reduce_call_generic_constraints(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameters: &[GenericParameterId],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // combine each generic parameter constraint
        for parameter in parameters {
            let generic = self.inference.generic_parameter(*parameter).clone();

            decision = decision.and(self.reduce_call_generic_constraint(
                module,
                substitution,
                *parameter,
                &generic,
            )?);
        }

        Ok(decision)
    }

    /// Decide whether one call generic argument satisfies its declared constraint.
    fn reduce_call_generic_constraint(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameter: GenericParameterId,
        generic: &GenericParameterBinding,
    ) -> CompilerResult<Decision> {
        match generic {
            GenericParameterBinding::Type {
                constraint: Some(constraint),
                ..
            }
            | GenericParameterBinding::VariadicType {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_type_operand(substitution, parameter) else {
                    return Ok(Decision::Undecidable);
                };
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.decide_type_relation(TypeRelation::Assignable, argument, constraint)
            }
            GenericParameterBinding::Static {
                constraint: Some(constraint),
                ..
            }
            | GenericParameterBinding::VariadicStatic {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_static_operand(substitution, parameter)
                else {
                    return Ok(Decision::Undecidable);
                };
                let source = self
                    .inference
                    .push_term(TypeTerm::StaticValue { value: argument });
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.decide_type_relation(TypeRelation::Assignable, source, constraint)
            }
            GenericParameterBinding::Type {
                constraint: None, ..
            }
            | GenericParameterBinding::VariadicType {
                constraint: None, ..
            }
            | GenericParameterBinding::Static {
                constraint: None, ..
            }
            | GenericParameterBinding::VariadicStatic {
                constraint: None, ..
            } => Ok(Decision::Yes),
        }
    }

    /// Solve omitted generic arguments from their defaults when inference has no input.
    fn solve_omitted_generic_defaults(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameters: &[GenericParameterId],
        explicit_count: usize,
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        for parameter in parameters.iter().skip(explicit_count) {
            progress = progress.merge(self.solve_omitted_generic_default(
                module,
                substitution,
                *parameter,
            )?);
        }

        Ok(progress)
    }

    /// Solve one omitted generic argument from its default.
    fn solve_omitted_generic_default(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameter: GenericParameterId,
    ) -> CompilerResult<Progress> {
        let generic = self.inference.generic_parameter(parameter).clone();

        let progress = match generic {
            GenericParameterBinding::Type {
                default: Some(default),
                ..
            }
            | GenericParameterBinding::VariadicType {
                default: Some(default),
                ..
            } => {
                let Some(argument) = self
                    .substitution_type_operand(substitution, parameter)
                    .and_then(|operand| operand.variable())
                else {
                    return Ok(Progress::Unchanged);
                };
                let default = self.substitute_type_operand(module, substitution, default)?;

                self.solve_default_type_variable(argument, default)?
            }
            GenericParameterBinding::Static {
                default: Some(default),
                ..
            }
            | GenericParameterBinding::VariadicStatic {
                default: Some(default),
                ..
            } => {
                let Some(argument) = self
                    .substitution_static_operand(substitution, parameter)
                    .and_then(|operand| operand.variable())
                else {
                    return Ok(Progress::Unchanged);
                };
                let default = self.substitute_static_operand(module, substitution, default)?;

                self.solve_default_static_variable(argument, default)?
            }
            GenericParameterBinding::Type { .. }
            | GenericParameterBinding::VariadicType { .. }
            | GenericParameterBinding::Static { .. }
            | GenericParameterBinding::VariadicStatic { .. } => Progress::Unchanged,
        };

        Ok(progress)
    }

    /// Decide whether arguments are assignable to parameters.
    fn decide_call_arguments(
        &mut self,
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
        &mut self,
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
        &mut self,
        function: &FunctionTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Decision> {
        let Some(expected) = expected else {
            return Ok(Decision::Yes);
        };
        let Some(return_type) = function.return_type else {
            let Some(expected) = self.type_solution(expected)? else {
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
        origin: Origin,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        for (argument, parameter) in arguments.iter().zip(parameters) {
            let parameter = parameter.ty;

            progress = progress
                .merge(self.relate_contextual_type_assignability(origin, *argument, parameter)?);
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
        origin: Origin,
        function: &FunctionTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Progress> {
        let Some(expected) = expected else {
            return Ok(Progress::Unchanged);
        };
        let Some(return_type) = function.return_type else {
            let void = TypeTerm::Literal(TypeLiteralTerm::Void);
            let void = self.inference.push_term(void);

            return self.relate_contextual_type_assignability(origin, void, expected);
        };

        self.relate_contextual_type_assignability(origin, return_type, expected)
    }
}
