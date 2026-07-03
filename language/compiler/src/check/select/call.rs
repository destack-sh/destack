use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Decision, Dependency, FlowSite, Origin, PlaceUse, SignatureRejection,
    answer,
};
use crate::{CompilerError, CompilerResult};

/// One callable candidate collected from a callee node.
struct CallableCandidate {
    /// The declaring symbol, when the callee names one.
    symbol: Option<dir::GlobalSymbolId>,
    /// The resolved receiver type for member callees.
    receiver: Option<dir::GlobalTypeId>,
    /// The callable type.
    ty: dir::GlobalTypeId,
    /// The owner generic arguments already selected by member lookup.
    generic_arguments: Vec<dir::GenericArgumentBinding>,
}

impl CallableCandidate {
    /// Return owner and signature generic arguments as one call binding list.
    fn call_generic_arguments(
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

impl CheckState<'_> {
    /// Select the callable meaning of one call node.
    pub(in crate::check) fn select_call(
        &mut self,
        site: FlowSite,
        callee: dir::LocalNodeId<dir::Expression>,
        generic_argument_nodes: &[dir::LocalNodeId<dir::GenericArgument>],
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Answer<()>> {
        let node = site.node.into_typed::<dir::Expression>();
        let module = node.module_id;
        let node = node.into_any();
        let origin = site.origin();
        let callee_site = self.node_site(callee.into_global_any(module))?;

        // collect explicit type arguments from the call node
        let mut type_arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in generic_argument_nodes {
            let argument = argument.into_global_any(module);
            let ty = answer!(self.committed_node_type(argument)?);
            type_arguments.push(ty);
        }

        // infer argument value types at this call site
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in argument_nodes {
            let ty = answer!(self.argument_value_type(site, *argument)?);
            arguments.push(ty);
        }

        // collect callable candidates from the callee
        let Some(callees) = answer!(self.callable_candidates(origin, module, callee_site)?) else {
            // rejected callees already reported their own diagnostic
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
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

            return Ok(Answer::Ready(()));
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
                return self.select_newtype_construct(
                    site,
                    node,
                    origin,
                    symbol,
                    argument_nodes,
                    &type_arguments,
                    &arguments,
                );
            }
        }

        // union receivers must hold for every variant
        if let CallCandidates::All(candidates) = &callees {
            return self.select_universal_call(
                site,
                node,
                origin,
                argument_nodes,
                candidates,
                &type_arguments,
                &arguments,
            );
        }

        // try candidates in declaration order
        let is_single_candidate = candidates.len() == 1;
        for candidate in candidates {
            let attempt = self.attempt_call(
                origin,
                module,
                candidate,
                argument_nodes,
                &type_arguments,
                &arguments,
            )?;

            match answer!(attempt) {
                Ok(selection) => {
                    return self.commit_call_selection(site, node, argument_nodes, selection);
                }
                Err(rejection) if is_single_candidate && rejection.is_precise() => {
                    self.report_signature_rejection(origin, module, argument_nodes, rejection)?;
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(()));
                }
                Err(_) => {}
            }
        }

        // no candidate matched the arguments
        self.report_no_matching_call(origin, &arguments, None)?;
        self.commit_decision(node, Decision::Rejected)?;
        self.commit_error_node(node)?;

        Ok(Answer::Ready(()))
    }

    /// Collect callable candidates in declaration order from one callee node.
    /// Returns ready none when the callee already failed upstream.
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

