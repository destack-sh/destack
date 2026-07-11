use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CandidateOutcome, CandidatePass, CandidateVerdict, CheckFailure,
    CheckOutcome, Decision, DecisionKind, Dependency, FlowSite, Origin, PlaceUse, ProbeReason,
    SignatureRejection, SignatureSelection, answer,
};
use crate::{CompilerError, CompilerResult};

/// One callable candidate collected from a callee node.
struct CallableCandidate {
    /// The declaring symbol, when the callee names one.
    symbol: Option<dir::GlobalSymbolId>,
    /// The generic scope whose arguments are carried into this call.
    generic_scope: Option<dir::GlobalSymbolId>,
    /// The resolved receiver type for member callees.
    receiver: Option<dir::GlobalTypeId>,
    /// The receiver projection steps recorded by member lookup.
    adjustments: Vec<dir::Projection>,
    /// The callable type.
    ty: dir::GlobalTypeId,
    /// The owner generic arguments already selected by member lookup.
    generic_arguments: Vec<dir::GenericArgumentBinding>,
}

impl CallableCandidate {
    /// Return the durable symbol call candidate for this lookup candidate.
    fn resolution_candidate(&self, signature: &SignatureSelection) -> Option<dir::CallCandidate> {
        let generic_arguments = self.call_generic_arguments(&signature.generic_arguments);
        let mut adjustments = self.adjustments.clone();
        if let Some(steps) = &signature.receiver_steps {
            adjustments.extend(steps.iter().cloned());
        }

        Some(dir::CallCandidate {
            receiver: self.receiver,
            adjustments,
            generic_scope: self.generic_scope,
            symbol: self.symbol?,
            generic_arguments,
        })
    }

    /// Join lookup and selection bindings into one call binding list.
    fn call_generic_arguments(
        &self,
        signature_arguments: &[dir::GenericArgumentBinding],
    ) -> Vec<dir::GenericArgumentBinding> {
        let mut arguments =
            Vec::with_capacity(self.generic_arguments.len() + signature_arguments.len());
        for binding in &self.generic_arguments {
            let solved = signature_arguments
                .iter()
                .find(|solved| solved.parameter == binding.parameter);
            arguments.push(*solved.unwrap_or(binding));
        }
        for binding in signature_arguments {
            if !arguments
                .iter()
                .any(|existing| existing.parameter == binding.parameter)
            {
                arguments.push(*binding);
            }
        }

        arguments
    }
}

/// Callable candidates collected from one callee.
enum CallCandidates {
    /// At least one candidate must match.
    Any(SmallVec<[CallableCandidate; 2]>),
    /// Every candidate must match.
    All(SmallVec<[CallableCandidate; 2]>),
}

/// One accepted call selection.
struct CallSelection {
    /// The selected call resolution.
    resolution: dir::CallResolution,
    /// The selected call result type.
    return_type: dir::GlobalTypeId,
}

