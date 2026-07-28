use std::slice;

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CallableArgument, CandidateOutcome, CandidateVerdict, CheckFailure,
    CheckOutcome, Decision, DecisionKind, Dependency, Expectation, FlowSite, InferMode, MemoryRank,
    Origin, PlaceUse, SignatureMatch, SignatureSelection, TypeSubstitution, Value, ValueCheck,
    ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

/// The operation performed by one callable candidate.
#[derive(Debug)]
enum CallableTarget {
    /// Call a function-typed runtime value.
    Expression,
    /// Call one declaration-backed function.
    Symbol(dir::GlobalSymbolId),
    /// Construct one newtype.
    Newtype(dir::GlobalSymbolId),
    /// Construct one derived tagged variant.
    Variant(dir::VariantCase),
}

/// One checked receiver and its durable member resolution.
struct CallableReceiver {
    /// The receiver stored in the selected call.
    resolution: dir::MemberReceiver,
    /// The checked receiver value used by signature selection.
    value: Value,
}

/// One callable candidate collected from a callee node.
struct CallableCandidate {
    /// The operation performed after signature selection.
    target: CallableTarget,
    /// The generic scope whose arguments are carried into this call.
    generic_scope: Option<dir::GlobalSymbolId>,
    /// The receiver selected for member callees.
    receiver: Option<CallableReceiver>,
    /// The callable type.
    ty: dir::GlobalTypeId,
    /// The owner generic arguments already selected by member lookup.
    generic_arguments: Vec<dir::GenericArgumentBinding>,
}

impl CallableCandidate {
    /// Return the receiver after signature selection.
    fn selected_receiver(&self, signature: &SignatureSelection) -> Option<dir::MemberReceiver> {
        let mut receiver = self.receiver.as_ref()?.resolution.clone();
        if let Some(steps) = &signature.receiver_steps {
            receiver
                .adjusted_mut()
                .adjustments
                .extend(steps.iter().cloned());
        }

        Some(receiver)
    }

    /// Return the durable target for one declaration-backed function.
    fn function_target(
        &self,
        signature: &SignatureSelection,
        receiver: Option<dir::AdjustedReceiver>,
    ) -> CompilerResult<dir::FunctionTarget> {
        let CallableTarget::Symbol(symbol) = &self.target else {
            return Err(CompilerError::Internal {
                message: format!("call target {:?} is not a symbol", self.target),
            });
        };

        Ok(dir::FunctionTarget {
            receiver,
            generic_scope: self.generic_scope,
            symbol: *symbol,
            generic_arguments: signature.generic_arguments.clone(),
        })
    }
}

/// One runtime callee and its declaration alternatives.
struct CallableArm {
    /// The overloads declared for this runtime callee.
    overloads: SmallVec<[CallableCandidate; 2]>,
}

/// Callable alternatives grouped by runtime callee.
struct CallCandidates {
    /// The possible runtime callees.
    arms: SmallVec<[CallableArm; 2]>,
}

