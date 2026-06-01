use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallCallee, CallDecision, CallFailure, CallTargetResolution, CallTerm, CandidateResolution,
    CheckState, Condition, ConstructCandidates, ConstructDecision, ConstructFailure,
    ConstructTargetResolution, Decision, FunctionParameter, FunctionTerm, GenericApplication,
    GenericArgument, GenericSlot, GenericSlotHeader, GenericSubstitution, GenericSubstitutionEntry,
    MemberCallCallee, Origin, Progress, ShapeMember, StaticRelation, StaticTerm, TypeLiteralTerm,
    TypeOperand, TypeRelation, TypeTerm, VariableId, VariableKind,
};
use smallvec::SmallVec;

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
pub(in crate::check) struct FunctionApplication {
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
    /// Return the concrete target symbol when this target has one.
    fn symbol(&self) -> Option<dir::GlobalSymbolId> {
        match self {
            Self::Symbol { symbol, .. } => Some(*symbol),
            Self::Construct(target) => Some(target.symbol()),
            Self::Expression | Self::Union { .. } => None,
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

/// Candidate call applicability before solving omitted generic arguments.
enum CallCandidateDecision {
    /// Candidate applicability waits for solver input.
    Pending,
    /// Candidate rejects the call.
    Rejected(CallFailure),
    /// Candidate accepts the call.
    Applicable,
}

/// Generic arguments visible while probing one call candidate.
struct CallCandidateSubstitution {
    /// Generic arguments known before candidate probing.
    entries: SmallVec<[CallCandidateSubstitutionEntry; 4]>,
}

impl CallCandidateSubstitution {
    /// Create an empty candidate substitution.
    fn empty() -> Self {
        Self {
            entries: SmallVec::new(),
        }
    }

    /// Return one substituted type argument.
    fn type_argument(&self, variable: VariableId) -> Option<TypeOperand> {
        self.entries.iter().find_map(|entry| {
            if entry.variable == variable {
                entry.argument.type_operand()
            } else {
                None
            }
        })
    }
}

/// One candidate generic substitution entry.
struct CallCandidateSubstitutionEntry {
    /// The declared generic variable.
    variable: VariableId,
    /// The applied generic argument.
    argument: GenericArgument,
}

/// Type facts inferred while probing one candidate.
struct CallCandidateInference {
    /// Type lower bounds keyed by generic parameter.
    type_lower: SmallVec<[(VariableId, TypeOperand); 4]>,
}

impl CallCandidateInference {
    /// Create empty candidate inference.
    fn empty() -> Self {
        Self {
            type_lower: SmallVec::new(),
        }
    }

    /// Add one lower type bound.
    fn add_type_lower(&mut self, variable: VariableId, operand: TypeOperand) {
        if !self
            .type_lower
            .iter()
            .any(|(known, known_operand)| *known == variable && *known_operand == operand)
        {
            self.type_lower.push((variable, operand));
        }
    }

    /// Return the first lower type bound for one generic parameter.
    fn type_lower(&self, variable: VariableId) -> Option<TypeOperand> {
        self.type_lower.iter().find_map(|(known, operand)| {
            if *known == variable {
                Some(*operand)
            } else {
                None
            }
        })
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
        if let Some(selection) = self.selected_call(call)? {
            return Ok(selection);
        }
        if let Some(result) = self.select_member_call(origin, call, expected)? {
            return Ok(result);
        }
        if let Some(result) = self.select_reference_call(origin, call, expected)? {
            return Ok(result);
        }

        self.select_expression_call(origin, call, expected)
    }

    /// Return the already chosen decision for one call.
    fn selected_call(&self, call: &CallTerm) -> CompilerResult<Option<CallableDispatch>> {
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

        if self.environment.language.item(symbol) == Some(dir::LanguageItem::Symbol) {
            let result = self.select_symbol_call(origin, call, expected, symbol)?;

            return Ok(Some(result));
        }

        let module = call.source.module_id;
        if self.symbol_kind(module, symbol) == Some(dir::SymbolKind::Newtype) {
            return self.select_newtype_call(origin, call, expected, symbol, &[]);
        }

        let ty = self.require_symbol_type(symbol);
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
        )?;

        Ok(Some(result))
    }

    /// Select the `Symbol(value)` compatibility constructor.
    fn select_symbol_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<CallableDispatch> {
        let string = self.push_term(TypeTerm::Literal(TypeLiteralTerm::Primitive(
            dir::PrimitiveType::String,
        )));
        let number = self.push_term(TypeTerm::Literal(TypeLiteralTerm::Primitive(
            dir::PrimitiveType::Float(dir::FloatType::Float64),
        )));
        let description = self.push_term(TypeTerm::Union {
            elements: vec![string.into(), number.into()],
        });
        let result = self.push_term(TypeTerm::Literal(TypeLiteralTerm::Primitive(
            dir::PrimitiveType::Symbol,
        )));
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: SmallVec::new(),
            this_parameter: None,
            parameters: SmallVec::from_vec(vec![FunctionParameter {
                ty: description.into(),
                static_slot: None,
                is_optional: true,
                is_rest: false,
            }]),
            return_type: Some(result.into()),
            is_generator: false,
        };

        let result = self.select_call_signature(
            origin,
            call.source.module_id,
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
        )?;

        Ok(result)
    }

    /// Select a newtype constructor exposed through call syntax.
    fn select_newtype_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<CallableDispatch>> {
        let module = call.source.module_id;
        let callee = self.require_symbol_type(symbol);
        let term = TypeTerm::Reference {
            origin: Origin::Node(call.source),
            symbol,
            arguments: arguments.to_vec().into(),
        };
        let candidates = match self.construct_candidates(module, callee, &term)? {
            ConstructCandidates::Pending => return Ok(Some(CallableDispatch::pending())),
            ConstructCandidates::Absent => return Ok(None),
            ConstructCandidates::Present(candidates) => candidates,
        };

        // choose the first compatible declaration-order newtype candidate
        for candidate in candidates {
            let probe = self.begin_inference_probe();
            let application = candidate.target.application().cloned();
            let target = CallableTarget::Construct(candidate.target);
            let result = self.select_call_signature(
                origin,
                module,
                call.source,
                Some(symbol),
                application,
                candidate.function,
                &call.generic_arguments,
                &call.arguments,
                &call.argument_values,
                expected,
                target,
            )?;
            match result {
                CallableDispatch::ConstructSelected { .. } => {
                    self.commit_inference_probe(probe);

                    return Ok(Some(result));
                }
                CallableDispatch::CallSelected { .. } => {
                    panic!("newtype dispatch produced a call selection");
                }
                CallableDispatch::Pending { .. } => {
                    self.drop_inference_probe(probe);

                    return Ok(Some(CallableDispatch::pending()));
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
        let member_match = match &member.callee {
            MemberCallCallee::Source { .. } => {
                self.member_type_candidates(origin, module, &receiver, &member.key)?
            }
            MemberCallCallee::Protocol { protocol } => self.member_type_candidates_for_protocol(
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
        let dispatch_candidates = member_matches
            .iter()
            .map(|member_match| {
                let target = if member_matches.len() == 1 {
                    CallableTarget::Symbol {
                        symbol: member_match.symbol,
                        receiver: Some(member.receiver),
                    }
                } else {
                    let candidates = member_matches
                        .iter()
                        .map(|candidate| CandidateResolution {
                            symbol: candidate.symbol,
                            application: candidate.application.clone(),
                        })
                        .collect();

                    CallableTarget::Union {
                        candidates,
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
    ) -> CompilerResult<CallableDispatch> {
        let decision = self.decide_call_signature(
            module,
            application.as_ref(),
            &function,
            generic_arguments,
            arguments,
            expected,
        )?;
        match decision {
            CallCandidateDecision::Applicable => {}
            CallCandidateDecision::Pending => return Ok(CallableDispatch::pending()),
            CallCandidateDecision::Rejected(failure) => {
                return Ok(CallableDispatch::call_rejected(failure));
            }
        }

        self.commit_call_signature(
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
        )
    }

    /// Commit one selected callable signature.
    fn commit_call_signature(
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
    ) -> CompilerResult<CallableDispatch> {
        let is_generic = !function.generic_parameters.is_empty();
        if generic_arguments.len() > function.generic_parameters.len()
            || (!is_generic && !generic_arguments.is_empty())
        {
            return Ok(CallableDispatch::call_rejected(CallFailure::NoMatch));
        }
        let instantiation = self.instantiate_function_signature(
            module,
            source,
            owner,
            target.symbol(),
            application,
            function,
            generic_arguments,
            argument_values,
        )?;
        let function = instantiation.function;
        let mut progress = Progress::Unchanged;

        // push argument types into fresh call generic variables
        if is_generic {
            progress = progress.merge(self.expect_call_arguments(
                origin,
                arguments,
                &function.parameters,
            )?);
            progress = progress.merge(self.expect_call_return(origin, &function, expected)?);
            progress = progress.merge(self.solve_omitted_generic_defaults(
                module,
                &instantiation.substitution,
                &instantiation.generic_parameters,
                generic_arguments.len(),
            )?);
            progress = progress.merge(self.expect_call_generic_constraints(
                origin,
                module,
                &instantiation.substitution,
                &instantiation.generic_parameters,
            )?);
        }

        let argument_decision = self.decide_call_arguments(arguments, &function.parameters)?;
        let return_type = self.decide_call_return(&function, expected)?;
        let generic_constraints = self.reduce_call_generic_constraints(
            module,
            &instantiation.substitution,
            &instantiation.generic_parameters,
        )?;
        match argument_decision.and(return_type).and(generic_constraints) {
            Decision::Yes => {
                progress = progress.merge(self.expect_call_arguments(
                    origin,
                    arguments,
                    &function.parameters,
                )?);
                progress = progress.merge(self.expect_call_return(origin, &function, expected)?);

                Ok(target.into_dispatch(instantiation.application, function, progress))
            }
            Decision::Undecidable => Ok(CallableDispatch::Pending { progress }),
            Decision::No if !progress.is_unchanged() => Ok(CallableDispatch::Pending { progress }),
            Decision::No => {
                if let Some((argument, parameter)) =
                    self.call_argument_type_failure(arguments, &function.parameters)?
                {
                    Ok(CallableDispatch::call_rejected(CallFailure::ArgumentType {
                        argument,
                        parameter,
                    }))
                } else {
                    Ok(CallableDispatch::call_rejected(CallFailure::NoMatch))
                }
            }
        }
    }

    /// Decide whether one callable signature can accept a call.
    fn decide_call_signature(
        &self,
        module: ModuleId,
        application: Option<&GenericApplication>,
        function: &FunctionTerm,
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallCandidateDecision> {
        let is_generic = !function.generic_parameters.is_empty();
        if generic_arguments.len() > function.generic_parameters.len()
            || (!is_generic && !generic_arguments.is_empty())
        {
            return Ok(CallCandidateDecision::Rejected(CallFailure::NoMatch));
        }

        let substitution = self.candidate_substitution(application, function, generic_arguments)?;
        let mut inference = CallCandidateInference::empty();
        let arguments_decision = self.decide_candidate_arguments(
            module,
            arguments,
            &function.parameters,
            &substitution,
            &mut inference,
        )?;
        let return_decision =
            self.decide_candidate_return(module, function, expected, &substitution, &inference)?;
        let decision = arguments_decision.and(return_decision);

        match decision {
            Decision::Yes => Ok(CallCandidateDecision::Applicable),
            Decision::Undecidable => Ok(CallCandidateDecision::Pending),
            Decision::No => {
                if let Some((argument, parameter)) = self.candidate_argument_type_failure(
                    module,
                    arguments,
                    &function.parameters,
                    &substitution,
                )? {
                    Ok(CallCandidateDecision::Rejected(CallFailure::ArgumentType {
                        argument,
                        parameter,
                    }))
                } else {
                    Ok(CallCandidateDecision::Rejected(CallFailure::NoMatch))
                }
            }
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

        // choose the first compatible declaration-order candidate
        for candidate in candidates {
            let probe = self.begin_inference_probe();
            let reduction = self.reduce_type_term(origin, &candidate.ty)?;
            let Some(term) = reduction.value else {
                self.drop_inference_probe(probe);
                saw_pending = true;
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
            )?;

            match result {
                CallableDispatch::CallSelected { .. } => {
                    self.commit_inference_probe(probe);

                    return Ok(result);
                }
                CallableDispatch::ConstructSelected { .. } => {
                    panic!("callable candidate dispatch produced a construct selection");
                }
                CallableDispatch::Pending { .. } => {
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

        // preserve the precise single-candidate argument error
        if callable_count == 1
            && let Some((argument, parameter)) = argument_failure
        {
            return Ok(CallableDispatch::call_rejected(CallFailure::ArgumentType {
                argument,
                parameter,
            }));
        }

        // distinguish rejected callable overloads from non-callable values
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
            TypeTerm::Variable(variable) => {
                let Some(term) = self.type_solution(*variable)? else {
                    return Ok(CallableSignature::Pending);
                };

                return self.call_signature(variable.module, &term);
            }
            TypeTerm::Function(function) => {
                CallableSignature::Present(self.term(*function).clone())
            }
            TypeTerm::Reference {
                origin: _,
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
    ) -> CompilerResult<Option<SmallVec<[FunctionParameter; 4]>>> {
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

    /// Return generic arguments known before candidate probing.
    fn candidate_substitution(
        &self,
        application: Option<&GenericApplication>,
        function: &FunctionTerm,
        generic_arguments: &[GenericArgument],
    ) -> CompilerResult<CallCandidateSubstitution> {
        let mut substitution = CallCandidateSubstitution::empty();

        // collect only arguments that already exist before candidate selection
        for (index, parameter) in function.generic_parameters.iter().enumerate() {
            let slot = self.generic_parameter_slot(*parameter);
            let is_static = self.generic_parameter_is_static(*parameter);
            let argument = if let Some(argument) = generic_arguments.get(index) {
                Some(self.select_argument_for_static_slot(argument, is_static))
            } else {
                application
                    .filter(|application| application.owner == slot.owner)
                    .and_then(|application| application.arguments.get(slot.index.0 as usize))
                    .map(|argument| self.select_argument_for_static_slot(argument, is_static))
            };
            let Some(argument) = argument else {
                continue;
            };

            substitution.entries.push(CallCandidateSubstitutionEntry {
                variable: *parameter,
                argument,
            });
        }

        Ok(substitution)
    }

    /// Decide whether call arguments are assignable under candidate-local inference.
    fn decide_candidate_arguments(
        &self,
        module: ModuleId,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
        substitution: &CallCandidateSubstitution,
        inference: &mut CallCandidateInference,
    ) -> CompilerResult<Decision> {
        if !self.call_arity_accepts(arguments, parameters) {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        // every argument must be assignable to the corresponding parameter
        for (argument, parameter) in arguments.iter().zip(parameters) {
            decision = decision.and(self.decide_candidate_argument(
                module,
                *argument,
                parameter.ty,
                substitution,
                inference,
            )?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide one argument against one candidate parameter.
    fn decide_candidate_argument(
        &self,
        module: ModuleId,
        argument: TypeOperand,
        parameter: TypeOperand,
        substitution: &CallCandidateSubstitution,
        inference: &mut CallCandidateInference,
    ) -> CompilerResult<Decision> {
        self.decide_candidate_assignability(module, argument, parameter, substitution, inference)
    }

    /// Decide assignability while collecting local generic inference facts.
    fn decide_candidate_assignability(
        &self,
        module: ModuleId,
        source: TypeOperand,
        target: TypeOperand,
        substitution: &CallCandidateSubstitution,
        inference: &mut CallCandidateInference,
    ) -> CompilerResult<Decision> {
        if let Some(target) = self.candidate_type_argument(module, target, substitution)? {
            return self.decide_type_relation(TypeRelation::Assignable, source, target);
        }
        if let Some(parameter) = self.candidate_type_parameter(module, target)? {
            if let Some(inferred) = inference.type_lower(parameter) {
                return self.decide_type_relation(TypeRelation::Assignable, source, inferred);
            }
            if self.type_operand_term(source)?.is_some() {
                inference.add_type_lower(parameter, source);

                return Ok(Decision::Yes);
            }

            return Ok(Decision::Undecidable);
        }

        let Some(source_term) = self.type_operand_term(source)? else {
            return Ok(Decision::Undecidable);
        };
        let Some(target_term) = self.type_operand_term(target)? else {
            return Ok(Decision::Undecidable);
        };

        self.decide_candidate_term_assignability(
            module,
            source,
            target,
            &source_term,
            &target_term,
            substitution,
            inference,
        )
    }

    /// Decide solved candidate terms while collecting local generic inference facts.
    fn decide_candidate_term_assignability(
        &self,
        module: ModuleId,
        source: TypeOperand,
        target: TypeOperand,
        source_term: &TypeTerm,
        target_term: &TypeTerm,
        substitution: &CallCandidateSubstitution,
        inference: &mut CallCandidateInference,
    ) -> CompilerResult<Decision> {
        let decision = match (source_term, target_term) {
            (
                TypeTerm::Array { element: source },
                TypeTerm::Array { element: target }
                | TypeTerm::Slice {
                    element: target, ..
                },
            )
            | (
                TypeTerm::Slice {
                    element: source, ..
                },
                TypeTerm::Slice {
                    element: target, ..
                },
            )
            | (
                TypeTerm::FixedArray {
                    element: source, ..
                },
                TypeTerm::Slice {
                    element: target, ..
                },
            ) => self.decide_candidate_assignability(
                module,
                *source,
                *target,
                substitution,
                inference,
            )?,
            (
                TypeTerm::FixedArray {
                    element: source_element,
                    length: source_length,
                    ..
                },
                TypeTerm::FixedArray {
                    element: target_element,
                    length: target_length,
                    ..
                },
            ) => {
                let element = self.decide_candidate_assignability(
                    module,
                    *source_element,
                    *target_element,
                    substitution,
                    inference,
                )?;
                let length = self.decide_static_relation(
                    StaticRelation::Equal,
                    *source_length,
                    *target_length,
                )?;

                element.and(length)
            }
            (
                TypeTerm::Tuple {
                    elements: source_elements,
                    ..
                },
                TypeTerm::Tuple {
                    elements: target_elements,
                    ..
                },
            ) if source_elements.len() == target_elements.len() => {
                let mut decision = Decision::Yes;

                // compare tuple elements by position
                for (source, target) in source_elements.iter().zip(target_elements) {
                    decision = decision.and(self.decide_candidate_assignability(
                        module,
                        source.ty,
                        target.ty,
                        substitution,
                        inference,
                    )?);
                    if decision == Decision::No {
                        return Ok(Decision::No);
                    }
                }

                decision
            }
            (TypeTerm::Union { elements }, _) => {
                let mut decision = Decision::Yes;

                // every source union member must fit the target
                for element in elements {
                    decision = decision.and(self.decide_candidate_assignability(
                        module,
                        *element,
                        target,
                        substitution,
                        inference,
                    )?);
                    if decision == Decision::No {
                        return Ok(Decision::No);
                    }
                }

                decision
            }
            (_, TypeTerm::Union { elements }) => {
                let mut decision = Decision::No;

                // at least one target union member must accept the source
                for element in elements {
                    decision = decision.or(self.decide_candidate_assignability(
                        module,
                        source,
                        *element,
                        substitution,
                        inference,
                    )?);
                    if decision == Decision::Yes {
                        return Ok(Decision::Yes);
                    }
                }

                decision
            }
            _ => self.decide_type_relation(TypeRelation::Assignable, source, target)?,
        };

        Ok(decision)
    }

    /// Return the substituted type argument for one direct generic parameter.
    fn candidate_type_argument(
        &self,
        module: ModuleId,
        parameter: TypeOperand,
        substitution: &CallCandidateSubstitution,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(parameter) = self.candidate_type_parameter(module, parameter)? else {
            return Ok(None);
        };
        let Some(argument) = substitution.type_argument(parameter) else {
            return Ok(None);
        };

        Ok(Some(argument))
    }

    /// Return the generic parameter represented by one type operand.
    fn candidate_type_parameter(
        &self,
        module: ModuleId,
        parameter: TypeOperand,
    ) -> CompilerResult<Option<VariableId>> {
        if let Some(parameter) = parameter.variable()
            && let Some(parameter) = self.candidate_type_parameter_variable(parameter)?
        {
            return Ok(Some(parameter));
        }

        let Some(term) = self.type_operand_term(parameter)? else {
            return Ok(None);
        };

        self.candidate_type_parameter_term(module, &term)
    }

    /// Return the generic parameter represented by one variable.
    fn candidate_type_parameter_variable(
        &self,
        parameter: VariableId,
    ) -> CompilerResult<Option<VariableId>> {
        let Some(generic) = self.generic_slot(parameter) else {
            let Some(term) = self.type_solution(parameter)? else {
                return Ok(None);
            };

            return self.candidate_type_parameter_term(parameter.module, &term);
        };
        if generic.is_type() {
            Ok(Some(parameter))
        } else {
            Ok(None)
        }
    }

    /// Return the generic parameter represented by one type term.
    fn candidate_type_parameter_term(
        &self,
        module: ModuleId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<VariableId>> {
        match term {
            TypeTerm::Variable(parameter) => self.candidate_type_parameter_variable(*parameter),
            TypeTerm::Parameter(slot) => Ok(self.resolve_type_generic_slot(module, *slot)),
            _ => Ok(None),
        }
    }

    /// Decide whether a candidate return type can flow into the expected result.
    fn decide_candidate_return(
        &self,
        module: ModuleId,
        function: &FunctionTerm,
        expected: Option<VariableId>,
        substitution: &CallCandidateSubstitution,
        inference: &CallCandidateInference,
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
        if let Some(return_type) =
            self.candidate_type_argument(module, return_type, substitution)?
        {
            return self.decide_type_relation(TypeRelation::Assignable, return_type, expected);
        }
        if let Some(parameter) = self.candidate_type_parameter(module, return_type)?
            && let Some(inferred) = inference.type_lower(parameter)
        {
            return self.decide_type_relation(TypeRelation::Assignable, inferred, expected);
        }

        self.decide_type_relation(TypeRelation::Assignable, return_type, expected)
    }

    /// Return the first solved candidate argument type failure.
    fn candidate_argument_type_failure(
        &self,
        module: ModuleId,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
        substitution: &CallCandidateSubstitution,
    ) -> CompilerResult<Option<(TypeOperand, TypeOperand)>> {
        if !self.call_arity_accepts(arguments, parameters) {
            return Ok(None);
        }

        for (argument, parameter) in arguments.iter().zip(parameters) {
            let parameter = self
                .candidate_type_argument(module, parameter.ty, substitution)?
                .unwrap_or(parameter.ty);
            let decision =
                self.decide_type_relation(TypeRelation::Assignable, *argument, parameter)?;
            if decision == Decision::No {
                return Ok(Some((*argument, parameter)));
            }
        }

        Ok(None)
    }

    /// Instantiate function generics as call-local inference variables.
    pub(in crate::check) fn instantiate_function_signature(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        _target: Option<dir::GlobalSymbolId>,
        application: Option<GenericApplication>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
    ) -> CompilerResult<FunctionApplication> {
        if function.generic_parameters.is_empty() {
            return Ok(FunctionApplication {
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
        function.generic_parameters.clear();
        let owner = owner.or(application_owner);
        let application = application.or_else(|| {
            owner.map(|owner| self.attach_generic_application(source, owner, arguments))
        });

        let instantiation = FunctionApplication {
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
            .position(|candidate| candidate.static_slot == Some(parameter))
        else {
            return Ok(None);
        };
        let Some(value) = argument_values.get(index).cloned() else {
            return Ok(None);
        };
        let origin = Origin::Node(value.clone().into_any());
        let variable = self.allocate_variable(module, VariableKind::Static, origin);
        let term = StaticTerm::Expression(value);

        self.equate_static(variable, term, Condition::Always);

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
        substitution: &GenericSubstitution,
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
        substitution: &GenericSubstitution,
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
        substitution: &GenericSubstitution,
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
        substitution: &GenericSubstitution,
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
        substitution: &GenericSubstitution,
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
        substitution: &GenericSubstitution,
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