        match self.decision(callee_node) {
            Some(Decision::Name(resolution)) => {
                let symbols = resolution
                    .symbols()
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 2]>>();

                // value bindings call through their inferred node type,
                // which carries flow narrowing; declarations carry
                // their overload sets on the symbol
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
                        receiver: None,
                        ty,
                        generic_arguments: Vec::new(),
                    });
                }
                if !blockers.is_empty() {
                    return Ok(Answer::Pending(blockers));
                }

                Ok(Answer::Ready(Some(CallCandidates::Any(candidates))))
            }
            Some(Decision::Member(resolution)) => match &resolution.target {
                // member candidates carry their receiver-applied types
                dir::MemberTarget::Symbol(candidate) => {
                    let mut candidates = SmallVec::new();
                    candidates.push(CallableCandidate {
                        symbol: Some(candidate.symbol),
                        receiver: Some(resolution.receiver),
                        ty: candidate.ty,
                        generic_arguments: candidate.generic_arguments.clone(),
                    });

                    Ok(Answer::Ready(Some(CallCandidates::Any(candidates))))
                }
                // existential candidates need one match, universal candidates need every match
                dir::MemberTarget::Existential(candidates)
                | dir::MemberTarget::Universal(candidates) => {
                    let is_universal = matches!(resolution.target, dir::MemberTarget::Universal(_));
                    let receiver = resolution.receiver;
                    let candidates = candidates
                        .iter()
                        .map(|candidate| CallableCandidate {
                            symbol: Some(candidate.symbol),
                            receiver: Some(receiver),
                            ty: candidate.ty,
                            generic_arguments: candidate.generic_arguments.clone(),
                        })
                        .collect::<SmallVec<[_; 2]>>();

                    Ok(Answer::Ready(Some(if is_universal {
                        CallCandidates::All(candidates)
                    } else {
                        CallCandidates::Any(candidates)
                    })))
                }
                // field members call through their function-typed values
                _ => self.value_callable_candidates(origin, callee_site),
            },
            // rejected callees already carry a diagnostic
            Some(Decision::Rejected) => Ok(Answer::Ready(None)),
            Some(other) => Err(CompilerError::Internal {
                message: format!("call callee {callee_node:?} decided as {other:?}"),
            }),
            // named and member callees must resolve before call selection
            None => Ok(Answer::pending([Dependency::Decision(callee_node)])),
        }
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
                    receiver: None,
                    ty: reduced,
                    generic_arguments: Vec::new(),
                });

                Ok(Answer::Ready(Some(CallCandidates::Any(candidates))))
            }
            dir::Type::Variable(variable) => {
                let representative = self.solver.representative(variable)?;

                Ok(Answer::pending([Dependency::Variable(representative)]))
            }
            _ => Ok(Answer::Ready(Some(CallCandidates::Any(candidates)))),
        }
    }

    /// Attempt one callable candidate against collected arguments.
    fn attempt_call(
        &mut self,
        origin: Origin,
        module: ModuleId,
        candidate: &CallableCandidate,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Result<CallSelection, SignatureRejection>>> {
        let argument_sources = self.argument_value_sources(module, argument_nodes);
        let attempt = self.attempt_callable(
            origin,
            candidate.ty,
            candidate.receiver,
            &candidate.generic_arguments,
            type_arguments,
            arguments,
            &argument_sources,
        )?;

        let signature = match answer!(attempt) {
            Ok(signature) => signature,
            Err(rejection) => return Ok(Answer::Ready(Err(rejection))),
        };
        let generic_arguments = candidate.call_generic_arguments(&signature.generic_arguments);

        // build accepted resolution
        let target = match candidate.symbol {
            Some(symbol) => dir::CallTarget::Symbol(dir::CallCandidate {
                receiver: candidate.receiver,
                symbol,
                generic_arguments,
            }),
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

        Ok(Answer::Ready(Ok(CallSelection {
            resolution,
            return_type: signature.return_type,
        })))
    }

    /// Select one call on a union receiver.
    /// Every variant must match the arguments; the call dispatches at
    /// runtime and joins the variant returns.
    fn select_universal_call(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        origin: Origin,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        candidates: &[CallableCandidate],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let mut targets = Vec::with_capacity(candidates.len());
        let mut returns = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut parameters: Option<SmallVec<[dir::FunctionParameterType; 4]>> = None;
        let argument_sources = self.argument_value_sources(origin.module(), argument_nodes);

        for candidate in candidates {
            let attempt = self.attempt_callable(
                origin,
                candidate.ty,
                candidate.receiver,
                &candidate.generic_arguments,
                type_arguments,
                arguments,
                &argument_sources,
            )?;
            let Ok(signature) = answer!(attempt) else {
                // one rejecting variant rejects the whole union call
                let variant = self.format_type(candidate.ty);
                let note =
                    format!("every union variant must accept the call; '{variant}' does not");
                self.report_no_matching_call(origin, arguments, Some(note))?;
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(()));
            };
            let generic_arguments = candidate.call_generic_arguments(&signature.generic_arguments);

            let Some(symbol) = candidate.symbol else {
                continue;
            };
            targets.push(dir::CallCandidate {
                receiver: candidate.receiver,
                symbol,
                generic_arguments,
            });

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
        answer!(self.push_argument_constraints(site, argument_nodes, &resolution.arguments)?);
        self.commit_decision(node, Decision::Call(resolution))?;

        self.commit_node_type(node, return_type)?;

        Ok(Answer::Ready(()))
    }

    /// Commit one accepted call selection.
    fn commit_call_selection(
        &mut self,
        site: FlowSite,
        node: dir::GlobalNodeIdAny,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        mut selection: CallSelection,
    ) -> CompilerResult<Answer<()>> {
        answer!(self.push_argument_constraints(
            site,
            argument_nodes,
            &selection.resolution.arguments
        )?);

        // preserve exact static-key expressions after normal signature selection
        if let Some(ty) = self.static_key_expression_type(site)? {
            selection.resolution.return_type = ty;
            selection.return_type = ty;
        }

        self.commit_decision(node, Decision::Call(selection.resolution))?;
        self.commit_node_type(node, selection.return_type)?;

        Ok(Answer::Ready(()))
    }
}