impl BodyState<'_, '_> {
    /// Collect callable candidates in declaration order from one callee node.
    fn callable_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        callee_site: FlowSite,
    ) -> CompilerResult<Answer<Option<CallCandidates>>> {
        let callee_node = callee_site.node;
        let callee = callee_node.into_typed::<dir::Expression>().local_id;

        // use the recorded callee decision for reference and member callees
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
                    return Err(CompilerError::Internal {
                        message: format!(
                            "call callee {callee_node:?} has a name decision without a resolution"
                        ),
                    });
                };
                let symbols = resolution
                    .symbols()
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();

                // call a value binding through its inferred node type,
                //  and a declaration through its symbol's overload set
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
                    let target = if matches!(self.symbol_kind(symbol), dir::SymbolKind::Newtype) {
                        CallableTarget::Newtype(symbol)
                    } else {
                        CallableTarget::Symbol(symbol)
                    };
                    let ty = match &target {
                        CallableTarget::Newtype(_) => ty,
                        _ => {
                            let Some(ty) = answer!(self.callable_type(origin, ty)?) else {
                                continue;
                            };

                            ty
                        }
                    };

                    candidates.push(CallableCandidate {
                        target,
                        generic_scope: None,
                        receiver: None,
                        ty,
                        generic_arguments: Vec::new(),
                    });
                }
                if !blockers.is_empty() {
                    return Ok(Answer::Pending(blockers));
                }

                let mut arms = SmallVec::new();
                arms.push(CallableArm {
                    overloads: candidates,
                });

                Ok(Answer::Ready(Some(CallCandidates { arms })))
            }
            Some(DecisionKind::Member) => {
                let dir::Expression::Member { left, .. } = self.module(module).view().get(callee)
                else {
                    return Err(CompilerError::Internal {
                        message: format!("member call callee {callee_node:?} is not a member"),
                    });
                };
                let receiver_site = self.node_site(left.into_global_any(module))?;
                let receiver_type = answer!(self.infer_node_type(receiver_site, PlaceUse::Read)?);
                let receiver = answer!(self.expression_value(receiver_site, receiver_type)?);
                let Some(resolution) = self
                    .resolutions(callee_node.module_id)
                    .member_resolution(callee_node)
                    .cloned()
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "call callee {callee_node:?} has a member decision without a resolution"
                        ),
                    });
                };
                let candidates = match &resolution {
                    dir::OperationResolution::One(access) => {
                        answer!(self.member_access_call_candidates(origin, receiver, access)?)
                    }
                    dir::OperationResolution::Union { arms, .. } => {
                        let mut runtime_arms = SmallVec::with_capacity(arms.len());
                        for access in arms {
                            let candidates = answer!(
                                self.member_access_call_candidates(origin, receiver, access)?
                            );
                            runtime_arms.extend(candidates.arms);
                        }

                        CallCandidates { arms: runtime_arms }
                    }
                };

                Ok(Answer::Ready(Some(candidates)))
            }
            // skip rejected callees, they already reported a diagnostic
            Some(DecisionKind::Rejected) => Ok(Answer::Ready(None)),
            Some(other) => Err(CompilerError::Internal {
                message: format!("call callee {callee_node:?} decided as {other:?}"),
            }),
            // member callees decide by checking the callee in place;
            //  a checked callee without a decision calls through its value
            None => {
                let _ = answer!(self.infer_node(callee_site, PlaceUse::Read, InferMode::Exact)?);
                match self.decision_kind(callee_node) {
                    Some(_) => self.callable_candidates(origin, module, callee_site),
                    None => self.value_callable_candidates(origin, callee_site),
                }
            }
        }
    }

    /// Build callable candidates from one singular member access.
    fn member_access_call_candidates(
        &mut self,
        origin: Origin,
        receiver: Value,
        access: &dir::MemberAccess,
    ) -> CompilerResult<Answer<CallCandidates>> {
        if let Some(arm) = answer!(self.member_target_callable_arm(receiver, &access.target)?) {
            let mut arms = SmallVec::new();
            arms.push(arm);

            return Ok(Answer::Ready(CallCandidates { arms }));
        }

        // stored and computed members call through their selected value
        let arms = answer!(self.callable_value_arms(origin, access.ty)?);

        Ok(Answer::Ready(CallCandidates { arms }))
    }

    /// Build callable candidates from one singular member target.
    fn member_target_callable_arm(
        &mut self,
        receiver: Value,
        target: &dir::MemberTarget,
    ) -> CompilerResult<Answer<Option<CallableArm>>> {
        match target {
            dir::MemberTarget::Symbol(candidate) => {
                let mut candidates = SmallVec::new();
                if let Some(candidate) = answer!(self.member_call_candidate(receiver, candidate)?) {
                    candidates.push(candidate);
                }

                Ok(Answer::Ready(Some(CallableArm {
                    overloads: candidates,
                })))
            }
            dir::MemberTarget::Existential(targets) => {
                let mut candidates = SmallVec::new();
                for target in targets {
                    let dir::MemberTarget::Symbol(candidate) = target else {
                        return Err(CompilerError::Internal {
                            message: "member overload set contains a non-symbol target".to_string(),
                        });
                    };
                    if let Some(candidate) =
                        answer!(self.member_call_candidate(receiver, candidate)?)
                    {
                        candidates.push(candidate);
                    }
                }

                Ok(Answer::Ready(Some(CallableArm {
                    overloads: candidates,
                })))
            }
            dir::MemberTarget::Intersection(targets) => {
                let mut candidates = SmallVec::new();
                for target in targets {
                    let Some(selected) =
                        answer!(self.member_target_callable_arm(receiver, target)?)
                    else {
                        continue;
                    };
                    candidates.extend(selected.overloads);
                }

                Ok(Answer::Ready(Some(CallableArm {
                    overloads: candidates,
                })))
            }
            dir::MemberTarget::Projection { .. }
            | dir::MemberTarget::Field(_)
            | dir::MemberTarget::Call(_)
            | dir::MemberTarget::Index(_) => Ok(Answer::Ready(None)),
        }
    }

    /// Build one callable candidate from a declaration-backed member target.
    fn member_call_candidate(
        &mut self,
        receiver: Value,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<Answer<Option<CallableCandidate>>> {
        let Some(ty) = candidate.callable_type else {
            return Ok(Answer::Ready(None));
        };
        let target = self.member_callable_target(candidate)?;
        let receiver = match &target {
            CallableTarget::Expression => None,
            _ => Some(CallableReceiver {
                resolution: candidate.receiver.clone(),
                value: Value {
                    ty: candidate.receiver.ty(),
                    ..receiver
                },
            }),
        };
        let candidate = CallableCandidate {
            target,
            generic_scope: self.call_generic_scope(candidate)?,
            receiver,
            ty,
            generic_arguments: candidate.generic_arguments.clone(),
        };

        Ok(Answer::Ready(Some(candidate)))
    }

    /// Return the operation selected by one declaration-backed member.
    fn member_callable_target(
        &mut self,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<CallableTarget> {
        match self.symbol_kind(candidate.symbol) {
            dir::SymbolKind::AssociatedConst => return Ok(CallableTarget::Expression),
            dir::SymbolKind::Function => return Ok(CallableTarget::Symbol(candidate.symbol)),
            dir::SymbolKind::Variant => {}
            kind => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "callable member {:?} has non-callable symbol kind {kind:?}",
                        candidate.symbol
                    ),
                });
            }
        }

        let variant = match self.definition(candidate.owner)? {
            Some(dir::Definition::Newtype(definition)) => {
                definition.tagged_variant_by_symbol(candidate.symbol)
            }
            _ => None,
        };
        let Some(variant) = variant else {
            return Err(CompilerError::Internal {
                message: format!(
                    "callable variant member {:?} is absent from its owner",
                    candidate.symbol
                ),
            });
        };
        let case = dir::VariantCase {
            owner: candidate.owner,
            key: variant.key,
            variant: candidate.symbol,
        };

        Ok(CallableTarget::Variant(case))
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

    /// Return the invocable form of one type.
    fn callable_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        let callable = matches!(
            self.ty(ty)?,
            dir::Type::FunctionSignature(_)
                | dir::Type::Function(_)
                | dir::Type::FunctionPointer(_)
        )
        .then_some(ty);

        Ok(Answer::Ready(callable))
    }

    /// Collect the callable candidate behind one function-typed callee value.
    fn value_callable_candidates(
        &mut self,
        origin: Origin,
        callee: FlowSite,
    ) -> CompilerResult<Answer<Option<CallCandidates>>> {
        let ty = answer!(self.infer_node_type(callee, PlaceUse::Read)?);
        let ty = answer!(self.strip_form(origin, ty)?);
        let arms = answer!(self.callable_value_arms(origin, ty)?);

        Ok(Answer::Ready(Some(CallCandidates { arms })))
    }

    /// Collect runtime callable arms from one value type.
    fn callable_value_arms(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[CallableArm; 2]>>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        // distribute runtime union alternatives into independent arms
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements = self.type_ids(ty.module_id, union.elements)?.to_vec();
            let mut arms = SmallVec::with_capacity(elements.len());
            for element in elements {
                let nested = answer!(self.callable_value_arms(origin, element)?);
                arms.extend(nested);
            }

            return Ok(Answer::Ready(arms));
        }

        // intersections contribute overload alternatives to one runtime value
        let overloads = answer!(self.callable_value_overloads(origin, ty)?);
        let mut arms = SmallVec::new();
        arms.push(CallableArm { overloads });

        Ok(Answer::Ready(arms))
    }

    /// Collect overload alternatives from one runtime callable value.
    fn callable_value_overloads(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[CallableCandidate; 2]>>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        // flatten intersection signatures into one declaration alternative set
        if let dir::Type::Intersection(intersection) = self.ty(ty)? {
            let elements = self.type_ids(ty.module_id, intersection.elements)?.to_vec();
            let mut overloads = SmallVec::with_capacity(elements.len());
            for element in elements {
                let nested = answer!(self.callable_value_overloads(origin, element)?);
                overloads.extend(nested);
            }

            return Ok(Answer::Ready(overloads));
        }

        // retain one candidate for an invocable value representation
        let mut overloads = SmallVec::new();
        if let Some(ty) = answer!(self.callable_type(origin, ty)?) {
            overloads.push(CallableCandidate {
                target: CallableTarget::Expression,
                generic_scope: None,
                receiver: None,
                ty,
                generic_arguments: Vec::new(),
            });
        }

        Ok(Answer::Ready(overloads))
    }
}

