use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallCallee, CallDecision, CallFailure, CallTargetResolution, CallTerm, CandidateResolution,
    CheckState, Condition, ConstructCandidates, ConstructDecision, ConstructFailure,
    ConstructTargetResolution, Decision, FunctionParameter, FunctionTerm, GenericApplication,
    GenericArgument, GenericSlot, GenericSlotHeader, GenericSubstitution, GenericSubstitutionEntry,
    MemberCallSource, Origin, Progress, ReceiverSubstitution, ShapeMember, StaticTerm,
    Substitution, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm, VariableId, VariableKind,
};
use smallvec::SmallVec;

use super::CandidateSet;

/// Callable candidate considered by dispatch selection.
pub(in crate::check) struct CallableCandidate {
    /// The module whose type context owns the candidate.
    pub(in crate::check) module: ModuleId,
    /// The resolved declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The callable type term to inspect.
    pub(in crate::check) ty: TypeTerm,
    /// The already resolved generic application.
    pub(in crate::check) application: Option<GenericApplication>,
    /// The final callable target if this candidate is selected.
    pub(in crate::check) target: CallableTarget,
}

/// Function signature after applying generic arguments.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct FunctionTermApplication {
    /// The instantiated function signature.
    pub(in crate::check) function: FunctionTerm,
    /// The resolved generic application.
    pub(in crate::check) application: Option<GenericApplication>,
    /// The substitution used for this application.
    pub(in crate::check) substitution: GenericSubstitution,
    /// The original generic parameters.
    pub(in crate::check) generic_parameters: Vec<VariableId>,
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
        application: Option<GenericApplication>,
        function: FunctionTerm,
        progress: Progress,
    ) -> CallableDispatch {
        match self {
            Self::Expression => {
                let target = CallTargetResolution::Expression { application };

                CallableDispatch::call_selected(target, function, progress)
            }
            Self::Symbol { symbol, receiver } => {
                let target = CallTargetResolution::Symbol {
                    symbol,
                    application,
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
                let target = target.with_application(application);

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
        if let Some(selection) = self.selected_call(call)? {
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
    fn selected_call(&self, call: &CallTerm) -> CompilerResult<Option<CallableDispatch>> {
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

            return Ok(Some(selection));
        }

        // return cached call decision
        let Some(decision) = self.inference.call(call.source) else {
            return Ok(None);
        };
        let selection = match decision {
            CallDecision::Resolved(selection) => CallableDispatch::call_selected(
                selection.target,
                selection.function,
                Progress::Unchanged,
            ),
            CallDecision::Rejected(failure) => CallableDispatch::call_rejected(failure),
        };

        Ok(Some(selection))
    }

    /// Select a call whose callee is an arbitrary callable expression.
    fn select_expression_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let CallCallee::Expression(callee) = call.callee else {
            panic!("non-expression call reached expression dispatch");
        };
        let Some(term) = self.type_operand_term(callee)? else {
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
            CandidateSet::Single,
        )
    }

    /// Select a call whose callee is a symbol-backed reference.
    fn select_reference_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let CallCallee::Reference { value: _, symbol } = call.callee else {
            return Ok(None);
        };

        let module = call.source.module_id;
        if self.symbol_kind(module, symbol) == Some(dir::SymbolKind::Newtype) {
            let callee = self.require_symbol_type(module, symbol);
            let term = TypeTerm::Reference {
                origin: Origin::Node(call.source),
                symbol,
                arguments: Vec::new().into(),
            };

            return self.select_construct_call(origin, call, expected, callee, &term);
        }

        let ty = self.require_symbol_type(module, symbol);
        let Some(term) = self.type_operand_term(ty)? else {
            return Ok(Some(CallableDispatch::pending()));
        };
        let function = match self.call_signature(module, &term)? {
            CallableSignature::Pending => return Ok(Some(CallableDispatch::pending())),
            CallableSignature::Absent => return Ok(None),
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
            CandidateSet::Single,
        )?;

        Ok(Some(result))
    }

    /// Select a construct target exposed through call syntax.
    fn select_construct_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
        callee: TypeOperand,
        term: &TypeTerm,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let module = call.source.module_id;
        let candidates = match self.construct_candidates(module, callee, term)? {
            ConstructCandidates::Pending => return Ok(Some(CallableDispatch::pending())),
            ConstructCandidates::Absent => return Ok(None),
            ConstructCandidates::Present(candidates) => candidates,
        };
        let candidate_set = CandidateSet::from_len(candidates.len());

        // choose the first compatible declaration order construct candidate
        for candidate in candidates {
            let probe = self.begin_inference_probe();
            let owner = Some(candidate.target.symbol());
            let application = candidate.target.application().cloned();
            let target = CallableTarget::Construct(candidate.target);
            let result = self.select_call_signature(
                origin,
                module,
                call.source,
                owner,
                application,
                candidate.function,
                &call.generic_arguments,
                &call.arguments,
                &call.argument_values,
                expected,
                target,
                candidate_set,
            )?;
            match result {
                CallableDispatch::ConstructSelected { .. } => {
                    self.commit_inference_probe(probe)?;

                    return Ok(Some(result));
                }
                CallableDispatch::CallSelected { .. } => {
                    panic!("construct call dispatch produced a call selection");
                }
                CallableDispatch::Pending { .. } => {
                    if candidate_set.keeps_pending_probe() {
                        self.commit_inference_probe(probe)?;

                        return Ok(Some(result));
                    } else {
                        self.drop_inference_probe(probe);

                        return Ok(Some(CallableDispatch::pending()));
                    }
                }
                CallableDispatch::CallRejected(_) | CallableDispatch::ConstructRejected(_) => {
                    self.drop_inference_probe(probe);
                }
            }
        }

        Ok(Some(CallableDispatch::construct_rejected(
            ConstructFailure::NoMatch,
        )))
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
        let member = self.term(member).clone();
        let Some(receiver) = self.type_operand_term(member.receiver)? else {
            return Ok(Some(CallableDispatch::pending()));
        };
        let module = call.source.module_id;
        let member_match = match &member.source {
            MemberCallSource::Expression { .. } => {
                self.member_type_candidates(origin, module, &receiver, &member.key)?
            }
            MemberCallSource::Protocol { protocol } => self.member_type_candidates_for_protocol(
                origin,
                module,
                &receiver,
                &member.key,
                protocol,
            )?,
        };
        let Some(member_matches) = member_match else {
            return Ok(Some(CallableDispatch::pending()));
        };
        let candidate_set = CandidateSet::from_len(member_matches.len());
        let union_target = if candidate_set == CandidateSet::Overload {
            let candidates = member_matches
                .iter()
                .map(|candidate| CandidateResolution {
                    symbol: candidate.symbol,
                    application: candidate.application.clone(),
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
                    application: member_match.application.clone(),
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
        application: Option<GenericApplication>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
        target: CallableTarget,
        candidate_set: CandidateSet,
    ) -> CompilerResult<CallableDispatch> {
        // probe one selected signature
        let probe = self.begin_inference_probe();
        let result = self.reduce_call_signature(
            origin,
            module,
            source,
            owner,
            application,
            function,
            generic_arguments,
            arguments,
            argument_values,
            expected,
            target,
            candidate_set,
        )?;

        // keep inference writes for selected or uniquely pending signatures
        match result {
            CallableDispatch::CallSelected { .. }
            | CallableDispatch::ConstructSelected { .. }
            | CallableDispatch::Pending { .. } => self.commit_inference_probe(probe)?,
            CallableDispatch::CallRejected(_) | CallableDispatch::ConstructRejected(_) => {
                self.drop_inference_probe(probe);
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
        application: Option<GenericApplication>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
        target: CallableTarget,
        candidate_set: CandidateSet,
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
            application,
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
                    Ok(target.into_dispatch(instantiation.application, function, progress))
                }
                Decision::No if !progress.is_unchanged() => {
                    Ok(CallableDispatch::Pending { progress })
                }
                Decision::No => self.call_signature_rejected(arguments, &function.parameters),
            },
            Decision::Undecidable if candidate_set.keeps_pending_probe() => {
                Ok(target.into_dispatch(instantiation.application, function, progress))
            }
            Decision::Undecidable => Ok(CallableDispatch::Pending { progress }),
            Decision::No if !progress.is_unchanged() => Ok(CallableDispatch::Pending { progress }),
            Decision::No => self.call_signature_rejected(arguments, &function.parameters),
        }
    }

    /// Return the precise rejection for a failed call signature.
    fn call_signature_rejected(
        &self,
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
        let mut callable_count = 0;
        let mut saw_pending = false;
        let mut argument_failure = None;
        let candidate_set = CandidateSet::from_len(candidates.len());

        // choose the first compatible declaration order candidate
        for candidate in candidates {
            let probe = self.begin_inference_probe();
            let reduction = self.reduce_type_term(origin, &candidate.ty)?;
            let Some(term) = reduction.value else {
                if candidate_set.keeps_pending_probe() {
                    self.commit_inference_probe(probe)?;

                    return Ok(CallableDispatch::Pending {
                        progress: reduction.progress,
                    });
                }

                saw_pending = true;
                self.drop_inference_probe(probe);

                continue;
            };
            let function = match self.call_signature(candidate.module, &term)? {
                CallableSignature::Pending => {
                    self.drop_inference_probe(probe);
                    saw_pending = true;
                    continue;
                }
                CallableSignature::Absent => {
                    self.drop_inference_probe(probe);

                    continue;
                }
                CallableSignature::Present(function) => function,
            };
            callable_count += 1;
            let result = self.select_call_signature(
                origin,
                candidate.module,
                source,
                Some(candidate.symbol),
                candidate.application.clone(),
                function,
                generic_arguments,
                arguments,
                argument_values,
                expected,
                candidate.target.clone(),
                candidate_set,
            )?;

            match result {
                CallableDispatch::CallSelected { .. } => {
                    self.commit_inference_probe(probe)?;

                    return Ok(result);
                }
                CallableDispatch::ConstructSelected { .. } => {
                    panic!("callable candidate dispatch produced a construct selection");
                }
                CallableDispatch::Pending { .. } => {
                    if candidate_set.keeps_pending_probe() {
                        self.commit_inference_probe(probe)?;

                        return Ok(result);
                    }
                    self.drop_inference_probe(probe);

                    return Ok(CallableDispatch::pending());
                }
                CallableDispatch::CallRejected(CallFailure::ArgumentType {
                    argument,
                    parameter,
                }) => {
                    self.drop_inference_probe(probe);
                    argument_failure.get_or_insert((argument, parameter));
                }
                CallableDispatch::CallRejected(CallFailure::NoMatch) => {
                    self.drop_inference_probe(probe);
                }
                CallableDispatch::CallRejected(CallFailure::NotCallable) => {
                    self.drop_inference_probe(probe);
                }
                CallableDispatch::ConstructRejected(_) => {
                    self.drop_inference_probe(probe);
                }
            }
        }

        // wait for unresolved candidate type input
        if saw_pending {
            return Ok(CallableDispatch::pending());
        }

        // preserve the precise single candidate argument error
        if callable_count == 1
            && let Some((argument, parameter)) = argument_failure
        {
            return Ok(CallableDispatch::call_rejected(CallFailure::ArgumentType {
                argument,
                parameter,
            }));
        }

        // distinguish rejected callable overloads from non callable values
        if callable_count > 0 {
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
                CallableSignature::Present(self.term(*function).clone())
            }
            TypeTerm::Type(ty) => self.committed_call_signature(module, *ty)?,
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.named_call_signature(*symbol, arguments)?,
            TypeTerm::Shape(shape) => {
                let members = self.term(*shape).members.clone();

                self.shape_call_signature(module, &members)?
            }
            _ => CallableSignature::Absent,
        };

        Ok(callable)
    }

    /// Return a callable signature from one committed DIR type.
    fn committed_call_signature(
        &mut self,
        module: ModuleId,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<CallableSignature> {
        let function = match self.global_type_value(ty) {
            dir::Type::Function(function) => function.clone(),
            _ => return Ok(CallableSignature::Absent),
        };

        Ok(CallableSignature::Present(
            self.function_type_term(module, function)?,
        ))
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
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters,
            return_type: Some(return_type.into()),
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
    ) -> CompilerResult<Option<SmallVec<[FunctionParameter; 2]>>> {
        let Some(term) = self.type_solution(variable)? else {
            return Ok(None);
        };
        let parameters = match term {
            TypeTerm::Tuple { elements, .. } => elements
                .iter()
                .map(|element| FunctionParameter {
                    ty: element.ty,
                    static_slot: None,
                    is_optional: element.is_optional,
                    is_rest: element.is_rest,
                })
                .collect(),
            TypeTerm::Literal(TypeLiteralTerm::Void) => SmallVec::new(),
            _ => return Ok(None),
        };

        Ok(Some(parameters))
    }

    /// Instantiate function generics as call-local inference variables.
    pub(in crate::check) fn instantiate_function_signature(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        receiver: Option<TypeOperand>,
        application: Option<GenericApplication>,
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
                application,
                substitution: GenericSubstitution::empty(),
                generic_parameters: Vec::new(),
            });
        }
        let mut substitution = GenericSubstitution::empty();
        let mut arguments = SmallVec::with_capacity(function.generic_parameters.len());
        let generic_parameters = function.generic_parameters.clone();
        let application_owner = function.generic_parameters.first().map(|parameter| {
            let slot = self.generic_parameter_slot(*parameter);

            slot.owner
        });

        // use explicit arguments first, then infer the remaining call generics
        for (index, parameter) in function.generic_parameters.iter().enumerate() {
            let slot = self.generic_parameter_slot(*parameter);
            if let Some(owner) = application_owner
                && slot.owner != owner
            {
                panic!(
                    "function signature {source:?} mixes generic owners {owner:?} and {:?}",
                    slot.owner
                );
            }
            let argument = if let Some(argument) = generic_arguments.get(index) {
                self.select_argument_for_static_slot(
                    argument,
                    self.generic_parameter_is_static(*parameter),
                )
            } else if let Some(argument) = application
                .as_ref()
                .filter(|application| application.owner == slot.owner)
                .and_then(|application| application.arguments.get(slot.index.0 as usize))
            {
                self.select_argument_for_static_slot(
                    argument,
                    self.generic_parameter_is_static(*parameter),
                )
            } else if let Some(argument) =
                self.generic_application_argument(source, slot.owner, slot.index)
            {
                argument
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
            let slot = slot.id();

            arguments.push(argument.clone());
            substitution.entries.push(GenericSubstitutionEntry {
                variable: *parameter,
                slot,
                argument,
            });
        }

        let mut function = function.substitute(module, &substitution, self)?;
        if let Some(receiver) = receiver {
            let substitution = ReceiverSubstitution::new(receiver);

            function = function.substitute_receiver(module, substitution, self)?;
        }
        function.generic_parameters.clear();
        let owner = owner.or(application_owner);
        let application = application.or_else(|| {
            owner.map(|owner| self.attach_generic_application(source, owner, arguments))
        });

        let instantiation = FunctionTermApplication {
            function,
            application,
            substitution,
            generic_parameters: generic_parameters.to_vec(),
        };

        Ok(instantiation)
    }

    /// Return the generic slot for one generic parameter.
    pub(in crate::check) fn generic_parameter_slot(
        &self,
        parameter: VariableId,
    ) -> GenericSlotHeader {
        let Some(generic) = self.generic_slot(parameter) else {
            panic!("generic parameter {parameter:?} is not a generic slot");
        };

        generic.slot().clone()
    }

    /// Return whether one generic parameter expects a static argument.
    fn generic_parameter_is_static(&self, parameter: VariableId) -> bool {
        let Some(generic) = self.generic_slot(parameter) else {
            panic!("generic parameter {parameter:?} is not a generic slot");
        };

        generic.is_static()
    }

    /// Create one omitted call instantiation argument.
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

    /// Return the runtime argument supplied to one static parameter slot.
    fn static_parameter_argument(
        &mut self,
        module: ModuleId,
        parameter: VariableId,
        parameters: &[FunctionParameter],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
    ) -> CompilerResult<Option<GenericArgument>> {
        let Some(index) = parameters
            .iter()
            .position(|candidate| candidate.static_slot.as_deref() == Some(&parameter))
        else {
            return Ok(None);
        };
        let Some(value) = argument_values.get(index).cloned() else {
            return Ok(None);
        };
        let origin = Origin::Node(value.clone().into_any());
        let variable = self.allocate_variable(module, VariableKind::Static, origin);
        let term = StaticTerm::Expression(value);
        let term = self.push_term(term);

        self.equate_static(origin, variable, term, Condition::Always);

        Ok(Some(GenericArgument::Static(variable.into())))
    }

    /// Return one stable omitted call instantiation argument variable.
    fn instantiation_variable(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: VariableId,
    ) -> CompilerResult<VariableId> {
        let kind = self.variable(parameter).kind;
        let origin = Origin::Node(source);

        Ok(self.allocate_variable(module, kind, origin))
    }

    /// Expect call instantiation arguments to satisfy declared constraints.
    fn expect_call_generic_constraints(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameters: &[VariableId],
    ) -> CompilerResult<Progress> {
        let mut progress = Progress::Unchanged;

        // constrain each substituted generic argument by its declared slot
        for parameter in parameters {
            let Some(generic) = self.generic_slot(*parameter).cloned() else {
                panic!("generic parameter {parameter:?} is not a generic slot");
            };

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
        parameter: VariableId,
        generic: &GenericSlot,
    ) -> CompilerResult<Progress> {
        match generic {
            GenericSlot::Type {
                constraint: Some(constraint),
                ..
            }
            | GenericSlot::VariadicType {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_type_operand(substitution, parameter) else {
                    return Ok(Progress::Unchanged);
                };
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.relate_contextual_type_assignability(origin, argument, constraint)
            }
            GenericSlot::Static {
                constraint: Some(constraint),
                ..
            }
            | GenericSlot::VariadicStatic {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_static_operand(substitution, parameter)
                else {
                    return Ok(Progress::Unchanged);
                };
                let source = self.push_term(TypeTerm::StaticValue { value: argument });
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.relate_contextual_type_assignability(origin, source, constraint)
            }
            GenericSlot::Type {
                constraint: None, ..
            }
            | GenericSlot::VariadicType {
                constraint: None, ..
            }
            | GenericSlot::Static {
                constraint: None, ..
            }
            | GenericSlot::VariadicStatic {
                constraint: None, ..
            } => Ok(Progress::Unchanged),
        }
    }

    /// Decide whether call generic arguments satisfy declared constraints.
    fn reduce_call_generic_constraints(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameters: &[VariableId],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // combine each generic slot constraint
        for parameter in parameters {
            let Some(generic) = self.generic_slot(*parameter).cloned() else {
                panic!("generic parameter {parameter:?} is not a generic slot");
            };

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
        parameter: VariableId,
        generic: &GenericSlot,
    ) -> CompilerResult<Decision> {
        match generic {
            GenericSlot::Type {
                constraint: Some(constraint),
                ..
            }
            | GenericSlot::VariadicType {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_type_operand(substitution, parameter) else {
                    return Ok(Decision::Undecidable);
                };
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.decide_type_relation(TypeRelation::Assignable, argument, constraint)
            }
            GenericSlot::Static {
                constraint: Some(constraint),
                ..
            }
            | GenericSlot::VariadicStatic {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_static_operand(substitution, parameter)
                else {
                    return Ok(Decision::Undecidable);
                };
                let source = self.push_term(TypeTerm::StaticValue { value: argument });
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.decide_type_relation(TypeRelation::Assignable, source, constraint)
            }
            GenericSlot::Type {
                constraint: None, ..
            }
            | GenericSlot::VariadicType {
                constraint: None, ..
            }
            | GenericSlot::Static {
                constraint: None, ..
            }
            | GenericSlot::VariadicStatic {
                constraint: None, ..
            } => Ok(Decision::Yes),
        }
    }

    /// Solve omitted generic arguments from their defaults when inference has no input.
    fn solve_omitted_generic_defaults(
        &mut self,
        module: ModuleId,
        substitution: Substitution<'_>,
        parameters: &[VariableId],
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
        parameter: VariableId,
    ) -> CompilerResult<Progress> {
        let Some(generic) = self.generic_slot(parameter).cloned() else {
            panic!("generic parameter {parameter:?} is not a generic slot");
        };

        let progress = match generic {
            GenericSlot::Type {
                default: Some(default),
                ..
            }
            | GenericSlot::VariadicType {
                default: Some(default),
                ..
            } => {
                let Some(argument) = self.substitution_type_variable(substitution, parameter)
                else {
                    return Ok(Progress::Unchanged);
                };
                let default = self.substitute_type_operand(module, substitution, default)?;

                self.solve_default_type_variable(argument, default)?
            }
            GenericSlot::Static {
                default: Some(default),
                ..
            }
            | GenericSlot::VariadicStatic {
                default: Some(default),
                ..
            } => {
                let Some(argument) = self.substitution_static_variable(substitution, parameter)
                else {
                    return Ok(Progress::Unchanged);
                };
                let default = self.substitute_static_operand(module, substitution, default)?;

                self.solve_default_static_variable(argument, default)?
            }
            GenericSlot::Type { .. }
            | GenericSlot::VariadicType { .. }
            | GenericSlot::Static { .. }
            | GenericSlot::VariadicStatic { .. } => Progress::Unchanged,
        };

        Ok(progress)
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
            let void = self.push_term(void);

            return self.relate_contextual_type_assignability(origin, void, expected);
        };

        self.relate_contextual_type_assignability(origin, return_type, expected)
    }
}
