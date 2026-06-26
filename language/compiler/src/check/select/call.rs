use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Condition, Constraint, Decision, Dependency, GenericArgumentMode, Origin,
    Relation, TypeRewrite, TypeSubstitution, ValueUse, answer,
};
use crate::{CompilerError, CompilerResult};

/// One callable candidate collected from a callee node.
struct CalleeCandidate {
    /// The declaring symbol, when the callee names one.
    symbol: Option<dir::GlobalSymbolId>,
    /// The declaration that exposed the callable.
    owner: Option<dir::GlobalSymbolId>,
    /// The resolved receiver type for member callees.
    receiver: Option<dir::GlobalTypeId>,
    /// The callable type.
    ty: dir::GlobalTypeId,
    /// The owner generic arguments already selected by member lookup.
    generic_arguments: Vec<dir::GenericArgumentBinding>,
}

impl CalleeCandidate {
    /// Return the complete generic argument bindings selected by this call.
    fn selected_generic_arguments(
        &self,
        signature_arguments: &[dir::GenericArgumentBinding],
    ) -> Vec<dir::GenericArgumentBinding> {
        let mut arguments =
            Vec::with_capacity(self.generic_arguments.len() + signature_arguments.len());
        arguments.extend_from_slice(&self.generic_arguments);
        arguments.extend_from_slice(signature_arguments);

        arguments
    }
}

/// One callable signature selected for an invocation.
pub(in crate::check) struct SignatureMatch {
    /// The callable type after substitution.
    pub(in crate::check) callable: dir::GlobalTypeId,
    /// The selected parameters after substitution.
    pub(in crate::check) parameters: SmallVec<[dir::FunctionParameterType; 4]>,
    /// The return type after substitution.
    pub(in crate::check) return_type: dir::GlobalTypeId,
    /// The solved generic argument bindings.
    pub(in crate::check) generic_arguments: Vec<dir::GenericArgumentBinding>,
}

/// One rejected argument selected inside a generic probe.
struct RejectedArgument {
    /// The source location that should receive the diagnostic.
    origin: Origin,
    /// The argument type that failed.
    argument: dir::GlobalTypeId,
    /// The parameter type that rejected the argument.
    parameter: dir::GlobalTypeId,
}

/// Callable candidates collected from one callee.
enum CallCandidates {
    /// At least one candidate must accept.
    Existential(SmallVec<[CalleeCandidate; 2]>),
    /// Every candidate must accept.
    Universal(SmallVec<[CalleeCandidate; 2]>),
}

