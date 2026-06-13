use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, Condition, Constraint, ConstraintCause, Decision, Dependency,
    GenericTemplateId, Mutation, Origin, Relation, Substitution, Task,
};
use crate::{CheckError, CompilerError, CompilerResult};

/// One callable candidate collected from a callee node.
struct CalleeCandidate {
    /// The declaring symbol, when the callee names one.
    symbol: Option<dir::GlobalSymbolId>,
    /// The resolved receiver type for member callees.
    receiver: Option<dir::GlobalTypeId>,
    /// The callable type.
    ty: dir::GlobalTypeId,
}

/// Callable candidates collected from one callee with their
/// acceptance rule.
struct Callees {
    /// The candidates in declaration order.
    candidates: SmallVec<[CalleeCandidate; 2]>,
    /// Whether every candidate must accept the call: union receivers
    /// dispatch at runtime, so the call must hold for every variant.
    universal: bool,
}

impl Callees {
    /// Collect candidates that one of them may accept.
    fn any(candidates: SmallVec<[CalleeCandidate; 2]>) -> Self {
        Self {
            candidates,
            universal: false,
        }
    }

    /// Collect candidates that all must accept.
    fn every(candidates: SmallVec<[CalleeCandidate; 2]>) -> Self {
        Self {
            candidates,
            universal: true,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl CheckState<'_> {
    /// Select the callable meaning of one call node.
    pub(in crate::check) fn select_call(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        callee: dir::LocalNodeId<dir::Expression>,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let node = node.into_any();
        let origin = Origin::Node(node);

        // collect argument types from walked inputs
        let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for argument in argument_nodes {
            let argument = argument.into_global_any(module);
            let Some(ty) = self.inputs.node_type(argument) else {
                return Err(CompilerError::Internal {
                    message: format!("call argument {argument:?} has no input type"),
                });
            };
            arguments.push(ty);
        }

        // collect callable candidates from the callee
        let callees = match self.call_candidates(origin, module, callee)? {
            Answer::Ready(Some(callees)) => callees,
            // rejected callees fail silently to avoid cascading diagnostics
            Answer::Ready(None) => {
                self.record_decision(node, Decision::Rejected)?;

                return Ok(Answer::Ready(()));
            }
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let candidates = callees.candidates;
        if candidates.is_empty() {
            let callee_type = self
                .inputs
                .node_type(callee.into_global_any(module))
                .map(|ty| self.format_type(ty))
                .unwrap_or_else(|| "unknown".to_string());
            let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
            let error = CheckError::NotCallable {
                anchor,
                module,
                ty: callee_type,
            };
            self.module_mut(module).diagnostics.push(error.into());
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
                return self.select_newtype_construct(node, origin, symbol, &arguments);
            }
        }

        // union receivers must hold for every variant
        if callees.universal {
            return self.select_union_call(node, origin, &candidates, &arguments);
        }

        // try candidates in declaration order
        for candidate in &candidates {
            let attempt = self.attempt_call(
                origin,
                node,
                candidate.symbol,
                candidate.receiver,
                candidate.ty,
                &arguments,
            )?;

            match attempt {
                Answer::Ready(true) => return Ok(Answer::Ready(())),
                Answer::Ready(false) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        // a sole candidate blames the failing argument directly
        if let [candidate] = candidates.as_slice()
            && self.blame_sole_candidate(origin, candidate.symbol, candidate.ty, &arguments)?
        {
            self.record_decision(node, Decision::Rejected)?;

            return Ok(Answer::Ready(()));
        }

        // no candidate accepted the arguments
        let arguments = self.format_types(&arguments);
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = CheckError::NoMatchingCall {
            anchor,
            module,
            arguments,
        };
        self.module_mut(module).diagnostics.push(error.into());
        self.record_decision(node, Decision::Rejected)?;

        Ok(Answer::Ready(()))
    }

    /// Blame the failing arguments of one sole rejected candidate.
    /// Returns whether blame was assigned; generic candidates decline
    /// because their unsubstituted parameters would mislead.
    fn blame_sole_candidate(
        &mut self,
        origin: Origin,
        symbol: Option<dir::GlobalSymbolId>,
        function_type: dir::GlobalTypeId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        if symbol
            .and_then(|symbol| self.generics.template_by_symbol(symbol))
            .is_some()
        {
            return Ok(false);
        }

        // read the closed callable shape, following closures
        let mut function_type = match self.evaluate_root(origin, function_type)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(_) => return Ok(false),
        };
        if let dir::Type::Closure(closure) = self.ty(function_type)? {
            function_type = closure.function;
        }
        let dir::Type::Function(function) = self.ty(function_type)? else {
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
            let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
            let error = CheckError::WrongArgumentCount {
                anchor,
                module,
                expected,
                supplied: arguments.len(),
            };
            self.module_mut(module).diagnostics.push(error.into());

            return Ok(true);
        }

        // re-relate each argument loudly under the argument cause
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = parameters.get(index).or_else(|| parameters.last());
            let Some(parameter) = parameter else {
                return Ok(false);
            };

            self.push_constraint(Constraint {
                relation: Relation::Assignable,
                left: *argument,
                right: parameter.ty,
                origin,
                condition: Condition::Always,
                cause: ConstraintCause::Argument,
            });
        }

        Ok(true)
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
    ) -> CompilerResult<Answer<Option<Callees>>> {
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

        match self.decisions.get(callee_node) {
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
                    match self.decide_availability(symbol)? {
                        Answer::Ready(true) => {}
                        Answer::Ready(false) => continue,
                        Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                    }

                    if let Some(ty) = self.symbol_type(symbol) {
                        candidates.push(CalleeCandidate {
                            symbol: Some(symbol),
                            receiver: None,
                            ty,
                        });
                    }
                }

                Ok(Answer::Ready(Some(Callees::any(candidates))))
            }
            Some(Decision::Member(resolution)) => match &resolution.target {
                // member candidates carry their receiver-applied types
                dir::MemberTarget::Symbol(candidate) => {
                    let mut candidates = SmallVec::new();
                    candidates.push(CalleeCandidate {
                        symbol: Some(candidate.symbol),
                        receiver: Some(resolution.receiver),
                        ty: candidate.ty,
                    });

                    Ok(Answer::Ready(Some(Callees::any(candidates))))
                }
                // one overload accepts; union members must all accept
                dir::MemberTarget::Overloaded(overloads) | dir::MemberTarget::Union(overloads) => {
                    let universal = matches!(resolution.target, dir::MemberTarget::Union(_));
                    let receiver = resolution.receiver;
                    let candidates = overloads
                        .iter()
                        .map(|candidate| CalleeCandidate {
                            symbol: Some(candidate.symbol),
                            receiver: Some(receiver),
                            ty: candidate.ty,
                        })
                        .collect::<SmallVec<[_; 2]>>();

                    Ok(Answer::Ready(Some(if universal {
                        Callees::every(candidates)
                    } else {
                        Callees::any(candidates)
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
            // wait for the callee's own selection
            None => Ok(Answer::pending([Dependency::Decision(callee_node)])),
        }
    }

    /// Collect the callable candidate behind one function-typed callee value.
    fn value_call_candidates(
        &mut self,
        origin: Origin,
        callee: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Answer<Option<Callees>>> {
        let Some(ty) = self.inputs.node_type(callee) else {
            return Err(CompilerError::Internal {
                message: format!("call callee {callee:?} has no input type"),
            });
        };
        let reduced = match self.evaluate_root(origin, ty)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        let mut candidates = SmallVec::new();
        match self.ty(reduced)? {
            dir::Type::Function(_) | dir::Type::Closure(_) => {
                candidates.push(CalleeCandidate {
                    symbol: None,
                    receiver: None,
                    ty: reduced,
                });

                Ok(Answer::Ready(Some(Callees::any(candidates))))
            }
            dir::Type::Variable(variable) => {
                let representative = self.variables.representative(*variable)?;

                Ok(Answer::pending([Dependency::Variable(representative)]))
            }
            _ => Ok(Answer::Ready(Some(Callees::any(candidates)))),
        }
    }

    /// Attempt one callable candidate against collected arguments,
    /// recording the call resolution when it accepts.
    #[allow(clippy::too_many_arguments)]
    fn attempt_call(
        &mut self,
        origin: Origin,
        node: dir::GlobalNodeIdAny,
        symbol: Option<dir::GlobalSymbolId>,
        receiver: Option<dir::GlobalTypeId>,
        function_type: dir::GlobalTypeId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        let attempt = self.attempt_callable(origin, symbol, function_type, arguments)?;

        match attempt {
            // record the accepted resolution
            Answer::Ready(Some((parameters, return_type))) => {
                let target = match symbol {
                    Some(symbol) => dir::CallTarget::Symbol(dir::CallCandidate {
                        receiver,
                        symbol,
                        arguments: Vec::new(),
                    }),
                    None => dir::CallTarget::Expression {
                        arguments: Vec::new(),
                    },
                };
                let resolution = dir::CallResolution::new(target, parameters, return_type);
                self.record_decision(node, Decision::Call(resolution))?;

                // flow the return type into the call node variable
                if let Some(variable) = self.node_variable(node)? {
                    self.push_lower_bound(variable, return_type)?;
                }

                Ok(Answer::Ready(true))
            }
            Answer::Ready(None) => Ok(Answer::Ready(false)),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Attempt one callable candidate without recording a decision.
    /// Returns the solved parameters and return type when it accepts,
    /// keeping the winning hypothesis.
    fn attempt_callable(
        &mut self,
        origin: Origin,
        symbol: Option<dir::GlobalSymbolId>,
        function_type: dir::GlobalTypeId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<(Vec<dir::GlobalTypeId>, dir::GlobalTypeId)>>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // close the callable shape first
        let function_type = match self.evaluate_root(origin, function_type)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let (parameters, return_type) = match self.ty(function_type)? {
            dir::Type::Function(function) => (
                function
                    .parameters
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>(),
                function.return_type,
            ),
            dir::Type::Closure(closure) => {
                let function = closure.function;

                return self.attempt_callable(origin, symbol, function, arguments);
            }
            _ => return Ok(Answer::Ready(None)),
        };

        // hypothesize generic parameters under a probe
        let template = symbol.and_then(|symbol| self.generics.template_by_symbol(symbol));
        let probe = self.begin_probe();
        let attempt = self.attempt_signature(
            origin,
            module,
            source,
            template,
            &parameters,
            return_type,
            arguments,
        )?;

        match attempt {
            // keep the winning hypothesis
            Answer::Ready(Some((parameters, return_type))) => {
                self.keep_probe(probe)?;

                Ok(Answer::Ready(Some((parameters.to_vec(), return_type))))
            }
            // roll back failed hypotheses
            Answer::Ready(None) => {
                self.unwind_probe(probe)?;

                Ok(Answer::Ready(None))
            }
            Answer::Pending(blockers) => {
                self.unwind_probe(probe)?;

                // blockers that died with the probe cannot wake this candidate
                let blockers = self.surviving_blockers(blockers);
                if blockers.is_empty() {
                    Ok(Answer::Ready(None))
                } else {
                    Ok(Answer::Pending(blockers))
                }
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
        candidates: &[CalleeCandidate],
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<()>> {
        let mut targets = Vec::with_capacity(candidates.len());
        let mut returns = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        let mut parameters: Option<Vec<dir::GlobalTypeId>> = None;

        for candidate in candidates {
            let attempt =
                self.attempt_callable(origin, candidate.symbol, candidate.ty, arguments)?;
            let (solved, return_type) = match attempt {
                Answer::Ready(Some(signature)) => signature,
                // one rejecting variant rejects the whole union call
                Answer::Ready(None) => {
                    let variant = self.format_type(candidate.ty);
                    let arguments = self.format_types(arguments);
                    let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
                    let error = CheckError::NoMatchingCall {
                        anchor,
                        module,
                        arguments,
                    };
                    let note =
                        format!("every union variant must accept the call; '{variant}' does not");
                    self.module_mut(module).diagnostics.push(error.note(note));
                    self.record_decision(node, Decision::Rejected)?;

                    return Ok(Answer::Ready(()));
                }
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

            let Some(symbol) = candidate.symbol else {
                continue;
            };
            targets.push(dir::CallCandidate {
                receiver: candidate.receiver,
                symbol,
                arguments: Vec::new(),
            });
            // join by content: variant returns allocate distinct ids
            let return_type = self.resolve_root(return_type)?;
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
            parameters.get_or_insert(solved);
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
            dir::CallTarget::Union(targets),
            parameters.unwrap_or_default(),
            return_type,
        );
        self.record_decision(node, Decision::Call(resolution))?;

        // flow the joined return into the call node variable
        if let Some(variable) = self.node_variable(node)? {
            self.push_lower_bound(variable, return_type)?;
        }

        Ok(Answer::Ready(()))
    }

    /// Attempt one function signature under an active probe.
    pub(in crate::check) fn attempt_signature(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        template: Option<GenericTemplateId>,
        function_parameters: &[dir::FunctionParameterType],
        function_return: Option<dir::GlobalTypeId>,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<(SmallVec<[dir::GlobalTypeId; 4]>, dir::GlobalTypeId)>>> {
        // reject arities the signature cannot accept
        let required = function_parameters
            .iter()
            .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
            .count();
        let has_rest = function_parameters
            .iter()
            .any(|parameter| parameter.is_rest);
        if arguments.len() < required || (!has_rest && arguments.len() > function_parameters.len())
        {
            return Ok(Answer::Ready(None));
        }

        // hypothesize the signature's generic parameters
        let substitution = match template {
            Some(template) => self.instantiate_template(origin, template)?,
            None => Default::default(),
        };

        // relate each argument into its substituted parameter
        let mut parameters = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for (index, argument) in arguments.iter().enumerate() {
            let parameter = function_parameters
                .get(index)
                .or_else(|| function_parameters.last());
            let Some(parameter) = parameter else {
                return Ok(Answer::Ready(None));
            };
            let parameter_type = if substitution.is_empty() {
                parameter.ty
            } else {
                self.fold_type(module, source, parameter.ty, substitution.rewrite())?
            };

            match self.constrain(origin, Relation::Assignable, *argument, parameter_type)? {
                Answer::Ready(true) => parameters.push(parameter_type),
                // rejected arguments fail the candidate silently
                Answer::Ready(false) => return Ok(Answer::Ready(None)),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        // solve hypothesized parameters from their argument bounds
        let floor = self.queue.solve_count();
        for variable_type in substitution.arguments.iter().copied() {
            if let Some(variable) = self.root_variable(variable_type)? {
                self.queue_task(Task::Solve(variable));
            }
        }
        // reject candidates whose inferred hypotheses violate their constraints
        if !self.drain_probe_tasks(floor)? {
            return Ok(Answer::Ready(None));
        }
        self.close_hypotheses(module, source, &substitution)?;

        // harvest the substituted return type
        let return_type = match function_return {
            Some(return_type) => {
                let return_type = if substitution.is_empty() {
                    return_type
                } else {
                    self.fold_type(module, source, return_type, substitution.rewrite())?
                };

                self.harvest_type(module, source, return_type)?
            }
            None => self.push_type(module, dir::Type::Void, source)?,
        };
        let parameters = parameters
            .into_iter()
            .map(|parameter| self.harvest_type(module, source, parameter))
            .collect::<CompilerResult<SmallVec<[_; 4]>>>()?;

        Ok(Answer::Ready(Some((parameters, return_type))))
    }

    /// Drop pending dependencies that died with an unwound probe.
    pub(in crate::check) fn surviving_blockers(
        &self,
        blockers: SmallVec<[Dependency; 2]>,
    ) -> SmallVec<[Dependency; 2]> {
        blockers
            .into_iter()
            .filter(|blocker| match blocker {
                Dependency::Variable(variable) => self.variables.get(*variable).is_ok(),
                Dependency::Decision(_) => true,
            })
            .collect()
    }

    /// Close leftover unsolved hypothesis variables to unknown.
    /// Harvested types must never reference variables the probe unwinds.
    pub(in crate::check) fn close_hypotheses(
        &mut self,
        module: destack_source::ModuleId,
        source: dir::LocalNodeIdAny,
        substitution: &Substitution,
    ) -> CompilerResult<()> {
        for variable_type in substitution.arguments.iter().copied() {
            let Some(variable) = self.root_variable(variable_type)? else {
                continue;
            };

            // uninferable hypotheses close to unknown
            if self.variables.solution(variable)?.is_none() {
                let unknown = self.push_type(module, dir::Type::Unknown, source)?;
                self.set_solution(variable, unknown)?;
            }
        }

        Ok(())
    }

    /// Run solve tasks queued above one floor to quiescence inside one
    /// probe. Returns whether every solved hypothesis met its upper
    /// bounds. The floor keeps the drain probe-scoped: solve tasks the
    /// outer loop queued before the probe stay untouched.
    pub(in crate::check) fn drain_probe_tasks(&mut self, floor: usize) -> CompilerResult<bool> {
        // bounded drain keeps candidate testing terminating; only the
        // solve class drains, other task classes stay queued untouched
        let mut budget = 1024usize;
        let mut consistent = true;
        while budget > 0 {
            let Some(task) = self.queue.pop_solve_above(floor) else {
                break;
            };
            self.journal.record(Mutation::TaskPopped { task });

            if let Task::Solve(variable) = task {
                consistent &= self.solve_probe_variable(variable)?;
            }
            budget -= 1;
        }

        Ok(consistent)
    }

    /// Solve one hypothesized variable from its bounds inside a probe.
    /// Returns whether the solution met the variable's upper bounds.
    fn solve_probe_variable(&mut self, variable: dir::TypeVariableId) -> CompilerResult<bool> {
        let representative = self.variables.representative(variable)?;

        // join available lower bounds directly
        let state = self.variables.get(representative)?;
        if state.solution.is_some() {
            return Ok(true);
        }
        let lower = state.lower.clone();
        if lower.is_empty() {
            return Ok(true);
        }
        let origin = state.origin;
        let upper = state.upper.clone();

        let joined = self.best_common(representative, &lower)?;
        self.set_solution(representative, joined)?;

        // check the solution against seeded constraint bounds
        for bound in upper {
            match self.decide_relation(origin, Relation::Assignable, joined, bound)? {
                Answer::Ready(true) => {}
                // violated constraints fail the candidate
                Answer::Ready(false) => return Ok(false),
                // undecidable bounds re-check outside the probe
                Answer::Pending(_) => {
                    self.push_constraint(Constraint {
                        relation: Relation::Assignable,
                        left: joined,
                        right: bound,
                        origin,
                        condition: Condition::Always,
                        cause: ConstraintCause::General,
                    });
                }
            }
        }

        Ok(true)
    }

    /// Return the open variable behind one node input.
    pub(in crate::check) fn node_variable(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<dir::TypeVariableId>> {
        let Some(input) = self.inputs.node_type(node) else {
            return Ok(None);
        };

        self.root_variable(input)
    }

    /// Return one symbol's walked or committed type.
    fn symbol_type(&self, symbol: dir::GlobalSymbolId) -> Option<dir::GlobalTypeId> {
        // prefer component inputs over committed tables
        if let Some(ty) = self.inputs.symbol_type(symbol) {
            return Some(ty);
        }

        // read external committed symbol types
        if let Some(external) = self.external_modules.get(&symbol.module_id) {
            return external.types.get_symbol_type_id(symbol);
        }

        None
    }
}
