use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CallArgument, CallCallee, CallDecision, CallFailure, CallTargetResolution, CallTerm,
    CallableApplicability, CallableCandidate, CallableSignature, CandidateResolution, CheckState,
    ConstructCandidates, ConstructDecision, ConstructFailure, ConstructTargetResolution,
    Dependency, FunctionTerm, GenericArgument, MemberDecision, MemberLookup,
    MemberProjectionOrigin, MemberReceiver, MemberResolution, MemberTargetResolution, Origin,
    TermId, TypeOperand, VariableId,
};

/// Transient callable dispatch while reducing a call expression.
pub(in crate::check) enum CallableDispatch {
    /// Dispatch is waiting for dependency.
    Pending(SmallVec<[Dependency; 2]>),
    /// Dispatch is invalid because a sub-selection already failed.
    Invalid,
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
    },
    /// One construct candidate resolved.
    ConstructSelected {
        /// The resolved construct target.
        target: ConstructTargetResolution,
        /// The resolved constructor signature.
        function: FunctionTerm,
    },
}

impl CallableDispatch {
    /// Return a pending dispatch.
    pub(in crate::check) fn pending(dependencies: impl IntoIterator<Item = Dependency>) -> Self {
        let mut blockers = SmallVec::new();

        for dependency in dependencies {
            if !blockers.contains(&dependency) {
                blockers.push(dependency);
            }
        }

        Self::Pending(blockers)
    }

    /// Return an invalid dispatch.
    pub(in crate::check) fn invalid() -> Self {
        Self::Invalid
    }

    /// Return a selected call dispatch.
    pub(in crate::check) fn call_selected(
        target: CallTargetResolution,
        function: FunctionTerm,
    ) -> Self {
        Self::CallSelected { target, function }
    }

    /// Return a selected construct dispatch.
    pub(in crate::check) fn construct_selected(
        target: ConstructTargetResolution,
        function: FunctionTerm,
    ) -> Self {
        Self::ConstructSelected { target, function }
    }

    /// Return a rejected call dispatch.
    pub(in crate::check) fn call_rejected(failure: CallFailure) -> Self {
        Self::CallRejected(failure)
    }

    /// Return a rejected construct dispatch.
    pub(in crate::check) fn construct_rejected(failure: ConstructFailure) -> Self {
        Self::ConstructRejected(failure)
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
}

impl CallableTarget {
    /// Return the selected receiver type when this target has one.
    pub(in crate::check) fn receiver(&self) -> Option<TypeOperand> {
        match self {
            Self::Symbol { receiver, .. } | Self::Union { receiver, .. } => *receiver,
            Self::Expression => None,
        }
    }

    /// Return this selected target as a callable dispatch.
    pub(in crate::check) fn into_callable_dispatch(
        self,
        signature: CallableApplicability,
    ) -> CallableDispatch {
        match signature {
            CallableApplicability::Pending(blockers) => CallableDispatch::Pending(blockers),
            CallableApplicability::Rejected(failure) => CallableDispatch::CallRejected(failure),
            CallableApplicability::Applicable { instance, function } => match self {
                Self::Expression => {
                    let target = CallTargetResolution::Expression { instance };

                    CallableDispatch::call_selected(target, function)
                }
                Self::Symbol { symbol, receiver } => {
                    let target = CallTargetResolution::Symbol {
                        symbol,
                        instance,
                        receiver,
                    };

                    CallableDispatch::call_selected(target, function)
                }
                Self::Union {
                    candidates,
                    receiver,
                } => {
                    let target = CallTargetResolution::Union {
                        candidates,
                        receiver,
                    };

                    CallableDispatch::call_selected(target, function)
                }
            },
        }
    }
}

impl CheckState<'_> {
    /// Select one runtime call target from callable candidates.
    pub(in crate::check) fn select_call_target(
        &mut self,
        origin: Origin,
        call: TermId<CallTerm>,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let source = self.inference.term(call).source;

        // return cached call decision
        if let Some(selection) = self.selected_call(source) {
            return Ok(selection);
        }

        // select member callee
        if let Some(result) = self.select_member_call(origin, call, expected)? {
            Ok(result)
        }
        // select reference callee
        else if let Some(result) = self.select_reference_call(origin, call, expected)? {
            Ok(result)
        }
        // select expression callee
        else {
            let callee = self.inference.term(call).callee;
            let CallCallee::Expression(callee) = callee else {
                return Ok(CallableDispatch::call_rejected(CallFailure::NotCallable));
            };
            let Answer::Ready(term) = self.reduce_type_operand(origin, callee)? else {
                return Ok(CallableDispatch::pending(callee.dependencies(self)));
            };
            let module = source.module_id;
            let function = match self.callable_signature(origin, module, term)? {
                CallableSignature::Pending(blockers) => {
                    return Ok(CallableDispatch::Pending(blockers));
                }
                CallableSignature::Absent => {
                    return Ok(CallableDispatch::call_rejected(CallFailure::NotCallable));
                }
                CallableSignature::Present(function) => function,
            };
            let (generic_arguments, arguments) = self.call_arguments(call);

            let signature = self.select_callable_signature(
                origin,
                module,
                source,
                None,
                None,
                function,
                &generic_arguments,
                &arguments,
                expected,
                None,
            )?;

            Ok(CallableTarget::Expression.into_callable_dispatch(signature))
        }
    }