impl CheckState<'_> {
    /// Select the callable meaning of one call node.
    pub(in crate::check) fn select_call(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        callee: dir::LocalNodeId<dir::Expression>,
        generic_argument_nodes: &[dir::LocalNodeId<dir::GenericArgument>],
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        // collect explicit type arguments from the call syntax
        let mut type_arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in generic_argument_nodes {
            let argument = argument.into_global_any(module);
            let ty = answer!(self.node_type_answer(argument)?);
            type_arguments.push(ty);
        }

        // collect argument types from walked inputs
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in argument_nodes {
            let ty = answer!(self.argument_type(module, *argument)?);
            arguments.push(ty);
        }

        // collect callable candidates from the callee
        let Some(callees) = answer!(self.call_candidates(origin, module, callee)?) else {
            // rejected callees already reported their own diagnostic
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        };
        let candidates = match &callees {
            CallCandidates::Existential(candidates) => candidates,
            CallCandidates::Universal(candidates) => candidates,
        };
        if candidates.is_empty() {
            let callee = callee.into_global_any(module);
            let callee_type = answer!(self.node_type_answer(callee)?);
            self.report_not_callable(origin, callee_type)?;
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        // newtype targets construct through call syntax
        if let [
            CalleeCandidate {
                symbol: Some(symbol),
                ..
            },
        ] = candidates.as_slice()
        {
            let symbol = *symbol;
            if matches!(self.symbol_kind(symbol), dir::SymbolKind::Newtype) {
                return self.select_newtype_construct(
                    node,
                    origin,
                    symbol,
                    argument_nodes,
                    &arguments,
                );
            }
        }

        // union receivers must hold for every variant
        if let CallCandidates::Universal(candidates) = &callees {
            return self.select_union_call(
                node,
                origin,
                argument_nodes,
                candidates,
                &type_arguments,
                &arguments,
            );
        }

        // try candidates in declaration order
        for candidate in candidates {
            let attempt = self.attempt_call(
                origin,
                node,
                module,
                callee.into_global_any(module),
                candidate,
                argument_nodes,
                &type_arguments,
                &arguments,
            )?;

            if answer!(attempt) {
                return Ok(Answer::Ready(()));
            }
        }

        // a sole candidate reports the failing argument directly
        if let [candidate] = candidates.as_slice()
            && self.explain_sole_candidate_failure(
                origin,
                candidate.ty,
                &type_arguments,
                argument_nodes,
                &arguments,
            )?
        {
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        // no candidate accepted the arguments
        self.report_no_matching_call(origin, &arguments, None)?;
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }

    /// Explain the failing arguments of one sole rejected candidate.
    /// Returns whether a diagnostic was emitted; generic candidates decline
    /// because their unsubstituted parameters would mislead.
    fn explain_sole_candidate_failure(
        &mut self,
        origin: Origin,
        function_type: dir::GlobalTypeId,
        type_arguments: &[dir::GlobalTypeId],
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let parameters = self.callable_generic_parameters(origin, function_type)?;

        // uninferred generic candidates cannot report one argument honestly
        let function_type = match parameters.as_slice() {
            [_first, ..] if type_arguments.is_empty() => {
                return self.explain_generic_candidate_failure(
                    origin,
                    function_type,
                    argument_nodes,
                    type_arguments,
                    arguments,
                );
            }
            [_first, ..] => {
                let Some(substitution) = self.instantiate_generic_parameters(
                    origin,
                    &parameters,
                    type_arguments,
                    GenericArgumentMode::Default,
                )?
                else {
                    return Ok(false);
                };

                self.fold_type(module, source, function_type, substitution.rewrite())?
            }
            [] if type_arguments.is_empty() => function_type,
            [] => return Ok(false),
        };

        // read the closed callable shape, following function values
        let Some(mut function_type) = self.reduce_type_root(origin, function_type)?.ready() else {
            return Ok(false);
        };
        if let dir::Type::Function(function) = self.ty(function_type)? {
            function_type = function.signature;
        }
        if let dir::Type::FunctionPointer(function) = self.ty(function_type)? {
            function_type = function.signature;
        }
        let dir::Type::FunctionSignature(function) = self.ty(function_type)? else {
            return Ok(false);
        };
        let parameters = function
            .parameters
            .iter()
            .copied()
            .collect::<SmallVec<[dir::FunctionParameterType; 4]>>();

        // wrong arities report the accepted count instead
        let required = parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = parameters.iter().any(|parameter| parameter.is_rest);
        if arguments.len() < required || (!has_rest && arguments.len() > parameters.len()) {
            let expected = Self::expected_arity(required, parameters.len(), has_rest);
            self.report_wrong_argument_count(origin, expected, arguments.len())?;

            return Ok(true);
        }

        // re-relate each argument loudly under the argument role
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = parameters.get(index).or_else(|| parameters.last());
            let Some(parameter) = parameter else {
                return Ok(false);
            };
            let expression = argument_nodes
                .get(index)
                .and_then(|argument| self.argument_value_node(module, *argument));
            let origin = expression.map(Origin::Node).unwrap_or(origin);

            self.push_constraint(Constraint::flow(
                Relation::Assignable,
                *argument,
                parameter.ty,
                origin,
                Condition::Always,
                ValueUse::Argument,
            ));
        }

        Ok(true)
    }

    /// Explain the first argument failure of one generic sole candidate.
    fn explain_generic_candidate_failure(
        &mut self,
        origin: Origin,
        function_type: dir::GlobalTypeId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let probe = self.begin_probe();
        let result = self.explain_generic_candidate_failure_in_probe(
            origin,
            module,
            source,
            function_type,
            argument_nodes,
            type_arguments,
            arguments,
        );
        self.reject_probe(probe);

        let Some(failure) = result? else {
            return Ok(false);
        };
        self.relate(
            failure.origin,
            Relation::Assignable,
            Some(ValueUse::Argument),
            failure.argument,
            failure.parameter,
        )?;

        Ok(true)
    }

    /// Select the first argument failure while probe variables are available.
    fn explain_generic_candidate_failure_in_probe(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        function_type: dir::GlobalTypeId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<RejectedArgument>> {
        let Some(function_type) = self.reduce_type_root(origin, function_type)?.ready() else {
            return Ok(None);
        };
        let function = match self.ty(function_type)? {
            dir::Type::FunctionSignature(function) => function.clone(),
            dir::Type::Function(function) => {
                return self.explain_generic_candidate_failure_in_probe(
                    origin,
                    module,
                    source,
                    function.signature,
                    argument_nodes,
                    type_arguments,
                    arguments,
                );
            }
            dir::Type::FunctionPointer(function) => {
                return self.explain_generic_candidate_failure_in_probe(
                    origin,
                    module,
                    source,
                    function.signature,
                    argument_nodes,
                    type_arguments,
                    arguments,
                );
            }
            _ => return Ok(None),
        };

        // instantiate the same generic parameters the candidate attempt used
        let generic_parameters = self.signature_generic_parameters(&function)?;
        let Some(substitution) = self.instantiate_generic_parameters(
            origin,
            &generic_parameters,
            type_arguments,
            GenericArgumentMode::Infer,
        )?
        else {
            return Ok(None);
        };
        // infer from arguments that are allowed to contribute
        let mut deferred_arguments =
            SmallVec::<[(usize, dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
        for (index, argument) in arguments.iter().copied().enumerate() {
            let Some(parameter) = function
                .parameters
                .get(index)
                .or_else(|| function.parameters.last())
            else {
                return Ok(None);
            };
            let parameter_type =
                self.fold_type(module, source, parameter.ty, substitution.rewrite())?;
            if self.type_contains_noinfer(parameter_type)? {
                deferred_arguments.push((index, argument, parameter_type));

                continue;
            }

            match self.constrain(origin, Relation::Assignable, argument, parameter_type)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => {
                    return Ok(Some(self.rejected_argument(
                        origin,
                        module,
                        argument_nodes,
                        index,
                        argument,
                        parameter_type,
                    )));
                }
                Answer::Pending(_) => return Ok(None),
            }
        }

        // solve probe variables from the contributing arguments
        let variables = self.substitution_variables(&substitution)?;
        match self.solve_probe_variables(variables)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) | Answer::Pending(_) => return Ok(None),
        }

        // report the first deferred argument that fails after inference
        for (index, argument, parameter_type) in deferred_arguments {
            let parameter_type =
                self.fold_type(module, source, parameter_type, TypeRewrite::Resolve)?;
            let Some(parameter_type) = self.reduce_type(origin, parameter_type)?.ready() else {
                return Ok(None);
            };
            match self.constrain(origin, Relation::Assignable, argument, parameter_type)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => {
                    return Ok(Some(self.rejected_argument(
                        origin,
                        module,
                        argument_nodes,
                        index,
                        argument,
                        parameter_type,
                    )));
                }
                Answer::Pending(_) => return Ok(None),
            }
        }

        Ok(None)
    }

    /// Return one rejected argument selected by a generic probe.
    fn rejected_argument(
        &self,
        origin: Origin,
        module: destack_source::ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        index: usize,
        argument: dir::GlobalTypeId,
        parameter: dir::GlobalTypeId,
    ) -> RejectedArgument {
        let origin = argument_nodes
            .get(index)
            .and_then(|argument| self.argument_value_node(module, *argument))
            .map(Origin::Node)
            .unwrap_or(origin);

        RejectedArgument {
            origin,
            argument,
            parameter,
        }
    }

    /// Render one accepted argument count phrase.
    fn expected_arity(required: usize, total: usize, has_rest: bool) -> String {
        let phrase = match (has_rest, required == total) {
            (true, _) => format!("at least {required}"),
            (false, true) => format!("{total}"),
            (false, false) => format!("{required} to {total}"),
        };
        let noun = if phrase.ends_with('1') && !phrase.ends_with("11") {
            "argument"
        } else {
            "arguments"
        };

        format!("{phrase} {noun}")
    }

    /// Collect callable candidates in declaration order from one callee node.
    /// Returns ready none when the callee already failed upstream.
    fn call_candidates(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        callee: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<Option<CallCandidates>>> {
        let callee_node = callee.into_global_any(module);

        // reference and member callees carry decided declaration meanings
        let awaits_decision = {
            let view = self.module(module).view();
            let expression = view.get(callee);

            expression.is_reference() || matches!(expression, dir::Expression::Member { .. })
        };
        if !awaits_decision {
            // every other callee calls through its function-typed value
            return self.value_call_candidates(origin, callee_node);
        }

        match self.solver.decision(callee_node) {
            Some(Decision::Name(resolution)) => {
                let symbols = resolution
                    .symbols()
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();

                // value bindings call through their walked node type,
                // which carries flow narrowing; declarations carry
                // their overload sets on the symbol
                let value_binding = symbols
                    .iter()
                    .all(|symbol| matches!(self.symbol_kind(*symbol), dir::SymbolKind::Variable));
                if value_binding {
                    return self.value_call_candidates(origin, callee_node);
                }

                let mut candidates = SmallVec::new();
                for symbol in symbols {
                    // gate candidates on their @if availability
                    if !answer!(self.decide_availability(symbol)?) {
                        continue;
                    }

                    if let Some(ty) = self.symbol_type_maybe(symbol) {
                        candidates.push(CalleeCandidate {
                            symbol: Some(symbol),
                            owner: Some(symbol),
                            receiver: None,
                            ty,
                            generic_arguments: Vec::new(),
                        });
                    }
                }

                Ok(Answer::Ready(Some(CallCandidates::Existential(candidates))))
            }
            Some(Decision::Member(resolution)) => match &resolution.target {
                // member candidates carry their receiver-applied types
                dir::MemberTarget::Symbol(candidate) => {
                    let mut candidates = SmallVec::new();
                    candidates.push(CalleeCandidate {
                        symbol: Some(candidate.symbol),
                        owner: Some(candidate.owner),
                        receiver: Some(resolution.receiver),
                        ty: candidate.ty,
                        generic_arguments: candidate.generic_arguments.clone(),
                    });

                    Ok(Answer::Ready(Some(CallCandidates::Existential(candidates))))
                }
                // existential candidates need one match, universal candidates need every match
                dir::MemberTarget::Existential(candidates)
                | dir::MemberTarget::Universal(candidates) => {
                    let is_universal = matches!(resolution.target, dir::MemberTarget::Universal(_));
                    let receiver = resolution.receiver;
                    let candidates = candidates
                        .iter()
                        .map(|candidate| CalleeCandidate {
                            symbol: Some(candidate.symbol),
                            owner: Some(candidate.owner),
                            receiver: Some(receiver),
                            ty: candidate.ty,
                            generic_arguments: candidate.generic_arguments.clone(),
                        })
                        .collect::<SmallVec<[_; 2]>>();

                    Ok(Answer::Ready(Some(if is_universal {
                        CallCandidates::Universal(candidates)
                    } else {
                        CallCandidates::Existential(candidates)
                    })))
                }
                // field members call through their function-typed values
                _ => self.value_call_candidates(origin, callee_node),
            },
            // rejected callees already carry a diagnostic
            Some(Decision::Rejected) => Ok(Answer::Ready(None)),
            Some(other) => Err(CompilerError::Internal {
                message: format!("call callee {callee_node:?} decided as {other:?}"),
            }),
            // function-valued callees can be called from their node type
            None if self.node_type_maybe(callee_node).is_some() => {
                self.value_call_candidates(origin, callee_node)
            }
            // named and member callees must resolve before call selection
            None => Ok(Answer::pending([Dependency::Decision(callee_node)])),
        }
    }

    /// Collect the callable candidate behind one function-typed callee value.
    fn value_call_candidates(
        &mut self,
        origin: Origin,
        callee: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<CallCandidates>>> {
        let Some(ty) = self.node_type_maybe(callee) else {
            return Ok(Answer::pending([Dependency::Decision(callee)]));
        };
        let reduced = answer!(self.reduce_type_root(origin, ty)?);

        let mut candidates = SmallVec::new();
        match self.ty(reduced)? {
            dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_) => {
                candidates.push(CalleeCandidate {
                    symbol: None,
                    owner: None,
                    receiver: None,
                    ty: reduced,
                    generic_arguments: Vec::new(),
                });

                Ok(Answer::Ready(Some(CallCandidates::Existential(candidates))))
            }
            dir::Type::Variable(variable) => {
                let representative = self.solver.representative(*variable)?;

                Ok(Answer::pending([Dependency::Variable(representative)]))
            }
            _ => Ok(Answer::Ready(Some(CallCandidates::Existential(candidates)))),
        }
    }

    /// Attempt one callable candidate against collected arguments,
    /// recording the call resolution when it accepts.
    fn attempt_call(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        module: destack_source::ModuleId,
        callee: dir::GlobalNodeIdAny,
        candidate: &CalleeCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let attempt = self.attempt_callable(origin, candidate.ty, type_arguments, arguments)?;

        let Some(accepted) = answer!(attempt) else {
            return Ok(Answer::Ready(false));
        };
        let selected_arguments = candidate.selected_generic_arguments(&accepted.generic_arguments);
        if !answer!(self.extension_clauses_hold(origin, candidate.owner, &selected_arguments)?) {
            return Ok(Answer::Ready(false));
        }

        // record the accepted resolution
        let target = match candidate.symbol {
            Some(symbol) => dir::CallTarget::Symbol(dir::CallCandidate {
                receiver: candidate.receiver,
                symbol,
                generic_arguments: selected_arguments,
            }),
            None => dir::CallTarget::Expression {
                generic_arguments: accepted.generic_arguments.clone(),
            },
        };
        let resolution = dir::CallResolution::new(
            target,
            Some(accepted.callable),
            Self::parameter_types(&accepted.parameters),
            self.argument_bindings(module, argument_nodes, &accepted.parameters),
            accepted.return_type,
        );
        answer!(self.push_argument_constraints(module, argument_nodes, &resolution.arguments)?);
        self.record_decision(node, Decision::Call(resolution))?;

        // flow the selected callable and result types into their nodes
        self.bind_node_type(node, accepted.return_type)?;
        self.bind_node_type(callee, accepted.callable)?;

        Ok(Answer::Ready(true))
    }

    /// Attempt one callable candidate without recording a decision.
    /// Returns the solved parameters and return type when it accepts.
    pub(in crate::check) fn attempt_callable(
        &mut self,
        origin: Origin,
        function_type: dir::GlobalTypeId,
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<SignatureMatch>>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // close the callable shape first
        let function_type = answer!(self.reduce_type_root(origin, function_type)?);
        let function = match self.ty(function_type)? {
            dir::Type::FunctionSignature(function) => function.clone(),
            dir::Type::Function(function) => {
                let function = function.signature;

                return self.attempt_callable(origin, function, type_arguments, arguments);
            }
            dir::Type::FunctionPointer(function) => {
                let function = function.signature;

                return self.attempt_callable(origin, function, type_arguments, arguments);
            }
            _ => return Ok(Answer::Ready(None)),
        };
        let return_type = function.return_type;

        // instantiate generic parameters under a probe
        let probe = self.begin_probe();
        let generic_parameters = self.signature_generic_parameters(&function)?;
        let attempt = self.attempt_signature(
            origin,
            module,
            source,
            &generic_parameters,
            type_arguments,
            &function,
            return_type,
            arguments,
        )?;

        match attempt {
            Answer::Ready(Some(accepted)) => {
                self.commit_probe(probe);

                Ok(Answer::Ready(Some(accepted)))
            }
            Answer::Ready(None) => {
                self.reject_probe(probe);

                Ok(Answer::Ready(None))
            }
            Answer::Pending(blockers) => {
                self.reject_probe(probe);

                // blockers that died with the probe cannot wake this candidate
                let blockers = self.live_blockers(blockers);
                Ok(Answer::ready_unless_blocked(None, blockers))
            }
        }
    }

    /// Select one call on a union receiver.
    /// Every variant must accept the arguments; the call dispatches at
    /// runtime and joins the variant returns.
    fn select_union_call(
        &mut self,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        candidates: &[CalleeCandidate],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let mut targets = Vec::with_capacity(candidates.len());
        let mut returns = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut parameters: Option<SmallVec<[dir::FunctionParameterType; 4]>> = None;

        for candidate in candidates {
            let attempt = self.attempt_callable(origin, candidate.ty, type_arguments, arguments)?;
            let Some(accepted) = answer!(attempt) else {
                // one rejecting variant rejects the whole union call
                let variant = self.format_type(candidate.ty);
                let note =
                    format!("every union variant must accept the call; '{variant}' does not");
                self.report_no_matching_call(origin, arguments, Some(note))?;
                self.record_decision(node, Decision::Rejected)?;

                return Ok(Answer::Ready(()));
            };
            let selected_arguments =
                candidate.selected_generic_arguments(&accepted.generic_arguments);
            if !answer!(self.extension_clauses_hold(
                origin,
                candidate.owner,
                &selected_arguments,
            )?) {
                let variant = self.format_type(candidate.ty);
                let note =
                    format!("every union variant must accept the call; '{variant}' does not");
                self.report_no_matching_call(origin, arguments, Some(note))?;
                self.record_decision(node, Decision::Rejected)?;

                return Ok(Answer::Ready(()));
            }

            let Some(symbol) = candidate.symbol else {
                continue;
            };
            targets.push(dir::CallCandidate {
                receiver: candidate.receiver,
                symbol,
                generic_arguments: selected_arguments,
            });
            // join by content: variant returns allocate distinct ids
            let return_type = self.settled_root(accepted.return_type)?;
            let mut duplicate = false;
            for seen in returns.iter().copied() {
                if self.ty(seen)? == self.ty(return_type)? {
                    duplicate = true;
                    break;
                }
            }
            if !duplicate {
                returns.push(return_type);
            }
            // the recorded parameters are the first variant's; every
            // variant already accepted the arguments above
            parameters.get_or_insert(accepted.parameters.clone());
        }

        // join the variant returns into the call result
        let return_type = match returns.as_slice() {
            [single] => *single,
            _ => self.push_type(
                origin.module(),
                dir::Type::Union(dir::UnionType {
                    elements: returns.to_vec(),
                }),
                self.origin_source_node(origin)?,
            )?,
        };

        let resolution = dir::CallResolution::new(
            dir::CallTarget::Universal(targets),
            None,
            parameters
                .as_ref()
                .map(|parameters| Self::parameter_types(parameters))
                .unwrap_or_default(),
            parameters
                .as_ref()
                .map(|parameters| {
                    self.argument_bindings(origin.module(), argument_nodes, parameters)
                })
                .unwrap_or_default(),
            return_type,
        );
        answer!(self.push_argument_constraints(
            origin.module(),
            argument_nodes,
            &resolution.arguments
        )?);
        self.record_decision(node, Decision::Call(resolution))?;

        // flow the joined return into the call node variable
        self.bind_node_type(node, return_type)?;

        Ok(Answer::Ready(()))
    }

    /// Attempt one function signature under an active probe.
    pub(in crate::check) fn attempt_signature(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        generic_parameters: &[dir::GlobalGenericParameterId],
        type_arguments: &[dir::GlobalTypeId],
        function: &dir::FunctionSignatureType,
        function_return: Option<dir::GlobalTypeId>,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<SignatureMatch>>> {
        // reject arities the signature cannot accept
        let required = function
            .parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = function
            .parameters
            .iter()
            .any(|parameter| parameter.is_rest);
        if arguments.len() < required || (!has_rest && arguments.len() > function.parameters.len())
        {
            return Ok(Answer::Ready(None));
        }

        // instantiate the signature's generic parameters
        let substitution = match generic_parameters {
            [_first, ..] => {
                let mode = if type_arguments.is_empty() {
                    GenericArgumentMode::Infer
                } else {
                    GenericArgumentMode::Default
                };
                match self.instantiate_generic_parameters(
                    origin,
                    generic_parameters,
                    type_arguments,
                    mode,
                )? {
                    Some(substitution) => substitution,
                    None => return Ok(Answer::Ready(None)),
                }
            }
            [] if type_arguments.is_empty() => Default::default(),
            [] => return Ok(Answer::Ready(None)),
        };
        // check written arguments against their declared constraints
        if !generic_parameters.is_empty() {
            for (parameter, argument) in generic_parameters
                .iter()
                .copied()
                .zip(substitution.arguments.iter().copied())
                .take(type_arguments.len())
            {
                let constraint = self
                    .generic_parameter(parameter)
                    .and_then(|binding| binding.constraint);
                let Some(constraint) = constraint else {
                    continue;
                };
                let constraint =
                    self.fold_type(module, source, constraint, substitution.rewrite())?;

                let source_node = source.into_global(module);
                let condition = self.node_static_condition(source_node);
                if !answer!(self.constrain_generic_argument(
                    origin,
                    source_node,
                    condition,
                    argument,
                    constraint,
                )?) {
                    return Ok(Answer::Ready(None));
                }
            }
        }

        // relate inference-bearing arguments first
        let mut deferred_arguments = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = function
                .parameters
                .get(index)
                .or_else(|| function.parameters.last());
            let Some(parameter) = parameter else {
                return Ok(Answer::Ready(None));
            };
            let parameter_type = if substitution.is_empty() {
                parameter.ty
            } else {
                self.fold_type(module, source, parameter.ty, substitution.rewrite())?
            };
            if self.type_contains_noinfer(parameter_type)? {
                deferred_arguments.push((*argument, parameter_type));

                continue;
            }

            if !answer!(self.constrain(origin, Relation::Assignable, *argument, parameter_type)?) {
                // rejected arguments fail only this candidate
                return Ok(Answer::Ready(None));
            }
        }

        // reject candidates whose inferred arguments violate their constraints
        let variables = self.substitution_variables(&substitution)?;
        match self.solve_probe_variables(variables)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) => return Ok(Answer::Ready(None)),
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        // check NoInfer arguments after inference closes
        for (argument, parameter_type) in deferred_arguments {
            let parameter_type =
                self.fold_type(module, source, parameter_type, TypeRewrite::Resolve)?;
            let parameter_type = answer!(self.reduce_type(origin, parameter_type)?);

            if !answer!(self.constrain(origin, Relation::Assignable, argument, parameter_type)?) {
                return Ok(Answer::Ready(None));
            }
        }

        // resolve the substituted return type
        let return_type = match function_return {
            Some(return_type) => {
                let return_type = if substitution.is_empty() {
                    return_type
                } else {
                    self.fold_type(module, source, return_type, substitution.rewrite())?
                };

                self.fold_type(module, source, return_type, TypeRewrite::Resolve)?
            }
            None => self.push_type(module, dir::Type::Void, source)?,
        };
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| {
                let ty = if substitution.is_empty() {
                    parameter.ty
                } else {
                    self.fold_type(module, source, parameter.ty, substitution.rewrite())?
                };
                let ty = self.fold_type(module, source, ty, TypeRewrite::Resolve)?;

                Ok(dir::FunctionParameterType {
                    ty,
                    static_parameter: None,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                })
            })
            .collect::<CompilerResult<SmallVec<[_; 4]>>>()?;
        let raw_arguments = substitution
            .arguments
            .iter()
            .copied()
            .map(|argument| self.fold_type(module, source, argument, TypeRewrite::Resolve))
            .collect::<CompilerResult<SmallVec<[_; 4]>>>()?;
        let arguments = self.generic_argument_bindings(generic_parameters, &raw_arguments)?;
        let function_type = self.resolve_inferred_function_signature(
            module,
            source,
            function,
            &substitution,
            return_type,
        )?;

        Ok(Answer::Ready(Some(SignatureMatch {
            callable: function_type,
            parameters,
            return_type,
            generic_arguments: arguments,
        })))
    }

    /// Return whether one type graph contains a NoInfer operation.
    fn type_contains_noinfer(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = indexmap::IndexSet::new();
        pending.push(ty);

        while let Some(ty) = pending.pop() {
            let ty = self.settled_root(ty)?;
            if !visited.insert(ty) {
                continue;
            }

            let ty = self.ty(ty)?;
            if matches!(ty, dir::Type::Operation(dir::TypeOperation::NoInfer(_))) {
                return Ok(true);
            }
            if let dir::Type::Instance(instance) = ty
                && self
                    .environment
                    .language
                    .item(instance.symbol)
                    .is_some_and(|item| item == dir::LanguageItem::NoInfer)
            {
                return Ok(true);
            }

            ty.for_each_child(|child| pending.push(child));
        }

        Ok(false)
    }

    /// Return the generic parameters carried by one callable type.
    fn callable_generic_parameters(
        &mut self,
        origin: Origin,
        function_type: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalGenericParameterId; 4]>> {
        let Some(function_type) = self.reduce_type_root(origin, function_type)?.ready() else {
            return Ok(SmallVec::new());
        };

        match self.ty(function_type)?.clone() {
            dir::Type::FunctionSignature(function) => self.signature_generic_parameters(&function),
            dir::Type::Function(function) => {
                self.callable_generic_parameters(origin, function.signature)
            }
            dir::Type::FunctionPointer(function) => {
                self.callable_generic_parameters(origin, function.signature)
            }
            _ => Ok(SmallVec::new()),
        }
    }

    /// Return the generic parameter ids carried by one function signature.
    /// Resolve one callable signature after substitution.
    fn resolve_inferred_function_signature(
        &mut self,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        function: &dir::FunctionSignatureType,
        substitution: &TypeSubstitution,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let this_parameter = match function.this_parameter {
            Some(this_parameter) if substitution.is_empty() => Some(this_parameter),
            Some(this_parameter) => {
                let this_parameter =
                    self.fold_type(module, source, this_parameter, substitution.rewrite())?;
                let this_parameter =
                    self.fold_type(module, source, this_parameter, TypeRewrite::Resolve)?;

                Some(this_parameter)
            }
            None => None,
        };
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| {
                let ty = if substitution.is_empty() {
                    parameter.ty
                } else {
                    self.fold_type(module, source, parameter.ty, substitution.rewrite())?
                };
                let ty = self.fold_type(module, source, ty, TypeRewrite::Resolve)?;

                Ok(dir::FunctionParameterType {
                    ty,
                    static_parameter: None,
                    is_optional: parameter.is_optional,
                    is_rest: parameter.is_rest,
                })
            })
            .collect::<CompilerResult<Vec<_>>>()?;

        self.push_type(
            module,
            dir::Type::FunctionSignature(dir::FunctionSignatureType {
                asynchrony: function.asynchrony,
                template: None,
                this_parameter,
                parameters,
                return_type: Some(return_type),
                is_generator: function.is_generator,
            }),
            source,
        )
    }

    /// Return selected parameter types.
    pub(in crate::check) fn parameter_types(
        parameters: &[dir::FunctionParameterType],
    ) -> Vec<dir::GlobalTypeId> {
        parameters.iter().map(|parameter| parameter.ty).collect()
    }

    /// Return runtime argument bindings for selected parameters.
    pub(in crate::check) fn argument_bindings(
        &self,
        module: destack_source::ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        parameters: &[dir::FunctionParameterType],
    ) -> Vec<dir::ArgumentBinding> {
        let mut argument_index = 0usize;
        let mut bindings = Vec::with_capacity(parameters.len());

        for (parameter, parameter_type) in parameters.iter().enumerate() {
            let argument = if parameter_type.is_rest {
                let rest = argument_nodes[argument_index..]
                    .iter()
                    .map(|argument| argument.into_global_any(module))
                    .collect();
                argument_index = argument_nodes.len();

                dir::ArgumentSource::Rest(rest)
            } else if let Some(argument) = argument_nodes.get(argument_index).copied() {
                argument_index += 1;

                dir::ArgumentSource::Provided(argument.into_global_any(module))
            } else {
                dir::ArgumentSource::Omitted
            };

            bindings.push(dir::ArgumentBinding {
                parameter,
                ty: parameter_type.ty,
                argument,
            });
        }

        bindings
    }

    /// Return generated argument bindings for selected parameters.
    pub(in crate::check) fn generated_argument_bindings(
        argument_sources: &[dir::ArgumentSource],
        parameters: &[dir::FunctionParameterType],
    ) -> Vec<dir::ArgumentBinding> {
        parameters
            .iter()
            .enumerate()
            .map(|(parameter, parameter_type)| {
                let argument = argument_sources
                    .get(parameter)
                    .cloned()
                    .unwrap_or(dir::ArgumentSource::Omitted);

                dir::ArgumentBinding {
                    parameter,
                    ty: parameter_type.ty,
                    argument,
                }
            })
            .collect()
    }

    /// Push final argument constraints for one selected signature.
    pub(in crate::check) fn push_argument_constraints(
        &mut self,
        module: destack_source::ModuleId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        bindings: &[dir::ArgumentBinding],
    ) -> CompilerResult<Answer<()>> {
        for argument in argument_nodes.iter().copied() {
            let argument_node = argument.into_global_any(module);
            let Some(binding) = bindings.iter().find(|binding| {
                matches!(
                    &binding.argument,
                    dir::ArgumentSource::Provided(source) if *source == argument_node
                ) || matches!(
                    &binding.argument,
                    dir::ArgumentSource::Rest(sources) if sources.contains(&argument_node)
                )
            }) else {
                continue;
            };
            let Some(value) = self.argument_value_node(module, argument) else {
                continue;
            };
            let source = answer!(self.argument_type(module, argument)?);

            self.push_constraint(Constraint::flow(
                Relation::Assignable,
                source,
                binding.ty,
                Origin::Node(value),
                Condition::Always,
                ValueUse::Argument,
            ));
        }

        Ok(Answer::Ready(()))
    }

    /// Return the type flowing through one runtime argument.
    pub(in crate::check) fn argument_type(
        &mut self,
        module: destack_source::ModuleId,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let argument_node = argument.into_global_any(module);
        if let Some(ty) = self.node_type_maybe(argument_node) {
            return Ok(Answer::Ready(ty));
        }

        let Some(value) = self.argument_value_node(module, argument) else {
            let error = self.push_type(module, dir::Type::Error, argument.into_any())?;
            self.bind_node_type(argument_node, error)?;

            return Ok(Answer::Ready(error));
        };
        let Some(ty) = self.node_type_maybe(value) else {
            return Ok(Answer::pending([Dependency::Decision(value)]));
        };
        self.bind_node_type(argument_node, ty)?;

        Ok(Answer::Ready(ty))
    }

    /// Return the value expression carried by one argument node.
    pub(in crate::check) fn argument_value_node(
        &self,
        module: destack_source::ModuleId,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> Option<dir::GlobalNodeIdAny> {
        match self.module(module).view().get(argument) {
            dir::Argument::Spread { value, .. }
            | dir::Argument::Positional { value }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. } => Some(value.into_global_any(module)),
            dir::Argument::Error => None,
        }
    }
}