/// The best overload combination selected across every runtime arm.
struct OverloadSelection<'candidate> {
    /// The selected callable for each runtime arm.
    candidates: SmallVec<[&'candidate CallableCandidate; 2]>,
    /// The rejected candidate descriptions.
    rejections: Vec<String>,
}

/// The signatures selected across every runtime callee.
struct CallSelection {
    /// The selected signatures in runtime-arm order.
    signatures: SmallVec<[SignatureSelection; 2]>,
    /// The greatest memory conversion required by any selected arm.
    rank: MemoryRank,
}

/// Result of matching one call across every possible runtime callee.
enum CallMatch {
    /// Every runtime callee accepts the invocation and its expected result.
    Selected(CallSelection),
    /// Every runtime callee accepts the invocation but not its expected result.
    ReturnMismatch(CallSelection),
    /// At least one runtime callee rejects the invocation.
    Inapplicable(String),
}

impl BodyState<'_, '_> {
    /// Attempt one callable candidate against collected arguments.
    fn attempt_call(
        &mut self,
        origin: Origin,
        candidate: &CallableCandidate,
        arguments: &[CallableArgument],
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<SignatureMatch>> {
        let matched = answer!(self.attempt_callable(
            origin,
            candidate.ty,
            candidate.generic_scope,
            candidate.receiver.as_ref().map(|receiver| receiver.value),
            &candidate.generic_arguments,
            argument_types,
            arguments,
            expectation,
        )?);

        Ok(Answer::Ready(matched))
    }

    /// Select one jointly viable overload from every runtime arm.
    fn select_overloads<'candidate>(
        &mut self,
        origin: Origin,
        arms: &'candidate [CallableArm],
        arguments: &[CallableArgument],
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<OverloadSelection<'candidate>>> {
        if arms.iter().any(|arm| arm.overloads.is_empty()) {
            return Ok(Answer::Ready(OverloadSelection {
                candidates: SmallVec::new(),
                rejections: Vec::new(),
            }));
        }

        // one declaration on one runtime arm needs no ranking probe
        if let [CallableArm { overloads }] = arms
            && let [candidate] = overloads.as_slice()
        {
            let candidates = SmallVec::from_slice(&[candidate]);

            return Ok(Answer::Ready(OverloadSelection {
                candidates,
                rejections: Vec::new(),
            }));
        }

        let mut indices = vec![0; arms.len()];
        let mut winner = None;
        let mut indeterminate = None;
        let mut rejections = Vec::new();

        // rank complete overload combinations under one inference transaction
        loop {
            let selected = arms
                .iter()
                .zip(&indices)
                .map(|(arm, index)| &arm.overloads[*index])
                .collect::<SmallVec<[_; 2]>>();
            let mut rank = MemoryRank::Exact;
            let (verdict, rejection) = answer!(self.probe_candidate_noted(
                |state| {
                    let matched = state.attempt_calls(
                        origin,
                        &selected,
                        arguments,
                        argument_types,
                        expectation,
                    )?;
                    match matched {
                        Answer::Ready(CallMatch::Selected(selection)) => {
                            rank = selection.rank;

                            Ok(Answer::Ready(CandidateOutcome::Accepted(())))
                        }
                        Answer::Ready(CallMatch::ReturnMismatch(selection)) => {
                            rank = selection.rank;

                            Ok(Answer::Ready(CandidateOutcome::Rejected(
                                "return type does not satisfy the expected type".to_string(),
                            )))
                        }
                        Answer::Ready(CallMatch::Inapplicable(rejection)) => {
                            Ok(Answer::Ready(CandidateOutcome::Rejected(rejection)))
                        }
                        Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                    }
                },
                |_state, rejection| Ok(rejection.clone()),
            )?);
            match verdict {
                CandidateVerdict::Rejected => rejections.extend(rejection),
                CandidateVerdict::Viable => {
                    if winner
                        .as_ref()
                        .is_none_or(|(best, _): &(MemoryRank, Vec<usize>)| rank < *best)
                    {
                        winner = Some((rank, indices.clone()));
                    }
                }
                CandidateVerdict::Indeterminate => {
                    indeterminate.get_or_insert_with(|| indices.clone());
                }
            }
            if rank == MemoryRank::Exact && matches!(verdict, CandidateVerdict::Viable) {
                break;
            }

            // advance the declaration-order Cartesian traversal
            let mut advanced = false;
            for position in (0..indices.len()).rev() {
                if indices[position] + 1 < arms[position].overloads.len() {
                    indices[position] += 1;
                    indices[position + 1..].fill(0);
                    advanced = true;

                    break;
                }
            }
            if !advanced {
                break;
            }
        }

        // collect the best viable combination in runtime-arm order
        let indices = winner.map(|(_, indices)| indices).or(indeterminate);
        let mut candidates = SmallVec::new();
        if let Some(indices) = indices {
            for (arm, index) in arms.iter().zip(indices) {
                candidates.push(&arm.overloads[index]);
            }
        }

        Ok(Answer::Ready(OverloadSelection {
            candidates,
            rejections,
        }))
    }