    /// Return the already chosen decision for one call.
    fn selected_call(&self, source: dir::GlobalNodeIdAny) -> Option<CallableDispatch> {
        // return cached construct decision
        if let Some(decision) = self.inference.construct(source) {
            let selection = match decision {
                ConstructDecision::Resolved(selection) => CallableDispatch::construct_selected(
                    selection.target.clone(),
                    selection.function.clone(),
                ),
                ConstructDecision::Rejected(failure) => {
                    CallableDispatch::construct_rejected(*failure)
                }
            };

            return Some(selection);
        }

        // return cached call decision
        let Some(decision) = self.inference.call(source) else {
            return None;
        };
        let selection = match decision {
            CallDecision::Resolved(selection) => CallableDispatch::call_selected(
                selection.target.clone(),
                selection.function.clone(),
            ),
            CallDecision::Rejected(failure) => CallableDispatch::call_rejected(failure.clone()),
        };

        Some(selection)
    }

    /// Select a call whose callee is a symbol-backed reference.
    fn select_reference_call(
        &mut self,
        origin: Origin,
        call: TermId<CallTerm>,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let source = self.inference.term(call).source;
        let callee = self.inference.term(call).callee;
        let CallCallee::Reference {
            value: callee,
            symbol,
        } = callee
        else {
            return Ok(None);
        };

        let module = source.module_id;
        let Answer::Ready(term) = self.reduce_type_operand(origin, callee)? else {
            return Ok(Some(CallableDispatch::pending(callee.dependencies(self))));
        };
        let function = match self.callable_signature(origin, module, term)? {
            CallableSignature::Pending(blockers) => {
                return Ok(Some(CallableDispatch::Pending(blockers)));
            }
            CallableSignature::Absent => {
                let (generic_arguments, arguments) = self.call_arguments(call);

                return match self.select_construct_call(
                    origin,
                    source,
                    &generic_arguments,
                    &arguments,
                    expected,
                    term,
                )? {
                    Some(result) => Ok(Some(result)),
                    None => Ok(Some(CallableDispatch::call_rejected(
                        CallFailure::NotCallable,
                    ))),
                };
            }
            CallableSignature::Present(function) => function,
        };
        let target = CallableTarget::Symbol {
            symbol,
            receiver: None,
        };
        let (generic_arguments, arguments) = self.call_arguments(call);
        let signature = self.select_callable_signature(
            origin,
            module,
            source,
            Some(symbol),
            None,
            function,
            &generic_arguments,
            &arguments,
            expected,
            target.receiver(),
        )?;
        let result = target.into_callable_dispatch(signature);

        Ok(Some(result))
    }

    /// Select a construct target exposed through a call expression.
    fn select_construct_call(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        generic_arguments: &[GenericArgument],
        arguments: &[CallArgument],
        expected: Option<VariableId>,
        term: TypeOperand,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let module = source.module_id;
        let candidates = match self.construct_candidates(module, term)? {
            ConstructCandidates::Pending(blockers) => {
                return Ok(Some(CallableDispatch::Pending(blockers)));
            }
            ConstructCandidates::Absent => return Ok(None),
            ConstructCandidates::Present(candidates) => candidates,
        };
        let result = self.select_construct_candidate(
            origin,
            module,
            source,
            candidates,
            generic_arguments,
            arguments,
            expected,
        )?;

        Ok(Some(result.into_callable()))
    }

