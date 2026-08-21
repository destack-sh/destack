use std::slice;
use std::sync::Arc;

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Answer, BodyState, CallableArgument, Callee, CheckFailure, CheckOutcome, Expectation, FlowSite,
    InferMode, Origin, PlaceUse, REPORTED_REJECTIONS, Selected, SignatureFamily, SignatureInstance,
    SignatureMatch, SignatureSelection, Value, ValueCheck, ValueUse, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// The operation performed by one callable candidate.
#[derive(Debug)]
enum CallableTarget {
    /// Call a function-typed runtime value.
    Expression,
    /// Call one declaration-backed function.
    Symbol(dir::GlobalSymbolId),
    /// Call through one erased interface call signature.
    CallSignature {
        /// The signature's source declaration.
        source: dir::GlobalNodeIdAny,
        /// The erased receiver value type.
        receiver: dir::GlobalTypeId,
        /// The interface constraint declaring the signature.
        constraint: dir::GlobalTypeId,
    },
    /// Construct one newtype.
    Newtype(dir::GlobalSymbolId),
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
    /// The member space that selected this candidate, for member callees.
    member_space: Option<dir::MemberSpace>,
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
            selection: dir::Selection::new(*symbol, signature.generic_arguments.clone()),
        })
    }
}

/// One runtime callee and its declaration alternatives.
struct CallableArm {
    /// The overloads declared for this runtime callee.
    overloads: SmallVec<[CallableCandidate; 2]>,
}