    /// Attempt every selected runtime arm as one inference transaction.
    fn attempt_calls(
        &mut self,
        origin: Origin,
        candidates: &[&CallableCandidate],
        arguments: &[CallableArgument],
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<CallMatch>> {
        let mut signatures = SmallVec::with_capacity(candidates.len());
        let mut coercions = SmallVec::<[(dir::GlobalNodeIdAny, dir::Coercion); 4]>::new();
        let mut has_return_mismatch = false;
        let mut rank = MemoryRank::Exact;

        // require every runtime arm under the same inference state
        for candidate in candidates {
            let matched = answer!(self.attempt_call(
                origin,
                candidate,
                arguments,
                argument_types,
                expectation,
            )?);
            let signature = match matched {
                SignatureMatch::Selected(signature) => signature,
                SignatureMatch::ReturnMismatch(signature) => {
                    has_return_mismatch = true;

                    signature
                }
                SignatureMatch::Invalid { rejection, .. }
                | SignatureMatch::Inapplicable(rejection) => {
                    let rejection = self.check.describe_signature_rejection(
                        origin.module(),
                        candidate.ty,
                        &rejection,
                    )?;

                    return Ok(Answer::Ready(CallMatch::Inapplicable(rejection)));
                }
            };
            rank = rank.max(signature.rank);

            // require one uniform argument conversion across every runtime arm
            for (node, coercion) in &signature.coercions {
                match coercions.iter().find(|(source, _)| source == node) {
                    Some((_, selected)) if selected != coercion => {
                        return Ok(Answer::Ready(CallMatch::Inapplicable(
                            "runtime call arms require incompatible argument conversions"
                                .to_string(),
                        )));
                    }
                    Some(_) => {}
                    None => coercions.push((*node, coercion.clone())),
                }
            }
            signatures.push(signature);
        }

        let selection = CallSelection { signatures, rank };
        if has_return_mismatch {
            Ok(Answer::Ready(CallMatch::ReturnMismatch(selection)))
        } else {
            Ok(Answer::Ready(CallMatch::Selected(selection)))
        }
    }
}

impl BodyState<'_, '_> {
    /// Commit one rejected call node and produce its failed check.
    fn reject_call(
        &mut self,
        node: dir::GlobalNodeIdAny,
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        self.commit_decision(node, Decision::Rejected)?;
        let source = self.commit_error_node(node)?;
        let target = expectation.map_or(source, |expectation| expectation.target);

        Ok(ValueCheck {
            source,
            outcome: CheckOutcome::Fails(CheckFailure::Relation),
            target,
        })
    }