    /// Select a call whose callee is a member projection.
    fn select_member_call(
        &mut self,
        origin: Origin,
        call: TermId<CallTerm>,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let source = self.inference.term(call).source;
        let callee = self.inference.term(call).callee;
        let CallCallee::Member(member) = callee else {
            return Ok(None);
        };
        let member = self.inference.term(member);
        let member_origin = member.origin.clone();
        let member_receiver = member.receiver.clone();
        let member_key = member.key;
        let receiver = self.member_receiver_type_operand(&member_receiver);
        let module = source.module_id;
        let lookup = match &member_origin {
            MemberProjectionOrigin::Expression { source } => {
                self.resolve_member(Origin::Node(*source), module, &member_receiver, &member_key)?
            }
            MemberProjectionOrigin::Protocol { protocol } => {
                self.resolve_protocol_member(origin, module, receiver, &member_key, protocol)?
            }
        };
        let member_candidates = match lookup {
            MemberLookup::Found(candidates) => candidates,
            MemberLookup::Field(ty) => {
                return self.select_member_field_call(
                    origin,
                    source,
                    call,
                    &member_origin,
                    &member_receiver,
                    member_key,
                    ty,
                    expected,
                );
            }
            MemberLookup::Pending(blockers) => {
                return Ok(Some(CallableDispatch::Pending(blockers)));
            }
            MemberLookup::Missing => {
                return Ok(Some(CallableDispatch::invalid()));
            }
        };
        let receiver = self.member_receiver_type_operand(&member_receiver);
        let member_candidates = member_candidates
            .iter()
            .filter_map(|candidate| candidate.ty.map(|ty| (candidate, ty)))
            .collect::<Vec<_>>();
        let union_target = if member_candidates.len() > 1 {
            let candidates = member_candidates
                .iter()
                .map(|(candidate, _)| CandidateResolution {
                    symbol: candidate.symbol,
                    instance: candidate.instance.clone(),
                })
                .collect();

            Some(CallableTarget::Union {
                candidates,
                receiver: Some(receiver),
            })
        } else {
            None
        };
        let callable_candidates = member_candidates
            .iter()
            .map(|(candidate, ty)| {
                let target = if let Some(target) = &union_target {
                    target.clone()
                } else {
                    CallableTarget::Symbol {
                        symbol: candidate.symbol,
                        receiver: Some(receiver),
                    }
                };

                CallableCandidate {
                    module,
                    symbol: candidate.symbol,
                    ty: *ty,
                    instance: candidate.instance.clone(),
                    target,
                }
            })
            .collect::<Vec<_>>();
        let (generic_arguments, arguments) = self.call_arguments(call);
        let result = self.select_callable_candidate(
            origin,
            source,
            &callable_candidates,
            &generic_arguments,
            &arguments,
            expected,
        )?;

        Ok(Some(result))
    }

    /// Select a call whose callee is a structural field projection.
    fn select_member_field_call(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        call: TermId<CallTerm>,
        member_origin: &MemberProjectionOrigin,
        member_receiver: &MemberReceiver,
        member_key: dir::StaticKey,
        ty: TypeOperand,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableDispatch>> {
        // record the structural member target
        if let MemberProjectionOrigin::Expression { source } = member_origin {
            let receiver = self.member_receiver_type_operand(member_receiver);
            let resolution = MemberResolution {
                source: *source,
                receiver,
                target: MemberTargetResolution::Field(member_key),
            };

            self.inference
                .select_member(*source, MemberDecision::Resolved(resolution))?;
        }

        // reduce the field type to a callable signature
        let reduction = self.reduce_type_operand(origin, ty)?;
        let Answer::Ready(term) = reduction else {
            return Ok(Some(CallableDispatch::pending(ty.dependencies(self))));
        };
        let signature = match self.callable_signature(origin, source.module_id, term)? {
            CallableSignature::Pending(blockers) => {
                return Ok(Some(CallableDispatch::Pending(blockers)));
            }
            CallableSignature::Absent => return Ok(Some(CallableDispatch::invalid())),
            CallableSignature::Present(function) => function,
        };

        // select the structural field as an expression call
        let (generic_arguments, arguments) = self.call_arguments(call);
        let signature = self.select_callable_signature(
            origin,
            source.module_id,
            source,
            None,
            None,
            signature,
            &generic_arguments,
            &arguments,
            expected,
            None,
        )?;
        let result = CallableTarget::Expression.into_callable_dispatch(signature);

        Ok(Some(result))
    }

    /// Return the call argument lists needed by dispatch selection.
    fn call_arguments(
        &self,
        call: TermId<CallTerm>,
    ) -> (SmallVec<[GenericArgument; 2]>, SmallVec<[CallArgument; 4]>) {
        let call = self.inference.term(call);
        let generic_arguments = call.generic_arguments.iter().copied().collect();
        let arguments = call.arguments.iter().copied().collect();

        (generic_arguments, arguments)
    }
}