/// The overload combination selected across every runtime arm.
struct OverloadSelection<'candidate> {
    /// The selected callable with its declared position, for each runtime arm.
    candidates: SmallVec<[(usize, &'candidate CallableCandidate); 2]>,
    /// The rejected candidate descriptions.
    rejections: Vec<String>,
}

/// Result of matching one call across every possible runtime callee.
enum CallMatch {
    /// Every runtime callee accepts the invocation and its expected result.
    Selected(SmallVec<[SignatureSelection; 2]>),
    /// Every runtime callee accepts the invocation and refuses its expected result.
    ReturnMismatch(SmallVec<[SignatureSelection; 2]>),
    /// At least one runtime callee rejects the invocation.
    Inapplicable(String),
}

impl BodyState<'_, '_> {
    /// Collect callable candidates in declaration order from one callee node.
    fn callable_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        callee_site: FlowSite,
        is_optional: bool,
    ) -> CompilerResult<Option<SmallVec<[CallableArm; 2]>>> {
        let callee_node = callee_site.node;
        let callee = callee_node.into_typed::<dir::Expression>().local_id;

        // use the recorded callee decision for reference and member callees
        let is_decided_callee = {
            let view = self.module(module).view();
            let expression = view.get(callee);

            expression.is_reference() || matches!(expression, dir::Expression::Member { .. })
        };

        // call every other callee through its function-typed value
        if !is_decided_callee {
            return self.value_callable_candidates(origin, callee_site, is_optional);
        }

        // resolve declaration callees through their recorded name decision
        if let Some(resolution) = self.name_decision(callee_node).cloned() {
            let symbols = resolution
                .symbols()
                .iter()
                .copied()
                .collect::<SmallVec<[_; 2]>>();

            // call a value binding through its type, a declaration through its overloads
            let mut is_value_binding = true;
            for symbol in &symbols {
                is_value_binding &= self
                    .symbol_kind_maybe(*symbol)?
                    .is_some_and(dir::SymbolKind::is_binding);
            }
            if is_value_binding {
                return self.value_callable_candidates(origin, callee_site, is_optional);
            }

            // build one overload candidate per declared symbol
            let mut candidates = SmallVec::new();
            for symbol in symbols {
                // skip declarations removed by statically false gates
                if self.is_absent_symbol(symbol) {
                    continue;
                }

                // a newtype head constructs, every other declaration calls
                let ty = self.symbol_type(symbol)?;
                let is_newtype = matches!(
                    self.symbol_kind_maybe(symbol)?,
                    Some(dir::SymbolKind::Newtype)
                );
                let target = if is_newtype {
                    CallableTarget::Newtype(symbol)
                } else {
                    CallableTarget::Symbol(symbol)
                };

                // keep a declaration only while its type is invocable
                let ty = match &target {
                    CallableTarget::Newtype(_) => ty,
                    CallableTarget::Expression
                    | CallableTarget::Symbol(_)
                    | CallableTarget::CallSignature { .. } => {
                        let Some(ty) = self.callable_type(ty)? else {
                            continue;
                        };

                        ty
                    }
                };

                candidates.push(CallableCandidate {
                    target,
                    generic_scope: None,
                    receiver: None,
                    member_space: None,
                    ty,
                    generic_arguments: Vec::new(),
                });
            }

            // declaration callees name exactly one runtime callee
            let mut arms = SmallVec::new();
            arms.push(CallableArm {
                overloads: candidates,
            });

            return Ok(Some(arms));
        }

        // fall back to the callee's own decision
        match self.decision(callee_node).cloned() {
            // build candidates from the recorded member decision
            Some(dir::Decision::Member(_)) => {
                let callee_expression = self.module(module).view().get(callee).clone();
                let dir::Expression::Member { left, .. } = callee_expression else {
                    return Err(CompilerError::Internal {
                        message: format!("member call callee {callee_node:?} is not a member"),
                    });
                };
                let receiver_site = self.visit_site(left.into_global_any(module))?;
                let receiver_type = self.infer_node_type(receiver_site, PlaceUse::Read)?;
                let receiver = self.expression_value(receiver_site, receiver_type)?;
                let Some(resolution) = self
                    .decisions(callee_node.module_id)
                    .member_decision(callee_node)
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
                        self.member_access_candidates(origin, receiver, access, is_optional)?
                    }
                    dir::OperationResolution::Union { arms, .. } => {
                        let mut runtime_arms = SmallVec::with_capacity(arms.len());
                        for access in arms {
                            let candidates = self.member_access_candidates(
                                origin,
                                receiver,
                                access,
                                is_optional,
                            )?;
                            runtime_arms.extend(candidates);
                        }

                        runtime_arms
                    }
                };

                Ok(Some(candidates))
            }
            // skip a callee that already reported
            Some(dir::Decision::Rejected | dir::Decision::Poisoned) => Ok(None),
            // fail loudly on a callee decided as anything else
            Some(other) => Err(CompilerError::Internal {
                message: format!("call callee {callee_node:?} decided as {other:?}"),
            }),
            // resolve or check an undecided callee in place, then re-dispatch
            None => {
                // decide any lexical or qualified reference before reading its value
                if self.decide_reference(callee_node)?.is_some() {
                    return self.callable_candidates(origin, module, callee_site, is_optional);
                }

                // decide a member callee by checking it in place, else call its value
                self.infer_node(callee_site, PlaceUse::Read, InferMode::Regular)?;
                match self.decision(callee_node) {
                    Some(_) => self.callable_candidates(origin, module, callee_site, is_optional),
                    None => self.value_callable_candidates(origin, callee_site, is_optional),
                }
            }
        }
    }

    /// Build callable candidates from one singular member access.
    fn member_access_candidates(
        &mut self,
        origin: Origin,
        receiver: Value,
        access: &dir::MemberAccess,
        is_optional: bool,
    ) -> CompilerResult<SmallVec<[CallableArm; 2]>> {
        let ty = self.select_chain_operand(origin, access.ty, is_optional)?;
        if let Some(arm) = self.member_target_candidates(receiver, &access.target)? {
            let mut arms = SmallVec::new();
            arms.push(arm);

            return Ok(arms);
        }

        // stored and computed members call through their selected value
        let arms = self.callable_value_arms(origin, ty)?;

        Ok(arms)
    }

    /// Build callable candidates from one singular member target.
    fn member_target_candidates(
        &mut self,
        receiver: Value,
        target: &dir::MemberTarget,
    ) -> CompilerResult<Option<CallableArm>> {
        match target {
            dir::MemberTarget::Symbol(candidate) => {
                let mut candidates = SmallVec::new();
                if let Some(candidate) = self.member_candidate(receiver, candidate)? {
                    candidates.push(candidate);
                }

                Ok(Some(CallableArm {
                    overloads: candidates,
                }))
            }
            dir::MemberTarget::OverloadSet(targets) => {
                let mut candidates = SmallVec::new();
                for target in targets {
                    let dir::MemberTarget::Symbol(candidate) = target else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "member overload set contains a non-symbol target: {targets:?}"
                            ),
                        });
                    };
                    if let Some(candidate) = self.member_candidate(receiver, candidate)? {
                        candidates.push(candidate);
                    }
                }

                Ok(Some(CallableArm {
                    overloads: candidates,
                }))
            }
            dir::MemberTarget::Intersection(targets) => {
                let mut candidates = SmallVec::new();
                for target in targets {
                    let Some(selected) = self.member_target_candidates(receiver, target)? else {
                        continue;
                    };
                    candidates.extend(selected.overloads);
                }

                Ok(Some(CallableArm {
                    overloads: candidates,
                }))
            }
            dir::MemberTarget::Projection { .. }
            | dir::MemberTarget::Field(_)
            | dir::MemberTarget::Call(_)
            | dir::MemberTarget::Index(_) => Ok(None),
        }
    }

    /// Build one callable candidate from a declaration-backed member target.
    fn member_candidate(
        &mut self,
        receiver: Value,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<Option<CallableCandidate>> {
        let Some(ty) = candidate.callable_type else {
            return Ok(None);
        };
        let target = self.member_operation(candidate)?;
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
            member_space: Some(candidate.space),
            ty,
            generic_arguments: candidate.selection.arguments.clone(),
        };

        Ok(Some(candidate))
    }

    /// Return the operation selected by one declaration-backed member.
    fn member_operation(
        &mut self,
        candidate: &dir::MemberCandidate,
    ) -> CompilerResult<CallableTarget> {
        match self.symbol_kind(candidate.selection.symbol)? {
            dir::SymbolKind::AssociatedConst => Ok(CallableTarget::Expression),
            dir::SymbolKind::Function => Ok(CallableTarget::Symbol(candidate.selection.symbol)),
            kind => Err(CompilerError::Internal {
                message: format!(
                    "callable member {:?} has non-callable symbol kind {kind:?}",
                    candidate.selection.symbol
                ),
            }),
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

    /// Return the invocable form of one type.
    fn callable_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let callable = matches!(
            self.ty(ty)?,
            dir::Type::FunctionSignature(_)
                | dir::Type::Function(_)
                | dir::Type::FunctionPointer(_)
        )
        .then_some(ty);

        Ok(callable)
    }

    /// Collect the callable candidate behind one function-typed callee value.
    fn value_callable_candidates(
        &mut self,
        origin: Origin,
        callee: FlowSite,
        is_optional: bool,
    ) -> CompilerResult<Option<SmallVec<[CallableArm; 2]>>> {
        let ty = self.infer_node_type(callee, PlaceUse::Read)?;
        let ty = self.strip_form(origin, ty)?;
        let ty = self.select_chain_operand(origin, ty, is_optional)?;
        let arms = self.callable_value_arms(origin, ty)?;

        Ok(Some(arms))
    }

    /// Collect runtime callable arms from one value type.
    fn callable_value_arms(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[CallableArm; 2]>> {
        // distribute runtime union alternatives into independent arms
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements: SmallVec<[_; 8]> = self.type_ids(ty.module_id, union.elements)?.into();
            let mut arms = SmallVec::with_capacity(elements.len());
            for element in elements {
                let nested = self.callable_value_arms(origin, element)?;
                arms.extend(nested);
            }

            return Ok(arms);
        }

        // intersections contribute overload alternatives to one runtime value
        let overloads = self.callable_value_overloads(origin, ty)?;
        let mut arms = SmallVec::new();
        arms.push(CallableArm { overloads });

        Ok(arms)
    }

    /// Collect overload alternatives from one runtime callable value.
    fn callable_value_overloads(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[CallableCandidate; 2]>> {
        // read the arm's reduced callable head
        let ty = self.normalize(origin, ty)?;

        // flatten intersection signatures into one declaration alternative set
        if let dir::Type::Intersection(intersection) = self.ty(ty)? {
            let elements: SmallVec<[_; 8]> =
                self.type_ids(ty.module_id, intersection.elements)?.into();
            let mut overloads = SmallVec::with_capacity(elements.len());
            for element in elements {
                let nested = self.callable_value_overloads(origin, element)?;
                overloads.extend(nested);
            }

            return Ok(overloads);
        }

        // call erased values through their apparent constraint signatures
        if let Some(constraint) = self.erased_constraint(ty)? {
            let signatures = self.apparent_signatures(constraint, SignatureFamily::Call)?;
            let mut overloads = SmallVec::with_capacity(signatures.len());
            for signature in signatures {
                overloads.push(CallableCandidate {
                    target: CallableTarget::CallSignature {
                        source: signature.source,
                        receiver: ty,
                        constraint,
                    },
                    generic_scope: None,
                    receiver: None,
                    member_space: None,
                    ty: signature.ty,
                    generic_arguments: Vec::new(),
                });
            }

            return Ok(overloads);
        }

        // retain one candidate for an invocable value representation
        let mut overloads = SmallVec::new();
        if let Some(ty) = self.callable_type(ty)? {
            overloads.push(CallableCandidate {
                target: CallableTarget::Expression,
                generic_scope: None,
                receiver: None,
                member_space: None,
                ty,
                generic_arguments: Vec::new(),
            });
        }

        Ok(overloads)
    }

    /// Attempt one callable candidate against collected arguments.
    fn attempt_call(
        &mut self,
        origin: Origin,
        candidate: &CallableCandidate,
        arguments: &[CallableArgument],
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<SignatureMatch> {
        let matched = self.attempt_callable(
            origin,
            candidate.ty,
            candidate.generic_scope,
            candidate.receiver.as_ref().map(|receiver| receiver.value),
            &candidate.generic_arguments,
            argument_types,
            arguments,
            expectation,
        )?;

        Ok(matched)
    }

    /// Select the first applicable overload from every runtime arm.
    fn select_overloads<'candidate>(
        &mut self,
        origin: Origin,
        arms: &'candidate [CallableArm],
        arguments: &[CallableArgument],
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<OverloadSelection<'candidate>> {
        // leave the call unselectable when an arm declares nothing
        if arms.iter().any(|arm| arm.overloads.is_empty()) {
            return Ok(OverloadSelection {
                candidates: SmallVec::new(),
                rejections: Vec::new(),
            });
        }

        // skip the probe when every arm has one declaration
        if arms.iter().all(|arm| arm.overloads.len() == 1) {
            let candidates = arms
                .iter()
                .map(|arm| (0, &arm.overloads[0]))
                .collect::<SmallVec<[_; 2]>>();

            return Ok(OverloadSelection {
                candidates,
                rejections: Vec::new(),
            });
        }

        // select the first viable overload per arm, else the first undecided one
        let mut candidates = SmallVec::with_capacity(arms.len());
        let mut rejections = Vec::new();
        for arm in arms {
            let mut selected = None;
            let mut undecided = None;
            rejections.clear();
            for (position, candidate) in arm.overloads.iter().enumerate() {
                let (verdict, rejection) = self.probe_candidate_describing(
                    |state| {
                        let matched = state.attempt_call(
                            origin,
                            candidate,
                            arguments,
                            argument_types,
                            expectation,
                        )?;

                        Ok(matched.into_candidate())
                    },
                    |state, rejection| {
                        state.check.describe_signature_rejection(
                            origin.module(),
                            candidate.ty,
                            rejection,
                        )
                    },
                )?;
                match verdict {
                    // keep a refused overload's description for the report
                    Verdict::Fails => rejections.extend(rejection),
                    // strict declaration order: the first viable overload wins
                    Verdict::Holds => {
                        selected = Some((position, candidate));

                        break;
                    }
                    // hold the first undecided overload behind the viable ones
                    Verdict::Ambiguous => {
                        undecided.get_or_insert((position, candidate));
                    }
                }
            }

            // leave the whole call unselectable once one arm refuses
            let Some(selected) = selected.or(undecided) else {
                candidates.clear();

                break;
            };
            candidates.push(selected);
        }

        Ok(OverloadSelection {
            candidates,
            rejections,
        })
    }

    /// Apply one decided selection at a call site, converting each argument.
    pub(super) fn apply_signature_instance(
        &mut self,
        origin: Origin,
        selection: SignatureSelection,
        arguments: &[CallableArgument],
    ) -> CompilerResult<Option<SignatureSelection>> {
        // convert each argument against the decided parameter list
        let mut signature = selection;
        for (index, argument) in arguments.iter().copied().enumerate() {
            let parameter = signature
                .parameters
                .get(index)
                .or_else(|| signature.parameters.last());
            let Some(parameter) = parameter else {
                return Ok(None);
            };
            let parameter_type = parameter.argument_type;
            let conversion =
                self.match_signature_argument(origin, index, argument, parameter_type)?;
            match conversion {
                Ok(Some(coercion)) => signature.coercions.push((argument.source, coercion)),
                Ok(None) => {}
                Err(_) => return Ok(None),
            }
        }

        Ok(Some(signature))
    }

    /// Attempt every selected runtime arm as one inference transaction.
    fn attempt_call_arms(
        &mut self,
        origin: Origin,
        candidates: &[(usize, &CallableCandidate)],
        arguments: &[CallableArgument],
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<CallMatch> {
        // require every runtime arm to accept the call
        let mut signatures = SmallVec::with_capacity(candidates.len());
        let mut coercions = SmallVec::<[(dir::GlobalNodeIdAny, dir::Coercion); 4]>::new();
        let mut has_return_mismatch = false;
        for (_, candidate) in candidates {
            let matched =
                self.attempt_call(origin, candidate, arguments, argument_types, expectation)?;
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

                    return Ok(CallMatch::Inapplicable(rejection));
                }
            };

            // require one uniform argument conversion across every runtime arm
            for (node, coercion) in &signature.coercions {
                match coercions.iter().find(|(source, _)| source == node) {
                    Some((_, selected)) if selected != coercion => {
                        return Ok(CallMatch::Inapplicable(
                            "runtime call arms require incompatible argument conversions"
                                .to_string(),
                        ));
                    }
                    Some(_) => {}
                    None => coercions.push((*node, coercion.clone())),
                }
            }
            signatures.push(signature);
        }

        // hold the combination back on a mismatched return, else accept every arm
        if has_return_mismatch {
            Ok(CallMatch::ReturnMismatch(signatures))
        } else {
            Ok(CallMatch::Selected(signatures))
        }
    }

    /// Build one retained call attempt from an unmatched candidate.
    fn attempted_call(
        &mut self,
        origin: Origin,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Option<dir::Call>> {
        // name the callable the candidate reached for
        let target = match &candidate.target {
            CallableTarget::Symbol(symbol) => dir::CallableTarget::Symbol {
                function: dir::FunctionTarget {
                    receiver: None,
                    generic_scope: candidate.generic_scope,
                    selection: dir::Selection::new(*symbol, candidate.generic_arguments.clone()),
                },
                dispatch: dir::FunctionDispatch::Direct,
            },
            CallableTarget::Expression => dir::CallableTarget::Expression {
                generic_arguments: Vec::new(),
            },
            _ => return Ok(None),
        };

        // bind the authored arguments against the declared parameters the candidate reached for
        let signature = self.callable_signature_type(origin, candidate.ty)?;
        let (arguments, return_type) = match signature {
            // bind against the parameters the signature declares
            Some((signature_type, signature)) => {
                let parameters = self
                    .signature_parameters(signature_type.module_id, signature.parameters)?
                    .to_vec();
                let arguments =
                    self.argument_bindings(origin, origin.module(), argument_nodes, &parameters)?;
                let return_type = match signature.return_type {
                    Some(return_type) => return_type,
                    None => self.intern_type(dir::Type::Error)?,
                };

                (arguments, return_type)
            }
            // leave a callable without a signature unbound
            None => (Vec::new(), self.intern_type(dir::Type::Error)?),
        };

        Ok(Some(dir::Call {
            target,
            callable_type: candidate.ty,
            arguments,
            return_type,
        }))
    }

    /// Commit one rejected call node, retaining its best attempt when any.
    pub(in crate::sema) fn reject_call(
        &mut self,
        node: dir::GlobalNodeIdAny,
        expectation: Option<Expectation>,
        attempt: Option<dir::Call>,
    ) -> CompilerResult<ValueCheck> {
        // retain the call the node reached for, or commit a bare rejection
        match attempt {
            Some(attempt) => {
                self.commit_decision(node, dir::Decision::Attempted(Box::new(attempt)))?
            }
            None => self.commit_decision(node, dir::Decision::Rejected)?,
        }

        // fail the node against its expectation, or against itself
        let source = self.commit_error_node(node)?;
        let target = expectation.map_or(source, |expectation| expectation.target);

        Ok(ValueCheck {
            source,
            stored: source,
            outcome: CheckOutcome::Fails(CheckFailure::Relation),
            target,
        })
    }

    /// Poison one call whose operand already reported an error.
    pub(in crate::sema) fn poison_call(
        &mut self,
        node: dir::GlobalNodeIdAny,
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        let source = self.poison_node(node)?;
        let target = expectation.map_or(source, |expectation| expectation.target);

        Ok(ValueCheck {
            source,
            stored: source,
            outcome: CheckOutcome::Fails(CheckFailure::Relation),
            target,
        })
    }

    /// Defer one call whose callee value has not settled yet.
    fn defer_call_selection(
        &mut self,
        site: FlowSite,
        expectation: Option<Expectation>,
        stalled_on: dir::TypeVariableId,
    ) -> CompilerResult<ValueCheck> {
        let node = site.node;
        self.defer_selection(site, stalled_on)?;
        let source = self.require_node_type(node)?;
        let target = expectation.map_or(source, |expectation| expectation.target);

        Ok(ValueCheck {
            source,
            stored: source,
            outcome: CheckOutcome::Holds,
            target,
        })
    }

    /// Select the callable meaning of one call node.
    pub(in crate::sema) fn select_call(
        &mut self,
        site: FlowSite,
        callee: dir::LocalNodeId<dir::Expression>,
        generic_argument_nodes: &[dir::LocalNodeId<dir::GenericArgument>],
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        is_optional: bool,
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        let node = site.node;
        let module = node.module_id;
        let origin = site.origin();

        // walk the written generic arguments of the call node
        self.walk_body_generic_arguments(module, generic_argument_nodes)?;

        // walk the argument decorators, keeping the statically present arguments
        let argument_nodes = self.walk_body_arguments(module, argument_nodes)?;
        let argument_nodes = argument_nodes.as_slice();

        // register function-valued arguments before any candidate probe
        self.register_argument_function_values(module, argument_nodes)?;

        // decide identifier and receiver arguments ahead of candidate probes
        for argument in argument_nodes {
            if let Some(value) = self.module(module).view().get(*argument).value() {
                let value_node = value.into_global_any(module);
                self.decide_reference(value_node)?;
                if matches!(self.module(module).view().get(value), dir::Expression::This)
                    && self.check.decision(value_node).is_none()
                {
                    self.check
                        .commit_active_receiver_decision(value_node, dir::ReceiverKind::This)?;
                }
            }
        }

        // collect the written generic argument types
        let mut argument_types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in generic_argument_nodes {
            let argument = argument.into_global_any(module);
            let ty = self.require_node_type(argument)?;
            argument_types.push(ty);
        }

        // select the base class constructor for a super callee
        if matches!(
            self.module(module).view().get(callee),
            dir::Expression::Super
        ) {
            return self.select_super_construct(site, callee, argument_nodes);
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

        let callee_site = self.visit_site(callee.into_global_any(module))?;

        // collect callable candidates from the callee
        let Some(callees) = self.callable_candidates(origin, module, callee_site, is_optional)?
        else {
            // rejected callees already reported their own diagnostic
            return self.reject_call(node, expectation, None);
        };

        // decide the callee reference before selecting its call
        self.decide_reference(callee.into_global_any(module))?;

        // runtime union callees must accept the call through every arm
        if callees.len() > 1 {
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
        let Some(arm) = callees.first() else {
            return Err(CompilerError::Internal {
                message: "call candidate set contains no runtime arm".to_string(),
            });
        };

        // poison, defer, or reject a callee that exposes no callable
        let candidates = &arm.overloads;
        if candidates.is_empty() {
            let callee_type = self.require_node_type(callee_site.node)?;
            let callee_type = self.flow_type_at(callee_site, callee_type)?;

            // poison a callee that already reported an error
            if self.any_error_operand(&[callee_type])? {
                return self.poison_call(node, expectation);
            }

            // defer an unknown callee head until its value solves
            if let Some(stalled_on) = self.check.root_variable(callee_type)? {
                return self.defer_call_selection(site, expectation, stalled_on);
            }
            self.report_not_callable(origin, callee_type)?;

            return self.reject_call(node, expectation, None);
        }

        // select the nominal constructor for a newtype target
        if let [
            CallableCandidate {
                target: CallableTarget::Newtype(symbol),
                ..
            },
        ] = candidates.as_slice()
        {
            // read a newtype head as its declaration reference
            let symbol = *symbol;
            let callee_node = callee.into_global_any(module);
            let reference =
                self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
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

        // read the call's argument values
        let arguments = self.callable_arguments(module, argument_nodes, ValueUse::Argument)?;

        // ask the argument-blind and the argument-committed selection question
        let expected = expectation.map(|expectation| expectation.target);
        let first = candidates.first();
        let (blind, question) = match first.map(|first| &first.target) {
            Some(CallableTarget::Symbol(symbol)) => {
                let Some(first) = first else {
                    return Err(CompilerError::Internal {
                        message: "a symbol callee lost its first overload".to_string(),
                    });
                };

                // ask blind over the callee, its written arguments, and the argument types
                let mut operands = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                operands.push(first.ty);
                operands.extend(dir::GenericArgumentBinding::values(
                    &first.generic_arguments,
                ));
                operands.extend(argument_types.iter().copied());
                let blind = self.check.selection_question(
                    origin,
                    Callee::Symbol(*symbol),
                    expected,
                    &operands,
                )?;

                // ask again over the argument types this site has committed
                let committed = arguments
                    .iter()
                    .map(|argument| self.committed_node_type(argument.source))
                    .collect::<Option<SmallVec<[dir::GlobalTypeId; 8]>>>();
                let question = match committed {
                    Some(committed) => {
                        operands.extend(committed);
                        self.check.selection_question(
                            origin,
                            Callee::Symbol(*symbol),
                            expected,
                            &operands,
                        )?
                    }
                    None => None,
                };

                (blind, question)
            }
            // value, newtype, and erased callees ask no canonical question
            Some(
                CallableTarget::Expression
                | CallableTarget::CallSignature { .. }
                | CallableTarget::Newtype(_),
            )
            | None => (None, None),
        };

        // replay the decided answer at this site's live roots, preferring the committed one
        let replayed = [&question, &blind].into_iter().find_map(|asked| {
            let (asked, canonical) = asked.as_ref()?;
            match self.check.answers.get(asked) {
                Some(Answer::Selection(response)) => {
                    Some((response.clone(), Arc::clone(canonical)))
                }
                _ => None,
            }
        });
        if let Some((response, canonical)) = replayed {
            let mark = self.check.infer.mark(&self.check.fulfill);
            let replayed = match self
                .check
                .instantiate_response(origin, &canonical, &response)?
            {
                Selected::Callable(mut instance) if instance.overload < candidates.len() => {
                    // re-relate the receiver, adopting this site's projection steps
                    let callable = match self.check.ty(instance.selection.callable)? {
                        dir::Type::Function(function) => {
                            self.check.signature_head(function.signature)?
                        }
                        _ => self.check.signature_head(instance.selection.callable)?,
                    };
                    let is_accepted = match (
                        &candidates[instance.overload].receiver,
                        callable.and_then(|callable| callable.this_parameter),
                    ) {
                        // a declared this must accept this site's receiver
                        (Some(receiver), Some(this_parameter)) => {
                            match self.constrain_receiver(origin, receiver.value, this_parameter)? {
                                Some(steps) => {
                                    instance.selection.receiver_steps = Some(steps);

                                    true
                                }
                                None => false,
                            }
                        }
                        // a call without a declared this replays verbatim
                        (Some(_), None) | (None, _) => true,
                    };
                    if is_accepted {
                        self.apply_signature_instance(origin, instance.selection, &arguments)?
                            .map(|signature| (instance.overload, signature))
                    } else {
                        None
                    }
                }
                // every other stored answer re-derives at this site
                Selected::Callable(_)
                | Selected::Newtype(_)
                | Selected::Protocol(..)
                | Selected::Builtin { .. }
                | Selected::Rejected => None,
            };
            match replayed {
                Some((overload, signature)) => {
                    self.check.infer.commit(mark);
                    let source = self.commit_callable_signature(
                        node,
                        callee,
                        &candidates[overload],
                        argument_nodes,
                        signature,
                    )?;
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(ValueCheck {
                        source,
                        stored: source,
                        outcome: CheckOutcome::Holds,
                        target,
                    });
                }
                None => {
                    let poison = self.check.intern_type(dir::Type::Error)?;
                    self.check
                        .infer
                        .rollback(mark, poison, &mut self.check.fulfill)?;
                }
            }
        }

        // the decision starts here; its queued checks feed the stored response
        let checks_before = self.check.fulfill.checks.count();

        // select the first applicable overload in declaration order
        let is_single_candidate = candidates.len() == 1;
        let overload = self.select_overloads(
            origin,
            slice::from_ref(arm),
            &arguments,
            &argument_types,
            expectation,
        )?;

        // confirm the selected declaration outside any probe
        if let Some((position, candidate)) = overload.candidates.first().copied() {
            let mark = self.check.infer.mark(&self.check.fulfill);
            let attempt =
                self.attempt_call(origin, candidate, &arguments, &argument_types, expectation)?;
            match &attempt {
                // keep the inference an accepted call bound
                SignatureMatch::Selected(_) => self.check.infer.commit(mark),
                // keep what a sole candidate bound, reporting in place
                SignatureMatch::Invalid { .. } | SignatureMatch::Inapplicable(_)
                    if is_single_candidate =>
                {
                    self.check.infer.commit(mark)
                }
                // roll back everything a refused overload opened
                SignatureMatch::ReturnMismatch(_)
                | SignatureMatch::Invalid { .. }
                | SignatureMatch::Inapplicable(_) => {
                    let poison = self.check.intern_type(dir::Type::Error)?;
                    self.check
                        .infer
                        .rollback(mark, poison, &mut self.check.fulfill)?;
                }
            }

            match attempt {
                // commit the accepted call and remember its decision
                SignatureMatch::Selected(signature) => {
                    // fold the decision canonical over both asks
                    let mut stored = signature.clone();
                    stored.coercions = SmallVec::new();
                    stored.receiver_steps = None;
                    let value = Selected::Callable(SignatureInstance {
                        overload: position,
                        selection: stored,
                    });

                    // serve argument-independent answers under the blind question
                    if let Some((asked, canonical)) = &blind
                        && !self.check.answers.contains_key(asked)
                        && let Some(response) = self.check.canonicalize_response(
                            canonical,
                            checks_before,
                            value.clone(),
                        )?
                        && response.is_exact()
                    {
                        self.check
                            .answers
                            .insert(asked.clone(), Answer::Selection(Arc::new(response)));
                    }

                    // serve open generic answers under the argument-committed question
                    if let Some((asked, canonical)) = &question {
                        self.check.remember_answer(
                            asked,
                            canonical,
                            checks_before,
                            value,
                            Answer::Selection,
                        )?;
                    }

                    let source = self.commit_callable_signature(
                        node,
                        callee,
                        candidate,
                        argument_nodes,
                        signature,
                    )?;
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(ValueCheck {
                        source,
                        stored: source,
                        outcome: CheckOutcome::Holds,
                        target,
                    });
                }
                // commit the call and fail it against the expected result
                SignatureMatch::ReturnMismatch(signature) => {
                    let source = self.commit_callable_signature(
                        node,
                        callee,
                        candidate,
                        argument_nodes,
                        signature,
                    )?;
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(ValueCheck {
                        source,
                        stored: source,
                        outcome: CheckOutcome::Fails(CheckFailure::Relation),
                        target,
                    });
                }
                // a sole candidate reports its own invocation rejection
                SignatureMatch::Invalid {
                    selection,
                    rejection,
                } if is_single_candidate => {
                    self.report_signature_rejection(origin, rejection)?;
                    let source = self.commit_callable_signature(
                        node,
                        callee,
                        candidate,
                        argument_nodes,
                        selection,
                    )?;
                    let target = expectation.map_or(source, |expectation| expectation.target);

                    return Ok(ValueCheck {
                        source,
                        stored: source,
                        outcome: CheckOutcome::Holds,
                        target,
                    });
                }
                // a sole candidate reports its own precise refusal
                SignatureMatch::Inapplicable(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    self.report_signature_rejection(origin, rejection)?;
                    let attempt = self.attempted_call(origin, candidate, argument_nodes)?;

                    return self.reject_call(node, expectation, attempt);
                }
                // leave a refused overload to the shared report below
                SignatureMatch::Invalid { .. } | SignatureMatch::Inapplicable(_) => {}
            }
        }

        // report that no candidate matched the arguments
        let mut rejections = overload.rejections;
        rejections.truncate(REPORTED_REJECTIONS);
        let arguments = self.infer_argument_types(site, argument_nodes)?;
        self.report_no_matching_call(origin, &arguments, &rejections)?;

        // retain the first candidate so downstream passes keep a target
        let attempt = match overload.candidates.first() {
            Some((_, candidate)) => self.attempted_call(origin, candidate, argument_nodes)?,
            None => None,
        };

        self.reject_call(node, expectation, attempt)
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
    ) -> CompilerResult<ValueCheck> {
        let callee = callee.into_global_any(origin.module());
        let Some(expectation) = expectation else {
            self.report_cannot_infer_node(node)?;
            self.commit_error_node(callee)?;

            return self.reject_call(node, None, None);
        };

        // require the expected target to name one newtype
        let target = expectation.target;
        let symbol = match self.ty(target)? {
            dir::Type::Application(instance)
                if matches!(
                    self.symbol_kind_maybe(instance.symbol)?,
                    Some(dir::SymbolKind::Newtype)
                ) =>
            {
                instance.symbol
            }
            _ => {
                self.report_invalid_inferred_construct_target(origin, target)?;
                self.commit_error_node(callee)?;

                return self.reject_call(node, Some(expectation), None);
            }
        };

        // publish the inferred call head as the selected declaration reference
        let reference = self.intern_type(dir::Type::Reference(dir::TypeReference { symbol }))?;
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
        arms: &[CallableArm],
        argument_types: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
    ) -> CompilerResult<ValueCheck> {
        let arguments =
            self.callable_arguments(origin.module(), argument_nodes, ValueUse::Argument)?;

        // select the first applicable overload from every runtime arm
        let overload =
            self.select_overloads(origin, arms, &arguments, argument_types, expectation)?;
        if overload.candidates.is_empty() {
            let argument_types = self.infer_argument_types(site, argument_nodes)?;
            let mut rejections = overload.rejections;
            rejections.truncate(REPORTED_REJECTIONS);
            self.report_no_matching_call(origin, &argument_types, &rejections)?;

            return self.reject_call(node, expectation, None);
        }

        // confirm the selected combination outside the selection probes
        let (signatures, outcome) = match self.attempt_call_arms(
            origin,
            &overload.candidates,
            &arguments,
            argument_types,
            expectation,
        )? {
            CallMatch::Selected(selection) => (selection, CheckOutcome::Holds),
            CallMatch::ReturnMismatch(selection) => {
                (selection, CheckOutcome::Fails(CheckFailure::Relation))
            }
            CallMatch::Inapplicable(rejection) => {
                let argument_types = self.infer_argument_types(site, argument_nodes)?;
                self.report_no_matching_call(origin, &argument_types, &[rejection])?;

                return self.reject_call(node, expectation, None);
            }
        };

        // commit the confirmed calls in runtime-arm order
        for signature in &signatures {
            for (source, coercion) in &signature.coercions {
                self.commit_coercion(*source, coercion.clone())?;
            }
        }

        // build one call decision per runtime arm
        let mut calls = Vec::with_capacity(overload.candidates.len());
        let mut returns = SmallVec::<[_; 4]>::new();
        for ((_, candidate), signature) in overload.candidates.iter().zip(signatures) {
            let call = self.call_decision(
                origin,
                origin.module(),
                candidate,
                argument_nodes,
                &signature,
            )?;
            returns.push(call.return_type);
            calls.push(call);
        }

        // join the arm results into the call's value type
        let return_type = match returns.as_slice() {
            [single] => *single,
            _ => self.normalized_union_type(returns)?,
        };
        let resolution = dir::CallDecision::Union {
            arms: calls,
            ty: return_type,
        };
        self.commit_decision(node, dir::Decision::Call(resolution))?;

        self.commit_node_type(node, return_type)?;

        let target = expectation.map_or(return_type, |expectation| expectation.target);

        Ok(ValueCheck {
            source: return_type,
            stored: return_type,
            outcome,
            target,
        })
    }

    /// Commit one selected callable signature.
    fn commit_callable_signature(
        &mut self,
        node: dir::GlobalNodeIdAny,
        callee: dir::LocalNodeId<dir::Expression>,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: SignatureSelection,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // commit conversions only after the candidate has been selected
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }

        match &candidate.target {
            CallableTarget::Expression
            | CallableTarget::Symbol(_)
            | CallableTarget::CallSignature { .. } => {
                self.commit_call_signature(node, callee, candidate, argument_nodes, signature)
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
    ) -> CompilerResult<dir::GlobalTypeId> {
        // bare declaration heads type as the selected overload
        if matches!(&candidate.target, CallableTarget::Symbol(_))
            && candidate.member_space.is_none()
        {
            let callee = callee.into_global_any(node.module_id);
            self.commit_node_type(callee, signature.callable)?;
        }

        let origin = Origin::Node(node, None);
        let call = self.call_decision(
            origin,
            node.module_id,
            candidate,
            argument_nodes,
            &signature,
        )?;
        let resolution = dir::OperationResolution::One(call);

        self.commit_call_selection(node, resolution)
    }

    /// Build one expression or symbol call resolution.
    fn call_decision(
        &mut self,
        origin: Origin,
        module: ModuleId,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        signature: &SignatureSelection,
    ) -> CompilerResult<dir::Call> {
        let parameters = signature
            .parameters
            .iter()
            .map(|selected| selected.parameter)
            .collect::<Vec<_>>();
        let arguments = self.argument_bindings(origin, module, argument_nodes, &parameters)?;
        let return_type = signature.return_type;
        let target = match &candidate.target {
            // call a runtime value through its own callable type
            CallableTarget::Expression => dir::CallableTarget::Expression {
                generic_arguments: signature.generic_arguments.clone(),
            },
            // dispatch erased signature calls through the callee's own table
            CallableTarget::CallSignature {
                source,
                receiver,
                constraint,
            } => dir::CallableTarget::Dynamic {
                dispatch: dir::DynamicDispatch {
                    receiver: dir::AdjustedReceiver::direct(*receiver),
                    constraint: *constraint,
                },
                function: dir::DynamicFunction::CallSignature(*source),
                generic_arguments: signature.generic_arguments.clone(),
            },
            // drop the receiver static members were selected through
            CallableTarget::Symbol(symbol) => match candidate
                .selected_receiver(signature)
                .filter(|_| candidate.member_space != Some(dir::MemberSpace::Static))
            {
                Some(dir::MemberReceiver::Direct(receiver)) => dir::CallableTarget::Symbol {
                    function: candidate.function_target(signature, Some(receiver))?,
                    dispatch: dir::FunctionDispatch::Direct,
                },
                Some(dir::MemberReceiver::Dynamic(dispatch)) => dir::CallableTarget::Dynamic {
                    dispatch,
                    function: dir::DynamicFunction::Symbol(*symbol),
                    generic_arguments: signature.generic_arguments.clone(),
                },
                None => dir::CallableTarget::Symbol {
                    function: candidate.function_target(signature, None)?,
                    dispatch: dir::FunctionDispatch::Direct,
                },
            },
            // fail loudly on a constructor that reached ordinary call resolution
            CallableTarget::Newtype(_) => {
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

    /// Commit one accepted call selection.
    fn commit_call_selection(
        &mut self,
        node: dir::GlobalNodeIdAny,
        resolution: dir::CallDecision,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let return_type = resolution.return_type();
        self.commit_decision(node, dir::Decision::Call(resolution))?;
        self.commit_node_type(node, return_type)?;

        Ok(return_type)
    }
}