    /// Select the callable meaning of one call node.
    pub(in crate::check) fn select_call(
        &mut self,
        site: FlowSite,
        callee: dir::LocalNodeId<dir::Expression>,
        generic_argument_nodes: &[dir::LocalNodeId<dir::GenericArgument>],
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let node = site.node;
        let module = node.module_id;
        let origin = site.origin();

        // collect explicit type arguments from the call node
        let mut argument_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in generic_argument_nodes {
            let argument = argument.into_global_any(module);
            let ty = self.require_node_type(argument)?;
            argument_types.push(ty);
        }

        // omitted constructor heads use the expected nominal target
        if self.is_inferred_call_head(module, callee) {
            return self.select_inferred_newtype_construct(
                site,
                node,
                origin,
                callee,
                argument_nodes,
                &argument_types,
                expectation,
            );
        }

        let callee_site = self.node_site(callee.into_global_any(module))?;

        // collect callable candidates from the callee
        let Some(callees) = answer!(self.callable_candidates(origin, module, callee_site)?) else {
            // rejected callees already reported their own diagnostic
            return Ok(Answer::Ready(self.reject_call(node, expectation)?));
        };
        // runtime union callees must accept the call through every arm
        if callees.arms.len() > 1 {
            return self.select_call_arms(
                site,
                node,
                origin,
                argument_nodes,
                &callees,
                &argument_types,
                expectation,
            );
        }
        let Some(arm) = callees.arms.first() else {
            return Err(CompilerError::Internal {
                message: "call candidate set contains no runtime arm".to_string(),
            });
        };
        let candidates = &arm.overloads;
        if candidates.is_empty() {
            let callee_type = self.require_node_type(callee_site.node)?;
            let callee_type = answer!(self.flow_type_at(callee_site, callee_type)?);
            self.report_not_callable(origin, callee_type)?;
            return Ok(Answer::Ready(self.reject_call(node, expectation)?));
        }

        // newtype targets select their nominal constructor
        if let [
            CallableCandidate {
                target: CallableTarget::Newtype(symbol),
                ..
            },
        ] = candidates.as_slice()
        {
            // newtype heads read as their declaration reference
            let symbol = *symbol;
            let callee_node = callee.into_global_any(module);
            let reference =
                self.intern_type(module, dir::Type::Reference(dir::TypeReference { symbol }))?;
            self.commit_node_type(callee_node, reference)?;

            return self.select_newtype_construct(
                site,
                node,
                origin,
                symbol,
                argument_nodes,
                &argument_types,
                expectation,
            );
        }

        // select the best overload by memory rank and declaration order
        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;
        let is_single_candidate = candidates.len() == 1;
        let overload = answer!(self.select_overloads(
            origin,
            slice::from_ref(arm),
            &arguments,
            &argument_types,
            expectation,
        )?);

        // confirm the winner outside any probe
        if let Some(candidate) = overload.candidates.first().copied() {
            let attempt =
                self.attempt_call(origin, candidate, &arguments, &argument_types, expectation)?;

            match answer!(attempt) {
                SignatureMatch::Selected(signature) => {
                    let source = answer!(self.commit_callable_signature(
                        node,
                        callee,
                        candidate,
                        argument_nodes,
                        signature,
                    )?);
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(Answer::Ready(ValueCheck {
                        source,
                        outcome: CheckOutcome::Holds,
                        target,
                    }));
                }
                SignatureMatch::ReturnMismatch(signature) => {
                    let source = answer!(self.commit_callable_signature(
                        node,
                        callee,
                        candidate,
                        argument_nodes,
                        signature,
                    )?);
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(Answer::Ready(ValueCheck {
                        source,
                        outcome: CheckOutcome::Fails(CheckFailure::Relation),
                        target,
                    }));
                }
                SignatureMatch::Invalid {
                    selection,
                    rejection,
                } if is_single_candidate => {
                    self.report_signature_rejection(origin, rejection)?;
                    let source = answer!(self.commit_callable_signature(
                        node,
                        callee,
                        candidate,
                        argument_nodes,
                        selection,
                    )?);
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(Answer::Ready(ValueCheck {
                        source,
                        outcome: CheckOutcome::Holds,
                        target,
                    }));
                }
                SignatureMatch::Inapplicable(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    self.report_signature_rejection(origin, rejection)?;
                    return Ok(Answer::Ready(self.reject_call(node, expectation)?));
                }
                SignatureMatch::Invalid { .. } => {}
                SignatureMatch::Inapplicable(_) => {}
            }
        }

        // report that no candidate matched the arguments
        let mut rejections = overload.rejections;
        rejections.truncate(4);
        let arguments = answer!(self.infer_argument_types(site, argument_nodes)?);
        self.report_no_matching_call(origin, &arguments, &rejections)?;
        Ok(Answer::Ready(self.reject_call(node, expectation)?))
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
        callee: dir::LocalNodeId<dir::Expression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let callee = callee.into_global_any(origin.module());
        let Some(expectation) = expectation else {
            self.report_cannot_infer_node(node)?;
            self.commit_error_node(callee)?;

            return Ok(Answer::Ready(self.reject_call(node, None)?));
        };
        let target = answer!(self.reduce_type_head(origin, expectation.target)?);
        let symbol = match self.ty(target)? {
            dir::Type::Application(instance)
                if matches!(self.symbol_kind(instance.symbol), dir::SymbolKind::Newtype) =>
            {
                instance.symbol
            }
            _ => {
                self.report_invalid_inferred_construct_target(origin, target)?;
                self.commit_error_node(callee)?;

                return Ok(Answer::Ready(self.reject_call(node, Some(expectation))?));
            }
        };

        // publish the inferred call head as the selected declaration reference
        let reference = self.intern_type(
            origin.module(),
            dir::Type::Reference(dir::TypeReference { symbol }),
        )?;
        self.commit_node_type(callee, reference)?;

        self.select_newtype_construct(
            site,
            node,
            origin,
            symbol,
            argument_nodes,
            argument_types,
            Some(expectation),
        )
    }

