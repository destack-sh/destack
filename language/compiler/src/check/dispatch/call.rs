use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallCallee, CallDecision, CallFailure, CallTargetResolution, CallTerm, CandidateResolution,
    CheckState, ConstructCandidates, ConstructDecision, ConstructFailure,
    ConstructTargetResolution, Decision, FunctionParameter, FunctionTerm, GenericArgument,
    GenericInstance, GenericParameterBinding, GenericParameterId, MemberCallTerm, MemberDecision,
    MemberFailure, MemberLookup, MemberProjectionOrigin, MemberResolution, MemberTargetResolution,
    Origin, ShapeMember, SubstitutionSet, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
    VariableId,
};

use super::CandidateCardinality;

/// Callable candidate considered by dispatch selection.
pub(in crate::check) struct CallableCandidate {
    /// The module whose type context owns the candidate.
    pub(in crate::check) module: ModuleId,
    /// The resolved declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The callable type operand to inspect.
    pub(in crate::check) ty: TypeOperand,
    /// The already resolved generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The final callable target if this candidate is selected.
    pub(in crate::check) target: CallableTarget,
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

/// Transient callable dispatch while reducing a call expression.
pub(in crate::check) enum CallableDispatch {
    /// Dispatch is waiting for solver input.
    Pending,
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
    pub(in crate::check) fn pending() -> Self {
        Self::Pending
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
    /// Construct target exposed through a call expression.
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
    ) -> CallableDispatch {
        match self {
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
            Self::Construct(target) => {
                let target = target.with_application(instance);

                CallableDispatch::construct_selected(target, function)
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
            let CallCallee::Expression(callee) = call.callee else {
                return Ok(CallableDispatch::call_rejected(CallFailure::NotCallable));
            };
            let Some(term) = self.reduce_type_operand(origin, callee)? else {
                return Ok(CallableDispatch::pending());
            };
            let module = call.source.module_id;
            let function = match self.callable_signature(module, term)? {
                CallableSignature::Pending => return Ok(CallableDispatch::pending()),
                CallableSignature::Absent => {
                    return Ok(CallableDispatch::call_rejected(CallFailure::NotCallable));
                }
                CallableSignature::Present(function) => function,
            };

            self.select_callable_signature(
                origin,
                module,
                call.source,
                None,
                None,
                function,
                &call.generic_arguments,
                &call.argument_types,
                &call.arguments,
                expected,
                CallableTarget::Expression,
            )
        }
    }

    /// Return the already chosen decision for one call.
    fn selected_call(&self, call: &CallTerm) -> Option<CallableDispatch> {
        // return cached construct decision
        if let Some(decision) = self.inference.construct(call.source) {
            let selection = match decision {
                ConstructDecision::Resolved(selection) => {
                    CallableDispatch::construct_selected(selection.target, selection.function)
                }
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
            CallDecision::Resolved(selection) => {
                CallableDispatch::call_selected(selection.target, selection.function)
            }
            CallDecision::Rejected(failure) => CallableDispatch::call_rejected(failure),
        };

        Some(selection)
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
        let function = match self.callable_signature(module, term)? {
            CallableSignature::Pending => return Ok(Some(CallableDispatch::pending())),
            CallableSignature::Absent => {
                return match self.select_construct_call(origin, call, expected, term)? {
                    Some(result) => Ok(Some(result)),
                    None => Ok(Some(CallableDispatch::call_rejected(
                        CallFailure::NotCallable,
                    ))),
                };
            }
            CallableSignature::Present(function) => function,
        };
        let result = self.select_callable_signature(
            origin,
            module,
            call.source,
            Some(symbol),
            None,
            function,
            &call.generic_arguments,
            &call.argument_types,
            &call.arguments,
            expected,
            CallableTarget::Symbol {
                symbol,
                receiver: None,
            },
        )?;

        Ok(Some(result))
    }

    /// Select a construct target exposed through a call expression.
    fn select_construct_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
        term: TypeOperand,
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
            &call.argument_types,
            &call.arguments,
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
                self.resolve_type_member(origin, module, receiver, &member.key)?
            }
            MemberProjectionOrigin::Protocol { protocol } => {
                self.resolve_protocol_member(origin, module, receiver, &member.key, &protocol)?
            }
        };
        let member_matches = match member_match {
            MemberLookup::Found(member_matches) => member_matches,
            MemberLookup::Field(ty) => {
                return self.select_member_field_call(origin, call, &member, ty, expected);
            }
            MemberLookup::Pending => {
                return Ok(Some(CallableDispatch::pending()));
            }
            MemberLookup::Missing => {
                if let MemberProjectionOrigin::Expression { source } = member.origin {
                    self.inference.select_member(
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
            &call.argument_types,
            &call.arguments,
            expected,
        )?;

        Ok(Some(result))
    }

    /// Select a call whose callee is a structural field projection.
    fn select_member_field_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        member: &MemberCallTerm,
        ty: TypeOperand,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableDispatch>> {
        // record the structural member target
        if let MemberProjectionOrigin::Expression { source } = member.origin {
            let resolution = MemberResolution {
                source,
                receiver: member.receiver,
                target: MemberTargetResolution::Field(member.key),
            };

            self.inference
                .select_member(source, MemberDecision::Resolved(resolution))?;
        }

        // reduce the field type to a callable signature
        let reduction = self.reduce_type_operand(origin, ty)?;
        let Some(term) = reduction else {
            return Ok(Some(CallableDispatch::pending()));
        };
        let signature = match self.callable_signature(call.source.module_id, term)? {
            CallableSignature::Pending => return Ok(Some(CallableDispatch::pending())),
            CallableSignature::Absent => return Ok(Some(CallableDispatch::invalid())),
            CallableSignature::Present(function) => function,
        };

        // dispatch the structural field as an expression call
        let result = self.select_callable_signature(
            origin,
            call.source.module_id,
            call.source,
            None,
            None,
            signature,
            &call.generic_arguments,
            &call.argument_types,
            &call.arguments,
            expected,
            CallableTarget::Expression,
        )?;

        Ok(Some(result))
    }

    /// Select one callable signature for actual call arguments.
    pub(in crate::check) fn select_callable_signature(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        argument_types: &[TypeOperand],
        arguments: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
        target: CallableTarget,
    ) -> CompilerResult<CallableDispatch> {
        // reject impossible explicit generic arity
        let is_generic = !function.generic_parameters.is_empty();
        if generic_arguments.len() > function.generic_parameters.len()
            || (!is_generic && !generic_arguments.is_empty())
        {
            return Ok(CallableDispatch::call_rejected(CallFailure::NoMatch));
        }

        // instantiate receiver and call generics
        let generic_parameters = function.generic_parameters.clone();
        let (generic_instance, substitution) = self.instantiate_call_signature(
            module,
            source,
            owner,
            target.receiver(),
            instance,
            &function,
            generic_arguments,
            arguments,
        )?;
        let mut function = function.substitute(module, &substitution, self)?;
        function.generic_parameters.clear();

        // constrain call inputs against the selected signature
        self.expect_call_arguments(origin, argument_types, &function.parameters)?;
        self.expect_call_return(origin, &function, expected)?;

        // constrain inferred call generics
        if is_generic {
            self.solve_omitted_generic_defaults(
                module,
                &substitution,
                &generic_parameters,
                generic_arguments.len(),
            )?;
            self.expect_call_generic_constraints(
                origin,
                module,
                &substitution,
                &generic_parameters,
            )?;
        }

        // reduce final argument and result decisions
        let argument_decision = self.decide_call_arguments(argument_types, &function.parameters)?;
        let return_decision = self.decide_call_return(&function, expected)?;
        let generic_constraints =
            self.reduce_call_generic_constraints(module, &substitution, &generic_parameters)?;
        let input_decision = argument_decision.and(generic_constraints);
        match input_decision {
            Decision::Yes => match return_decision {
                Decision::Yes | Decision::Undecidable => {
                    Ok(target.into_dispatch(generic_instance, function))
                }
                Decision::No => self.call_signature_rejected(argument_types, &function.parameters),
            },
            Decision::Undecidable => Ok(CallableDispatch::Pending),
            Decision::No => self.call_signature_rejected(argument_types, &function.parameters),
        }
    }

    /// Return the precise rejection for a failed callable signature.
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
        argument_types: &[TypeOperand],
        arguments: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let mut callable_signature_count = 0;
        let mut pending_candidate = None;
        let mut pending_count = 0;

        // choose the first compatible declaration order candidate
        for (index, candidate) in candidates.iter().enumerate() {
            let probe = self.inference.begin_probe();
            let result = self.select_one_callable_candidate(
                origin,
                source,
                candidate,
                generic_arguments,
                argument_types,
                arguments,
                expected,
            )?;

            match result {
                CallableDispatch::CallSelected { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                CallableDispatch::ConstructSelected { .. } => {
                    unreachable!("callable candidate dispatch produced a construct selection");
                }
                CallableDispatch::Pending => {
                    self.inference.drop_probe(probe);
                    pending_count += 1;
                    pending_candidate = Some(index);
                }
                CallableDispatch::CallRejected(CallFailure::ArgumentType { .. }) => {
                    self.inference.drop_probe(probe);
                    callable_signature_count += 1;
                }
                CallableDispatch::CallRejected(CallFailure::NoMatch) => {
                    self.inference.drop_probe(probe);
                    callable_signature_count += 1;
                }
                CallableDispatch::CallRejected(CallFailure::NotCallable) => {
                    self.inference.drop_probe(probe);
                }
                CallableDispatch::ConstructRejected(_) => {
                    self.inference.drop_probe(probe);
                }
                CallableDispatch::Invalid => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
            }
        }

        // commit the unique unresolved candidate
        if let (1, Some(index)) = (pending_count, pending_candidate) {
            let probe = self.inference.begin_probe();
            let result = self.select_one_callable_candidate(
                origin,
                source,
                &candidates[index],
                generic_arguments,
                argument_types,
                arguments,
                expected,
            )?;

            self.inference.commit_probe(probe)?;

            return Ok(result);
        }

        // wait for ambiguous unresolved candidates
        if pending_count > 1 {
            return Ok(CallableDispatch::pending());
        }
        // distinguish rejected callable overloads from non callable values
        else if callable_signature_count > 0 {
            Ok(CallableDispatch::call_rejected(CallFailure::NoMatch))
        } else {
            Ok(CallableDispatch::call_rejected(CallFailure::NotCallable))
        }
    }

    /// Select one callable candidate inside the active inference probe.
    fn select_one_callable_candidate(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        candidate: &CallableCandidate,
        generic_arguments: &[GenericArgument],
        argument_types: &[TypeOperand],
        arguments: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let reduction = self.reduce_type_operand(origin, candidate.ty)?;
        let Some(term) = reduction else {
            return Ok(CallableDispatch::Pending);
        };
        let function = match self.callable_signature(candidate.module, term)? {
            CallableSignature::Pending => return Ok(CallableDispatch::pending()),
            CallableSignature::Absent => {
                return Ok(CallableDispatch::call_rejected(CallFailure::NotCallable));
            }
            CallableSignature::Present(function) => function,
        };

        self.select_callable_signature(
            origin,
            candidate.module,
            source,
            Some(candidate.symbol),
            candidate.instance.clone(),
            function,
            generic_arguments,
            argument_types,
            arguments,
            expected,
            candidate.target.clone(),
        )
    }

    /// Return the callable signature represented by one type operand.
    pub(in crate::check) fn callable_signature(
        &mut self,
        module: ModuleId,
        operand: TypeOperand,
    ) -> CompilerResult<CallableSignature> {
        let Some(term) = self.type_operand_term(operand)? else {
            return Ok(CallableSignature::Pending);
        };
        let callable = match term {
            TypeTerm::Function(function) => {
                CallableSignature::Present(self.inference.term(function).clone())
            }
            TypeTerm::Type(ty) => match self.r#type(ty) {
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
                let members = self.inference.term(shape).members.clone();

                self.shape_callable_signature(module, &members)?
            }
            _ => CallableSignature::Absent,
        };

        Ok(callable)
    }

    /// Return the callable signature for a shape term.
    fn shape_callable_signature(
        &mut self,
        module: ModuleId,
        members: &[ShapeMember],
    ) -> CompilerResult<CallableSignature> {
        for member in members {
            let ShapeMember::CallSignature { ty } = member else {
                continue;
            };
            return self.callable_signature(module, *ty);
        }

        Ok(CallableSignature::Absent)
    }

    /// Expect call instantiation arguments to satisfy declared constraints.
    fn expect_call_generic_constraints(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameters: &[GenericParameterId],
    ) -> CompilerResult<()> {
        // constrain each substituted generic argument by its declared parameter
        for parameter in parameters {
            let generic = self.inference.require_generic_parameter(*parameter).clone();

            self.expect_call_generic_constraint(
                origin,
                module,
                substitution,
                *parameter,
                &generic,
            )?;
        }

        Ok(())
    }

    /// Expect one call instantiation argument to satisfy its declared constraint.
    fn expect_call_generic_constraint(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameter: GenericParameterId,
        generic: &GenericParameterBinding,
    ) -> CompilerResult<()> {
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
                    return Ok(());
                };
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.reduce_contextual_type_assignability(origin, argument, constraint)?;
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
                    return Ok(());
                };
                let source = self
                    .inference
                    .push_term(TypeTerm::StaticValue { value: argument });
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.reduce_contextual_type_assignability(origin, source, constraint)?;
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
            } => {}
        }

        Ok(())
    }

    /// Decide whether call generic arguments satisfy declared constraints.
    fn reduce_call_generic_constraints(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameters: &[GenericParameterId],
    ) -> CompilerResult<Decision> {
        let mut decision = Decision::Yes;

        // combine each generic parameter constraint
        for parameter in parameters {
            let generic = self.inference.require_generic_parameter(*parameter).clone();

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
        substitution: &SubstitutionSet,
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
        substitution: &SubstitutionSet,
        parameters: &[GenericParameterId],
        explicit_count: usize,
    ) -> CompilerResult<()> {
        for parameter in parameters.iter().skip(explicit_count) {
            self.solve_omitted_generic_default(module, substitution, *parameter)?;
        }

        Ok(())
    }

    /// Solve one omitted generic argument from its default.
    fn solve_omitted_generic_default(
        &mut self,
        module: ModuleId,
        substitution: &SubstitutionSet,
        parameter: GenericParameterId,
    ) -> CompilerResult<()> {
        let generic = self.inference.require_generic_parameter(parameter).clone();

        match generic {
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
                    return Ok(());
                };
                let default = self.substitute_type_operand(module, substitution, default)?;

                self.solve_default_type_variable(argument, default)?;
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
                    return Ok(());
                };
                let default = self.substitute_static_operand(module, substitution, default)?;

                self.solve_default_static_variable(argument, default)?;
            }
            GenericParameterBinding::Type { .. }
            | GenericParameterBinding::VariadicType { .. }
            | GenericParameterBinding::Static { .. }
            | GenericParameterBinding::VariadicStatic { .. } => {}
        };

        Ok(())
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
    ) -> CompilerResult<()> {
        for (argument, parameter) in arguments.iter().zip(parameters) {
            let parameter = parameter.ty;

            self.reduce_contextual_type_assignability(origin, *argument, parameter)?;
        }

        Ok(())
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
    ) -> CompilerResult<()> {
        let Some(expected) = expected else {
            return Ok(());
        };
        let Some(return_type) = function.return_type else {
            let void = TypeTerm::Literal(TypeLiteralTerm::Void);
            let void = self.inference.push_term(void);

            self.reduce_contextual_type_assignability(origin, void, expected)?;

            return Ok(());
        };

        self.reduce_contextual_type_assignability(origin, return_type, expected)?;

        Ok(())
    }
}