impl BodyState<'_, '_> {
    /// Select the callable meaning of one call node.
    pub(in crate::check) fn select_call(
        &mut self,
        site: FlowSite,
        callee: dir::LocalNodeId<dir::Expression>,
        generic_argument_nodes: &[dir::LocalNodeId<dir::GenericArgument>],
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();

        // collect explicit type arguments from the call node
        let mut argument_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in generic_argument_nodes {
            let argument = argument.into_global_any(module);
            let ty = answer!(self.node_type(argument)?);
            argument_types.push(ty);
        }

        // omitted constructor heads use the expected nominal target
        if self.is_inferred_call_head(module, callee) {
            return self.select_inferred_newtype_construct(
                site,
                node,
                origin,
                argument_nodes,
                &argument_types,
                expected_return,
            );
        }

        let callee_site = self.node_site(callee.into_global_any(module))?;

        // collect callable candidates from the callee
        let Some(callees) = answer!(self.callable_candidates(origin, module, callee_site)?) else {
            // rejected callees already reported their own diagnostic
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(CheckOutcome::Fails(CheckFailure::Relation)));
        };
        let candidates = match &callees {
            CallCandidates::Any(candidates) => candidates,
            CallCandidates::All(candidates) => candidates,
        };
        if candidates.is_empty() {
            let callee_type = answer!(self.node_type_at(callee_site)?);
            self.report_not_callable(origin, callee_type)?;
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(CheckOutcome::Fails(CheckFailure::Relation)));
        }

        // newtype targets construct through call expression form
        if let [
            CallableCandidate {
                symbol: Some(symbol),
                ..
            },
        ] = candidates.as_slice()
        {
            let symbol = *symbol;
            if matches!(self.symbol_kind(symbol), dir::SymbolKind::Newtype) {
                // newtype heads read as their declaration reference
                let callee_node = callee.into_global_any(module);
                if self.node_type_maybe(callee_node).is_none() {
                    let reference = self
                        .intern_type(module, dir::Type::Reference(dir::TypeReference { symbol }))?;
                    self.commit_node_type(callee_node, reference)?;
                }
                answer!(self.select_newtype_construct(
                    site,
                    node,
                    origin,
                    symbol,
                    argument_nodes,
                    &argument_types,
                    expected_return,
                )?);

                return Ok(Answer::Ready(CheckOutcome::Holds));
            }
        }

        // tagged variant members construct through call expression form
        if let [candidate] = candidates.as_slice() {
            let head = self.settled_root(candidate.ty)?;
            if let dir::Type::EnumMember(member) = self.ty(head)? {
                answer!(self.select_variant_construct(
                    site,
                    node,
                    origin,
                    member,
                    argument_nodes,
                    &argument_types,
                    expected_return,
                )?);

                return Ok(Answer::Ready(CheckOutcome::Holds));
            }
        }

        // union receivers must hold for every variant
        if let CallCandidates::All(candidates) = &callees {
            answer!(self.select_universal_call(
                site,
                node,
                origin,
                argument_nodes,
                candidates,
                &argument_types,
                expected_return,
            )?);

            return Ok(Answer::Ready(CheckOutcome::Holds));
        }

        // winnow candidates in declaration order: the first viable one
        //  wins, and an ambiguous one wins only when nothing is viable,
        //  so decisive later overloads beat undecidable earlier ones
        let is_single_candidate = candidates.len() == 1;
        let mut winner = None;
        let mut ambiguous = None;
        for candidate in candidates {
            if is_single_candidate {
                winner = Some(candidate);
                break;
            }
            let verdict = self.probe_candidate(ProbeReason::Signature, |state| {
                state.attempt_call(
                    CandidatePass::Winnow,
                    origin,
                    module,
                    candidate,
                    argument_nodes,
                    &argument_types,
                    expected_return,
                )
            })?;
            match verdict {
                CandidateVerdict::Rejected => {}
                CandidateVerdict::Viable => {
                    winner = Some(candidate);
                    break;
                }
                CandidateVerdict::Ambiguous => {
                    ambiguous.get_or_insert(candidate);
                }
            }
        }

        // confirm the winner outside any probe
        if let Some(candidate) = winner.or(ambiguous) {
            let attempt = self.attempt_call(
                CandidatePass::Confirm,
                origin,
                module,
                candidate,
                argument_nodes,
                &argument_types,
                expected_return,
            )?;

            match answer!(attempt) {
                CandidateOutcome::Accepted(selection) => {
                    answer!(self.commit_call_selection(
                        site,
                        node,
                        callee,
                        argument_nodes,
                        selection
                    )?);

                    return Ok(Answer::Ready(CheckOutcome::Holds));
                }
                CandidateOutcome::Rejected(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    self.report_signature_rejection(origin, module, argument_nodes, rejection)?;
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(CheckOutcome::Fails(CheckFailure::Relation)));
                }
                CandidateOutcome::Rejected(_) => {}
            }
        }

        // no candidate matched the arguments
        let arguments = answer!(self.infer_argument_types(site, argument_nodes)?);
        self.report_no_matching_call(origin, &arguments)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(CheckOutcome::Fails(CheckFailure::Relation)))
    }

    /// Return whether one call head is an inference hole.
    fn is_inferred_call_head(
        &self,
        module: ModuleId,
        callee: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        matches!(
            self.module(module).view().get(callee),
            dir::Expression::Infer {
                form: dir::InferForm::Hole,
                name: None,
            }
        )
    }

    /// Select newtype construction from an expected call result.
    fn select_inferred_newtype_construct(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        argument_types: &[dir::GlobalTypeId],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<CheckOutcome>> {
        let Some(expected_return) = expected_return else {
            self.report_cannot_infer_node(node)?;
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(CheckOutcome::Fails(CheckFailure::Relation)));
        };
        let target = answer!(self.reduce_type_head(origin, expected_return)?);
        let symbol = match self.ty(target)? {
            dir::Type::Instance(instance)
                if matches!(self.symbol_kind(instance.symbol), dir::SymbolKind::Newtype) =>
            {
                instance.symbol
            }
            _ => {
                self.report_invalid_inferred_construct_target(origin, target)?;
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(CheckOutcome::Fails(CheckFailure::Relation)));
            }
        };

        answer!(self.select_newtype_construct(
            site,
            node,
            origin,
            symbol,
            argument_nodes,
            argument_types,
            Some(expected_return),
        )?);

        Ok(Answer::Ready(CheckOutcome::Holds))
    }

    /// Collect callable candidates in declaration order from one callee node.
    fn callable_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        callee_site: FlowSite,
    ) -> CompilerResult<Answer<Option<CallCandidates>>> {
        let callee_node = callee_site.node;
        let callee = callee_node.into_typed::<dir::Expression>().local_id;

        // reference and member callees carry decided declaration meanings
        let uses_callee_decision = {
            let view = self.module(module).view();
            let expression = view.get(callee);

            expression.is_reference() || matches!(expression, dir::Expression::Member { .. })
        };
        if !uses_callee_decision {
            // every other callee calls through its function-typed value
            return self.value_callable_candidates(origin, callee_site);
        }

        match self.decision_kind(callee_node) {
            Some(DecisionKind::Name) => {
                let Some(resolution) = self
                    .resolutions(callee_node.module_id)
                    .name_resolution(callee_node)
                    .cloned()
                else {
                    return Ok(Answer::Ready(None));
                };
                let symbols = resolution
                    .symbols()
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();

                // value bindings call through their inferred node type,
                //  which carries flow narrowing; declarations carry
                //  their overload sets on the symbol
                let value_binding = symbols
                    .iter()
                    .all(|symbol| matches!(self.symbol_kind(*symbol), dir::SymbolKind::Variable));
                if value_binding {
                    return self.value_callable_candidates(origin, callee_site);
                }

                let mut blockers = SmallVec::<[Dependency; 2]>::new();
                let mut candidates = SmallVec::new();
                for symbol in symbols {
                    // skip declarations removed by statically false gates
                    if self.is_absent_symbol(symbol) {
                        continue;
                    }

                    let Some(ty) = self.symbol_type_maybe(symbol) else {
                        blockers.push(Dependency::SymbolType(symbol));

                        continue;
                    };
                    candidates.push(CallableCandidate {
                        symbol: Some(symbol),
                        generic_scope: None,
                        receiver: None,
                        adjustments: Vec::new(),
                        ty,
                        generic_arguments: Vec::new(),
                    });
                }
                if !blockers.is_empty() {
                    return Ok(Answer::Pending(blockers));
                }

                Ok(Answer::Ready(Some(CallCandidates::Any(candidates))))
            }
            Some(DecisionKind::Member) => {
                let Some(resolution) = self
                    .resolutions(callee_node.module_id)
                    .member_resolution(callee_node)
                    .cloned()
                else {
                    return Ok(Answer::Ready(None));
                };
                match &resolution.target {
                    // member candidates carry their receiver-applied types
                    dir::MemberTarget::Symbol(candidate) => {
                        let mut candidates = SmallVec::new();
                        candidates.push(CallableCandidate {
                            symbol: Some(candidate.symbol),
                            generic_scope: self.call_generic_scope(candidate)?,
                            receiver: Some(candidate.receiver),
                            adjustments: candidate.adjustments.clone(),
                            ty: candidate.ty,
                            generic_arguments: candidate.generic_arguments.clone(),
                        });

                        Ok(Answer::Ready(Some(CallCandidates::Any(candidates))))
                    }
                    // existential candidates need one match, universal candidates need every match
                    dir::MemberTarget::Existential(candidates)
                    | dir::MemberTarget::Universal(candidates) => {
                        let is_universal =
                            matches!(resolution.target, dir::MemberTarget::Universal(_));
                        let mut collected = SmallVec::<[_; 2]>::new();
                        for candidate in candidates {
                            collected.push(CallableCandidate {
                                symbol: Some(candidate.symbol),
                                generic_scope: self.call_generic_scope(candidate)?,
                                receiver: Some(candidate.receiver),
                                adjustments: candidate.adjustments.clone(),
                                ty: candidate.ty,
                                generic_arguments: candidate.generic_arguments.clone(),
                            });
                        }
                        let candidates = collected;

                        Ok(Answer::Ready(Some(if is_universal {
                            CallCandidates::All(candidates)
                        } else {
                            CallCandidates::Any(candidates)
                        })))
                    }
                    // field members call through their function-typed values
                    _ => self.value_callable_candidates(origin, callee_site),
                }
            }
            // rejected callees already carry a diagnostic
            Some(DecisionKind::Rejected) => Ok(Answer::Ready(None)),
            Some(other) => Err(CompilerError::Internal {
                message: format!("call callee {callee_node:?} decided as {other:?}"),
            }),
            // member callees decide by checking the callee in place;
            //  a checked callee without a decision calls through its value
            None => {
                let () = answer!(self.infer_node(callee_site, PlaceUse::Read)?);
                match self.decision_kind(callee_node) {
                    Some(_) => self.callable_candidates(origin, module, callee_site),
                    None => self.value_callable_candidates(origin, callee_site),
                }
            }
        }
    }

    /// Return the generic scope whose arguments participate in this call.
    fn call_generic_scope(
        &mut self,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        if candidate.space != dir::MemberSpace::Static {
            return Ok(Some(candidate.owner));
        }
        if matches!(
            self.definition(candidate.owner)?,
            Some(dir::Definition::Extension(_))
        ) {
            return Ok(Some(candidate.owner));
        }

        Ok(None)
    }

    /// Collect the callable candidate behind one function-typed callee value.
    fn value_callable_candidates(
        &mut self,
        origin: Origin,
        callee: FlowSite,
    ) -> CompilerResult<Answer<Option<CallCandidates>>> {
        let ty = answer!(self.infer_node_type(callee, PlaceUse::Read)?);
        let reduced = answer!(self.reduce_type_head(origin, ty)?);

        let mut candidates = SmallVec::new();
        match self.ty(reduced)? {
            dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_) => {
                candidates.push(CallableCandidate {
                    symbol: None,
                    generic_scope: None,
                    receiver: None,
                    adjustments: Vec::new(),
                    ty: reduced,
                    generic_arguments: Vec::new(),
                });

                Ok(Answer::Ready(Some(CallCandidates::Any(candidates))))
            }
            dir::Type::Variable(variable) => {
                Ok(Answer::pending([self.variable_dependency(variable)?]))
            }
            _ => Ok(Answer::Ready(Some(CallCandidates::Any(candidates)))),
        }
    }

    /// Attempt one callable candidate against collected arguments.
    fn attempt_call(
        &mut self,
        pass: CandidatePass,
        origin: Origin,
        module: ModuleId,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        argument_types: &[dir::GlobalTypeId],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<CandidateOutcome<CallSelection, SignatureRejection>>> {
        let arguments = self.callable_arguments(module, argument_nodes)?;
        let attempt = self.attempt_callable(
            pass,
            origin,
            candidate.ty,
            candidate.generic_scope,
            candidate.receiver,
            &candidate.generic_arguments,
            argument_types,
            &arguments,
            expected_return,
        )?;

        let signature = match answer!(attempt) {
            CandidateOutcome::Accepted(signature) => signature,
            CandidateOutcome::Rejected(rejection) => {
                return Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)));
            }
        };

        // build accepted resolution
        let target = match candidate.resolution_candidate(&signature) {
            Some(candidate) => dir::CallTarget::Symbol(candidate),
            None => dir::CallTarget::Expression {
                generic_arguments: signature.generic_arguments.clone(),
            },
        };
        let resolution = dir::CallResolution::new(
            target,
            Some(signature.callable),
            Self::parameter_types(&signature.parameters),
            self.argument_bindings(module, argument_nodes, &signature.parameters),
            signature.return_type,
        );

        Ok(Answer::Ready(CandidateOutcome::Accepted(CallSelection {
            resolution,
            return_type: signature.return_type,
        })))
    }

    /// Select one call on a union receiver.
    fn select_universal_call(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        candidates: &[CallableCandidate],
        argument_types: &[dir::GlobalTypeId],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<()>> {
        let mut targets = Vec::with_capacity(candidates.len());
        let mut returns = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut parameters: Option<SmallVec<[dir::FunctionParameterType; 4]>> = None;
        let arguments = self.callable_arguments(origin.module(), argument_nodes)?;

        for candidate in candidates {
            let attempt = self.attempt_callable(
                CandidatePass::Confirm,
                origin,
                candidate.ty,
                candidate.generic_scope,
                candidate.receiver,
                &candidate.generic_arguments,
                argument_types,
                &arguments,
                expected_return,
            )?;
            let CandidateOutcome::Accepted(signature) = answer!(attempt) else {
                // one rejecting variant rejects the whole union call
                let argument_types = answer!(self.infer_argument_types(site, argument_nodes)?);
                self.report_no_matching_call(origin, &argument_types)?;
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(()));
            };
            let Some(target) = candidate.resolution_candidate(&signature) else {
                continue;
            };
            targets.push(target);

            // collect every variant return for the normalized join below
            let return_type = self.settled_root(signature.return_type)?;
            returns.push(return_type);

            // keep the first parameter list: every variant matched the arguments above
            parameters.get_or_insert(signature.parameters.clone());
        }

        // join the variant returns into the call result
        let return_type = match returns.as_slice() {
            [single] => *single,
            _ => self.normalized_union_type(origin.module(), returns)?,
        };
        let Some(parameters) = parameters else {
            return Err(CompilerError::Internal {
                message: "universal call selected no candidate signatures".to_string(),
            });
        };

        let resolution = dir::CallResolution::new(
            dir::CallTarget::Universal(targets),
            None,
            Self::parameter_types(&parameters),
            self.argument_bindings(origin.module(), argument_nodes, &parameters),
            return_type,
        );
        self.check_arguments(site, argument_nodes, &resolution.arguments)?;
        self.commit_decision(node, Decision::Call(resolution))?;

        self.commit_node_type(node, return_type)?;

        Ok(Answer::Ready(()))
    }

    /// Commit one accepted call selection.
    fn commit_call_selection(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        callee: dir::LocalNodeId<dir::Expression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        mut selection: CallSelection,
    ) -> CompilerResult<Answer<()>> {
        self.check_arguments(site, argument_nodes, &selection.resolution.arguments)?;

        // preserve exact static-key expressions after normal signature selection
        if let Some(ty) = self.static_key_expression_type(site)? {
            selection.resolution.return_type = ty;
            selection.return_type = ty;
        }

        // type declaration callees with their instantiated callable
        let callee = callee.into_global_any(node.module_id);
        if let Some(callable) = selection.resolution.callable_type
            && self.node_type_maybe(callee).is_none()
        {
            self.commit_node_type(callee, callable)?;
        }

        self.commit_decision(node, Decision::Call(selection.resolution))?;
        self.commit_node_type(node, selection.return_type)?;

        Ok(Answer::Ready(()))
    }
}