    /// Select one overload for every possible runtime callee.
    fn select_call_arms(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        candidates: &CallCandidates,
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<Answer<ValueCheck>> {
        let arguments =
            self.callable_arguments(origin.module(), argument_nodes, ValueUse::Argument)?;

        // select one jointly viable overload from every runtime arm
        let overload = answer!(self.select_overloads(
            origin,
            &candidates.arms,
            &arguments,
            argument_types,
            expectation,
        )?);
        if overload.candidates.is_empty() {
            let argument_types = answer!(self.infer_argument_types(site, argument_nodes)?);
            let mut rejections = overload.rejections;
            rejections.truncate(4);
            self.report_no_matching_call(origin, &argument_types, &rejections)?;
            return Ok(Answer::Ready(self.reject_call(node, expectation)?));
        }

        // confirm the selected combination outside the ranking probes
        let (selection, outcome) = match answer!(self.attempt_calls(
            origin,
            &overload.candidates,
            &arguments,
            argument_types,
            expectation,
        )?) {
            CallMatch::Selected(selection) => (selection, CheckOutcome::Holds),
            CallMatch::ReturnMismatch(selection) => {
                (selection, CheckOutcome::Fails(CheckFailure::Relation))
            }
            CallMatch::Inapplicable(rejection) => {
                let argument_types = answer!(self.infer_argument_types(site, argument_nodes)?);
                self.report_no_matching_call(origin, &argument_types, &[rejection])?;
                return Ok(Answer::Ready(self.reject_call(node, expectation)?));
            }
        };

        // commit the confirmed calls in runtime-arm order
        for signature in &selection.signatures {
            for (source, coercion) in &signature.coercions {
                self.commit_coercion(*source, coercion.clone())?;
            }
        }
        let mut calls = Vec::with_capacity(overload.candidates.len());
        let mut returns = SmallVec::<[_; 4]>::new();
        for (candidate, signature) in overload.candidates.iter().zip(selection.signatures) {
            let call =
                self.call_resolution(origin.module(), candidate, argument_nodes, &signature)?;
            returns.push(call.return_type);
            calls.push(call);
        }
        let return_type = match returns.as_slice() {
            [single] => *single,
            _ => self.normalized_union_type(origin.module(), returns)?,
        };
        let resolution = dir::CallResolution::Union {
            arms: calls,
            ty: return_type,
        };
        self.commit_decision(node, Decision::Call(resolution))?;

        self.commit_node_type(node, return_type)?;

        let target = expectation.map_or(return_type, |expectation| expectation.target);

        Ok(Answer::Ready(ValueCheck {
            source: return_type,
            outcome,
            target,
        }))
    }
}

impl BodyState<'_, '_> {
    /// Commit one selected callable signature.
    fn commit_callable_signature(
        &mut self,
        node: dir::GlobalNodeIdAny,
        callee: dir::LocalNodeId<dir::Expression>,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: SignatureSelection,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // commit conversions only after the candidate has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }

        match &candidate.target {
            CallableTarget::Expression | CallableTarget::Symbol(_) => {
                self.commit_call_signature(node, callee, candidate, argument_nodes, signature)
            }
            CallableTarget::Variant(case) => {
                self.commit_variant_signature(node, argument_nodes, signature, case)
            }
            CallableTarget::Newtype(symbol) => Err(CompilerError::Internal {
                message: format!("newtype {symbol:?} reached signature selection"),
            }),
        }
    }

    /// Commit one selected ordinary call signature.
    fn commit_call_signature(
        &mut self,
        node: dir::GlobalNodeIdAny,
        callee: dir::LocalNodeId<dir::Expression>,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: SignatureSelection,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        // bare declaration heads type as the selected overload
        if matches!(&candidate.target, CallableTarget::Symbol(_)) && candidate.receiver.is_none() {
            let callee = callee.into_global_any(node.module_id);
            self.commit_node_type(callee, signature.callable)?;
        }

        let call = self.call_resolution(node.module_id, candidate, argument_nodes, &signature)?;
        let resolution = dir::OperationResolution::One(call);

        self.commit_call_selection(node, resolution)
    }

    /// Build one expression or symbol call resolution.
    fn call_resolution(
        &mut self,
        module: ModuleId,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: &SignatureSelection,
    ) -> CompilerResult<dir::Call> {
        let arguments = self.argument_bindings(module, argument_nodes, &signature.parameters);
        let return_type =
            self.select_call_return_type(module, candidate, argument_nodes, signature.return_type)?;
        let target = match &candidate.target {
            CallableTarget::Expression => dir::CallTarget::Expression {
                generic_arguments: signature.generic_arguments.clone(),
            },
            CallableTarget::Symbol(symbol) => match candidate.selected_receiver(signature) {
                Some(dir::MemberReceiver::Direct(receiver)) => dir::CallTarget::Symbol {
                    function: candidate.function_target(signature, Some(receiver))?,
                    dispatch: dir::FunctionDispatch::Direct,
                },
                Some(dir::MemberReceiver::Dynamic(dispatch)) => dir::CallTarget::Dynamic {
                    dispatch,
                    function: dir::DynamicFunction::Symbol(*symbol),
                    generic_arguments: signature.generic_arguments.clone(),
                },
                None => dir::CallTarget::Symbol {
                    function: candidate.function_target(signature, None)?,
                    dispatch: dir::FunctionDispatch::Direct,
                },
            },
            CallableTarget::Newtype(_) | CallableTarget::Variant(_) => {
                return Err(CompilerError::Internal {
                    message: "constructor target reached call resolution".to_string(),
                });
            }
        };
        let resolution = dir::Call {
            target,
            callable_type: signature.callable,
            arguments,
            return_type,
        };

        Ok(resolution)
    }

    /// Select the exact result type of one accepted call.
    fn select_call_return_type(
        &mut self,
        module: ModuleId,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        declared: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let CallableTarget::Symbol(symbol) = &candidate.target else {
            return Ok(declared);
        };
        if self.language_item(*symbol)? != Some(dir::LanguageItem::SymbolFor) {
            return Ok(declared);
        }

        // known registry strings select their singleton symbol type
        let [argument] = argument_nodes else {
            return Ok(declared);
        };
        let Some(value) = self.argument_expression(module, *argument) else {
            return Ok(declared);
        };
        let ty = self.require_node_type(value)?;
        let Some(dir::StaticKey::Name(name)) = self.static_key_from_type(ty)? else {
            return Ok(declared);
        };
        let key = dir::StaticKey::Symbol(dir::SymbolKey::Registry(name));

        self.intern_type(module, dir::Type::Key(key))
    }

    /// Commit one selected tagged variant constructor signature.
    fn commit_variant_signature(
        &mut self,
        node: dir::GlobalNodeIdAny,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: SignatureSelection,
        case: &dir::VariantCase,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let generic_arguments = signature.generic_arguments.clone();
        let return_variant = match self.ty(signature.return_type)? {
            dir::Type::Variant(variant) if variant.variant == case.variant => variant,
            ty => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "tagged case {:?} selected non-variant return type {ty:?}",
                        case.variant,
                    ),
                });
            }
        };
        let (variant, discriminator) = match self.definition(case.owner)? {
            Some(dir::Definition::Newtype(definition)) => {
                let Some(variant) = definition.tagged_variant_by_symbol(case.variant).cloned()
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "tagged case {:?} is absent from owner {:?}",
                            case.variant, case.owner,
                        ),
                    });
                };
                let Some(discriminator) = definition.discriminator else {
                    return Err(CompilerError::Internal {
                        message: format!("tagged owner {:?} has no discriminator", case.owner),
                    });
                };

                (variant, discriminator)
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("tagged case owner {:?} is not a newtype", case.owner),
                });
            }
        };
        let substitution = TypeSubstitution::default()
            .with_carried(&generic_arguments)?
            .with_receiver(return_variant.owner);
        let backing = self.substitute_type(node.module_id, variant.backing, &substitution)?;
        let argument = variant
            .argument
            .map(|argument| self.substitute_type(node.module_id, argument, &substitution))
            .transpose()?;
        let target = dir::ConstructTarget::Variant(dir::VariantConstructCandidate {
            case: case.clone(),
            generic_arguments,
            backing,
            argument,
            discriminator,
            discriminant: dir::ScalarLiteral::String(variant.discriminant),
        });
        let arguments =
            self.argument_bindings(node.module_id, argument_nodes, &signature.parameters);
        let resolution = dir::ConstructResolution::new(target, arguments, signature.return_type);

        self.commit_decision(node, Decision::Construct(resolution))?;
        self.commit_node_type(node, signature.return_type)?;

        Ok(Answer::Ready(signature.return_type))
    }

    /// Commit one accepted call selection.
    fn commit_call_selection(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::CallResolution,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let return_type = resolution.return_type();
        self.commit_decision(node, Decision::Call(resolution))?;
        self.commit_node_type(node, return_type)?;

        Ok(Answer::Ready(return_type))
    }
}
