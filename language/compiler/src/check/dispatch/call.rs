use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CallCallee, CallFailure, CallInstantiation, CallInstantiationArgumentKey, CallInstantiationKey,
    CallResolutionTarget, CallSelection, CallTerm, CheckState, ConstructCandidates, Decision,
    FunctionParameter, FunctionTerm, GenericArgument, GenericInstance, GenericParameter,
    GenericSlot, GenericSubstitution, GenericSubstitutionEntry, MemberCallCallee, Origin, Progress,
    ShapeMember, StaticTerm, SymbolCandidate, TypeLiteralTerm, TypeOperand, TypeRelation, TypeTerm,
    VariableId, VariableKind, VariableOutput,
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
    /// The already resolved generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The final call target if this candidate is selected.
    pub(in crate::check) target: CallTarget,
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

/// Transient callable selection while reducing call-like syntax.
pub(in crate::check) enum CallableSelection {
    /// Call resolution is waiting for solver input.
    Pending {
        /// The variable changes made before resolution became pending.
        progress: Progress,
    },
    /// Call resolution failed.
    Rejected(CallFailure),
    /// One callable candidate resolved.
    Resolved {
        /// The resolved call target.
        target: CallResolutionTarget,
        /// The resolved function signature.
        function: FunctionTerm,
        /// The variable changes made while resolving.
        progress: Progress,
    },
}

impl CallableSelection {
    /// Return a pending selection with no side progress.
    pub(in crate::check) fn pending() -> Self {
        Self::Pending {
            progress: Progress::Unchanged,
        }
    }

    /// Return a resolved selection with side progress.
    pub(in crate::check) fn resolved(
        target: CallResolutionTarget,
        function: FunctionTerm,
        progress: Progress,
    ) -> Self {
        Self::Resolved {
            target,
            function,
            progress,
        }
    }

    /// Return a rejected selection.
    pub(in crate::check) fn rejected(failure: CallFailure) -> Self {
        Self::Rejected(failure)
    }

    /// Return progress made while selecting.
    pub(in crate::check) fn progress(&self) -> Progress {
        match self {
            Self::Pending { progress } | Self::Resolved { progress, .. } => progress.clone(),
            Self::Rejected(_) => Progress::Unchanged,
        }
    }
}

/// Callable target selected before generic instantiation.
#[derive(Clone)]
pub(in crate::check) enum CallTarget {
    /// Callable value without a declaration symbol.
    Value,
    /// Symbol-backed callable selected at compile time.
    Symbol {
        /// The resolved callable symbol.
        symbol: dir::GlobalSymbolId,
        /// The resolved receiver type for method calls.
        receiver: Option<VariableId>,
    },
    /// Symbol-backed callable variants selected from a union receiver.
    Select {
        /// The resolved callable candidates.
        candidates: Vec<SymbolCandidate>,
        /// The resolved receiver type for method calls.
        receiver: Option<VariableId>,
    },
    /// Constructor selected through call syntax.
    Constructor {
        /// The resolved constructor symbol.
        symbol: dir::GlobalSymbolId,
    },
}

impl CallTarget {
    /// Return the concrete target symbol when this target has one.
    fn symbol(&self) -> Option<dir::GlobalSymbolId> {
        match self {
            Self::Symbol { symbol, .. } | Self::Constructor { symbol } => Some(*symbol),
            Self::Value | Self::Select { .. } => None,
        }
    }

    /// Convert this selected target into a resolved call target.
    pub(in crate::check) fn into_resolution_target(
        self,
        instance: Option<GenericInstance>,
    ) -> CallResolutionTarget {
        match self {
            Self::Value => CallResolutionTarget::Value,
            Self::Symbol { symbol, receiver } => CallResolutionTarget::Symbol {
                symbol,
                instance,
                receiver,
            },
            Self::Select {
                candidates,
                receiver,
            } => CallResolutionTarget::Select {
                candidates,
                receiver,
            },
            Self::Constructor { symbol } => CallResolutionTarget::Constructor { symbol, instance },
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
    ) -> CompilerResult<CallableSelection> {
        if let Some(selection) = self.selected_call(call)? {
            return Ok(selection);
        }
        if let Some(result) = self.select_member_call(origin, call, expected)? {
            return Ok(result);
        }
        if let Some(result) = self.select_reference_call(origin, call, expected)? {
            return Ok(result);
        }

        self.select_value_call(origin, call, expected)
    }

    /// Return the already chosen decision for one call.
    fn selected_call(&self, call: &CallTerm) -> CompilerResult<Option<CallableSelection>> {
        let Some(decision) = self.solutions.call.get(&call.source).cloned() else {
            return Ok(None);
        };

        let selection = match decision {
            CallSelection::Resolved(resolution) => CallableSelection::resolved(
                resolution.target,
                resolution.function,
                Progress::Unchanged,
            ),
            CallSelection::Rejected(failure) => CallableSelection::rejected(failure),
        };

        Ok(Some(selection))
    }

    /// Select a call whose callee is an arbitrary value expression.
    fn select_value_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableSelection> {
        let CallCallee::Value(callee) = call.callee else {
            panic!("member call reached value dispatch");
        };
        let Some(term) = self.solved_type_term(callee)? else {
            return Ok(CallableSelection::pending());
        };
        let function = match self.call_signature(callee.module, &term)? {
            CallableSignature::Pending => return Ok(CallableSelection::pending()),
            CallableSignature::Absent => {
                return Ok(CallableSelection::rejected(CallFailure::NotCallable));
            }
            CallableSignature::Present(function) => function,
        };

        self.select_call_signature(
            origin,
            callee.module,
            call.source,
            None,
            None,
            function,
            &call.generic_arguments,
            &call.arguments,
            expected,
            CallTarget::Value,
        )
    }

    /// Select a call whose callee is a symbol-backed reference.
    fn select_reference_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableSelection>> {
        let CallCallee::Value(callee) = call.callee else {
            return Ok(None);
        };
        let Some(term) = self.solved_type_term(callee)? else {
            return Ok(Some(CallableSelection::pending()));
        };
        let TypeTerm::Reference {
            origin: _,
            symbol,
            arguments,
        } = term
        else {
            return Ok(None);
        };

        if self.environment.language.item(symbol) == Some(dir::LanguageItem::Symbol) {
            let result = self.select_symbol_call(origin, call, expected, symbol)?;

            return Ok(Some(result));
        }

        if self.symbol_kind(callee.module, symbol) == Some(dir::SymbolKind::Newtype) {
            return self.select_newtype_call(origin, call, expected, symbol, &arguments);
        }

        let ty = self.symbol_type_variable(callee.module, symbol);
        let Some(term) = self.solved_type_term(ty)? else {
            return Ok(Some(CallableSelection::pending()));
        };
        let function = match self.call_signature(ty.module, &term)? {
            CallableSignature::Pending => return Ok(Some(CallableSelection::pending())),
            CallableSignature::Absent => return Ok(None),
            CallableSignature::Present(function) => function,
        };
        let result = self.select_call_signature(
            origin,
            callee.module,
            call.source,
            Some(symbol),
            None,
            function,
            &call.generic_arguments,
            &call.arguments,
            expected,
            CallTarget::Symbol {
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
    ) -> CompilerResult<CallableSelection> {
        let string = self
            .terms
            .push(TypeTerm::Literal(TypeLiteralTerm::Primitive(
                dir::PrimitiveType::String,
            )));
        let number = self
            .terms
            .push(TypeTerm::Literal(TypeLiteralTerm::Primitive(
                dir::PrimitiveType::Float(dir::FloatType::Float64),
            )));
        let description = self.terms.push(TypeTerm::Union {
            elements: vec![string.into(), number.into()],
        });
        let result = self
            .terms
            .push(TypeTerm::Literal(TypeLiteralTerm::Primitive(
                dir::PrimitiveType::Symbol,
            )));
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: SmallVec::new(),
            this_parameter: None,
            parameters: SmallVec::from_vec(vec![FunctionParameter {
                ty: description.into(),
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
            expected,
            CallTarget::Symbol {
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
    ) -> CompilerResult<Option<CallableSelection>> {
        let module = call.source.module_id;
        let callee = self.symbol_type_variable(module, symbol);
        let term = TypeTerm::Reference {
            origin: Origin::Node(call.source),
            symbol,
            arguments: arguments.to_vec().into(),
        };
        let candidates = match self.construct_candidates(module, callee, &term)? {
            ConstructCandidates::Pending => return Ok(Some(CallableSelection::pending())),
            ConstructCandidates::Absent => return Ok(None),
            ConstructCandidates::Present(candidates) => candidates,
        };
        let mut saw_pending = false;

        // choose the first compatible declaration-order newtype candidate
        for candidate in candidates {
            let target_symbol = match candidate.symbol {
                Some(symbol) => symbol,
                None => symbol,
            };
            let result = self.select_call_signature(
                origin,
                module,
                call.source,
                Some(symbol),
                candidate.instance,
                candidate.function,
                &call.generic_arguments,
                &call.arguments,
                expected,
                CallTarget::Constructor {
                    symbol: target_symbol,
                },
            )?;
            match result {
                CallableSelection::Resolved { .. } => return Ok(Some(result)),
                CallableSelection::Pending { .. } => saw_pending = true,
                CallableSelection::Rejected(_) => {}
            }
        }

        let result = if saw_pending {
            CallableSelection::pending()
        } else {
            CallableSelection::rejected(CallFailure::NoMatch)
        };

        Ok(Some(result))
    }

    /// Select a call whose callee is a member projection.
    fn select_member_call(
        &mut self,
        origin: Origin,
        call: &CallTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<Option<CallableSelection>> {
        let CallCallee::Member(member) = call.callee else {
            return Ok(None);
        };
        let member = self.terms.get(member).clone();
        let Some(receiver) = self.solved_type_term(member.receiver)? else {
            return Ok(Some(CallableSelection::pending()));
        };
        let member_match = match &member.callee {
            MemberCallCallee::Source { .. } => {
                self.member_type_candidates(origin, member.receiver.module, &receiver, &member.key)?
            }
            MemberCallCallee::Protocol { protocol } => self.member_type_candidates_for_protocol(
                origin,
                member.receiver.module,
                &receiver,
                &member.key,
                protocol,
            )?,
        };
        let Some(member_matches) = member_match else {
            return Ok(Some(CallableSelection::pending()));
        };
        let dispatch_candidates = member_matches
            .iter()
            .map(|member_match| {
                let target = if member_matches.len() == 1 {
                    CallTarget::Symbol {
                        symbol: member_match.symbol,
                        receiver: Some(member.receiver),
                    }
                } else {
                    let candidates = member_matches
                        .iter()
                        .map(|candidate| SymbolCandidate {
                            symbol: candidate.symbol,
                            instance: candidate.instance.clone(),
                        })
                        .collect();

                    CallTarget::Select {
                        candidates,
                        receiver: Some(member.receiver),
                    }
                };

                CallableCandidate {
                    module: member.receiver.module,
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
            &call.arguments,
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
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        expected: Option<VariableId>,
        target: CallTarget,
    ) -> CompilerResult<CallableSelection> {
        let decision = self.decide_call_signature(
            instance.as_ref(),
            &function,
            generic_arguments,
            arguments,
            expected,
        )?;
        match decision {
            CallCandidateDecision::Applicable => {}
            CallCandidateDecision::Pending => return Ok(CallableSelection::pending()),
            CallCandidateDecision::Rejected(failure) => {
                return Ok(CallableSelection::rejected(failure));
            }
        }

        self.commit_call_signature(
            origin,
            module,
            source,
            owner,
            instance,
            function,
            generic_arguments,
            arguments,
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
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        expected: Option<VariableId>,
        target: CallTarget,
    ) -> CompilerResult<CallableSelection> {
        let is_generic = !function.generic_parameters.is_empty();
        if generic_arguments.len() > function.generic_parameters.len()
            || (!is_generic && !generic_arguments.is_empty())
        {
            return Ok(CallableSelection::rejected(CallFailure::NoMatch));
        }
        let instantiation = self.instantiate_call_signature(
            module,
            source,
            owner,
            target.symbol(),
            instance,
            function,
            generic_arguments,
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
            self.solve_defaulted_generic_arguments(
                module,
                &instantiation.substitution,
                &instantiation.generic_parameters,
                generic_arguments.len(),
            )?;
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
                let target = target.into_resolution_target(instantiation.instance);

                Ok(CallableSelection::resolved(target, function, progress))
            }
            Decision::Undecidable => Ok(CallableSelection::Pending { progress }),
            Decision::No if !progress.is_unchanged() => Ok(CallableSelection::Pending { progress }),
            Decision::No => {
                if let Some((argument, parameter)) =
                    self.call_argument_type_failure(arguments, &function.parameters)?
                {
                    Ok(CallableSelection::rejected(CallFailure::ArgumentType {
                        argument,
                        parameter,
                    }))
                } else {
                    Ok(CallableSelection::rejected(CallFailure::NoMatch))
                }
            }
        }
    }

    /// Decide whether one callable signature can accept a call.
    fn decide_call_signature(
        &self,
        instance: Option<&GenericInstance>,
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

        let substitution = self.candidate_substitution(instance, function, generic_arguments)?;
        let mut inference = CallCandidateInference::empty();
        let arguments_decision = self.decide_candidate_arguments(
            arguments,
            &function.parameters,
            &substitution,
            &mut inference,
        )?;
        let return_decision =
            self.decide_candidate_return(function, expected, &substitution, &inference)?;
        let decision = arguments_decision.and(return_decision);

        match decision {
            Decision::Yes => Ok(CallCandidateDecision::Applicable),
            Decision::Undecidable => Ok(CallCandidateDecision::Pending),
            Decision::No => {
                if let Some((argument, parameter)) = self.candidate_argument_type_failure(
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
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableSelection> {
        let mut callable_count = 0;
        let mut saw_pending = false;
        let mut argument_failure = None;

        // choose the first compatible declaration-order candidate
        for candidate in candidates {
            let reduction = self.reduce_type_term(origin, &candidate.ty)?;
            let Some(term) = reduction.value else {
                saw_pending = true;
                continue;
            };
            let function = match self.call_signature(candidate.module, &term)? {
                CallableSignature::Pending => {
                    saw_pending = true;
                    continue;
                }
                CallableSignature::Absent => continue,
                CallableSignature::Present(function) => function,
            };
            callable_count += 1;
            let result = self.select_call_signature(
                origin,
                candidate.module,
                source,
                Some(candidate.symbol),
                candidate.instance.clone(),
                function,
                generic_arguments,
                arguments,
                expected,
                candidate.target.clone(),
            )?;

            match result {
                CallableSelection::Resolved { .. } => return Ok(result),
                CallableSelection::Pending { .. } => saw_pending = true,
                CallableSelection::Rejected(CallFailure::ArgumentType {
                    argument,
                    parameter,
                }) => {
                    argument_failure.get_or_insert((argument, parameter));
                }
                CallableSelection::Rejected(CallFailure::NoMatch) => {}
                CallableSelection::Rejected(CallFailure::NotCallable) => {}
            }
        }

        // wait for unresolved candidate type input
        if saw_pending {
            return Ok(CallableSelection::pending());
        }

        // preserve the precise single-candidate argument error
        if callable_count == 1
            && let Some((argument, parameter)) = argument_failure
        {
            return Ok(CallableSelection::rejected(CallFailure::ArgumentType {
                argument,
                parameter,
            }));
        }

        // distinguish rejected callable overloads from non-callable values
        if callable_count > 0 {
            Ok(CallableSelection::rejected(CallFailure::NoMatch))
        } else {
            Ok(CallableSelection::rejected(CallFailure::NotCallable))
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
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(CallableSignature::Pending);
                };

                return self.call_signature(variable.module, &term);
            }
            TypeTerm::Function(function) => {
                CallableSignature::Present(self.terms.get(*function).clone())
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
        let Some(term) = self.solved_type_term(variable)? else {
            return Ok(None);
        };
        let parameters = match term {
            TypeTerm::Tuple { elements, .. } => elements
                .iter()
                .map(|element| FunctionParameter {
                    ty: element.ty,
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
        instance: Option<&GenericInstance>,
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
                instance
                    .filter(|instance| instance.symbol == slot.owner)
                    .and_then(|instance| instance.arguments.get(slot.index.0 as usize))
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
        argument: TypeOperand,
        parameter: TypeOperand,
        substitution: &CallCandidateSubstitution,
        inference: &mut CallCandidateInference,
    ) -> CompilerResult<Decision> {
        if let Some(target) = self.candidate_type_argument(parameter, substitution)? {
            return self.decide_type_relation(TypeRelation::Assignable, argument, target);
        }
        if let Some(parameter) = self.candidate_type_parameter(parameter)? {
            if self.type_operand_term(argument)?.is_some() {
                inference.add_type_lower(parameter, argument);

                return Ok(Decision::Yes);
            }

            return Ok(Decision::Undecidable);
        }

        self.decide_type_relation(TypeRelation::Assignable, argument, parameter)
    }

    /// Return the substituted type argument for one direct generic parameter.
    fn candidate_type_argument(
        &self,
        parameter: TypeOperand,
        substitution: &CallCandidateSubstitution,
    ) -> CompilerResult<Option<TypeOperand>> {
        let Some(parameter) = parameter.variable() else {
            return Ok(None);
        };
        let Some(argument) = substitution.type_argument(parameter) else {
            return Ok(None);
        };

        Ok(Some(argument))
    }

    /// Return the generic parameter represented by one direct type operand.
    fn candidate_type_parameter(
        &self,
        parameter: TypeOperand,
    ) -> CompilerResult<Option<VariableId>> {
        let Some(parameter) = parameter.variable() else {
            return Ok(None);
        };
        let variable = self.variable(parameter);
        let Some(VariableOutput::Generic(generic)) = &variable.output else {
            return Ok(None);
        };
        if !generic.is_type() {
            return Ok(None);
        }

        Ok(Some(parameter))
    }

    /// Decide whether a candidate return type can flow into the expected result.
    fn decide_candidate_return(
        &self,
        function: &FunctionTerm,
        expected: Option<VariableId>,
        substitution: &CallCandidateSubstitution,
        inference: &CallCandidateInference,
    ) -> CompilerResult<Decision> {
        let Some(expected) = expected else {
            return Ok(Decision::Yes);
        };
        let Some(return_type) = function.return_type else {
            let Some(expected) = self.solved_type_term(expected)? else {
                return Ok(Decision::Undecidable);
            };
            let void = TypeTerm::Literal(TypeLiteralTerm::Void);

            return self.decide_type_term_relation(TypeRelation::Assignable, &void, &expected);
        };
        if let Some(return_type) = self.candidate_type_argument(return_type, substitution)? {
            return self.decide_type_relation(TypeRelation::Assignable, return_type, expected);
        }
        if let Some(parameter) = self.candidate_type_parameter(return_type)?
            && let Some(inferred) = inference.type_lower(parameter)
        {
            return self.decide_type_relation(TypeRelation::Assignable, inferred, expected);
        }

        self.decide_type_relation(TypeRelation::Assignable, return_type, expected)
    }

    /// Return the first solved candidate argument type failure.
    fn candidate_argument_type_failure(
        &self,
        arguments: &[TypeOperand],
        parameters: &[FunctionParameter],
        substitution: &CallCandidateSubstitution,
    ) -> CompilerResult<Option<(TypeOperand, TypeOperand)>> {
        if !self.call_arity_accepts(arguments, parameters) {
            return Ok(None);
        }

        for (argument, parameter) in arguments.iter().zip(parameters) {
            let parameter = self
                .candidate_type_argument(parameter.ty, substitution)?
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
    pub(in crate::check) fn instantiate_call_signature(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        owner: Option<dir::GlobalSymbolId>,
        target: Option<dir::GlobalSymbolId>,
        instance: Option<GenericInstance>,
        function: FunctionTerm,
        generic_arguments: &[GenericArgument],
    ) -> CompilerResult<CallInstantiation> {
        if function.generic_parameters.is_empty() {
            return Ok(CallInstantiation {
                function,
                instance,
                substitution: GenericSubstitution::empty(),
                generic_parameters: Vec::new(),
            });
        }
        let key = CallInstantiationKey {
            source,
            owner,
            target,
        };
        if let Some(instantiation) = self.solutions.call_instantiation.get(&key) {
            return Ok(instantiation.clone());
        }
        let mut substitution = GenericSubstitution::empty();
        let mut arguments = SmallVec::with_capacity(function.generic_parameters.len());
        let generic_parameters = function.generic_parameters.clone();

        // use explicit arguments first, then infer the remaining call generics
        for (index, parameter) in function.generic_parameters.iter().enumerate() {
            let slot = self.generic_parameter_slot(*parameter);
            let argument = if let Some(argument) = generic_arguments.get(index) {
                self.select_argument_for_static_slot(
                    argument,
                    self.generic_parameter_is_static(*parameter),
                )
            } else if let Some(argument) = instance
                .as_ref()
                .filter(|instance| instance.symbol == slot.owner)
                .and_then(|instance| instance.arguments.get(slot.index.0 as usize))
            {
                self.select_argument_for_static_slot(
                    argument,
                    self.generic_parameter_is_static(*parameter),
                )
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
        let instance =
            instance.or_else(|| owner.map(|symbol| GenericInstance { symbol, arguments }));

        let instantiation = CallInstantiation {
            function,
            instance,
            substitution,
            generic_parameters: generic_parameters.to_vec(),
        };

        self.solutions
            .call_instantiation
            .insert(key, instantiation.clone());

        Ok(instantiation)
    }

    /// Return the generic slot for one generic parameter.
    pub(in crate::check) fn generic_parameter_slot(&self, parameter: VariableId) -> GenericSlot {
        let variable = self.variable(parameter);
        let Some(VariableOutput::Generic(generic)) = &variable.output else {
            panic!("generic parameter {parameter:?} is not a generic slot");
        };

        generic.slot().clone()
    }

    /// Return whether one generic parameter expects a static argument.
    fn generic_parameter_is_static(&self, parameter: VariableId) -> bool {
        let variable = self.variable(parameter);
        let Some(VariableOutput::Generic(generic)) = &variable.output else {
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

    /// Return one stable omitted call instantiation argument variable.
    fn instantiation_variable(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        parameter: VariableId,
    ) -> CompilerResult<VariableId> {
        let slot = self.generic_parameter_slot(parameter);
        let key = CallInstantiationArgumentKey {
            source,
            owner: slot.owner,
            index: slot.index,
        };
        let kind = self.variable(parameter).kind;

        if let Some(variable) = self
            .variables
            .call_instantiation_argument
            .get(&key)
            .copied()
        {
            return Ok(variable);
        }
        let origin = Origin::Node(source);
        let variable = self.allocate_inference_variable(module, kind, origin);

        self.variables
            .call_instantiation_argument
            .insert(key, variable);

        Ok(variable)
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
            let Some(VariableOutput::Generic(generic)) = self.variable(*parameter).output.clone()
            else {
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
        generic: &GenericParameter,
    ) -> CompilerResult<Progress> {
        match generic {
            GenericParameter::Type {
                constraint: Some(constraint),
                ..
            }
            | GenericParameter::VariadicType {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_type_operand(substitution, parameter) else {
                    return Ok(Progress::Unchanged);
                };
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.solve_contextual_type_assignability(origin, argument, constraint)
            }
            GenericParameter::Static {
                constraint: Some(constraint),
                ..
            }
            | GenericParameter::VariadicStatic {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_static_operand(substitution, parameter)
                else {
                    return Ok(Progress::Unchanged);
                };
                let source = self.terms.push(TypeTerm::StaticValue { value: argument });
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.solve_contextual_type_assignability(origin, source, constraint)
            }
            GenericParameter::Type {
                constraint: None, ..
            }
            | GenericParameter::VariadicType {
                constraint: None, ..
            }
            | GenericParameter::Static {
                constraint: None, ..
            }
            | GenericParameter::VariadicStatic {
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
            let Some(VariableOutput::Generic(generic)) = self.variable(*parameter).output.clone()
            else {
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
        generic: &GenericParameter,
    ) -> CompilerResult<Decision> {
        match generic {
            GenericParameter::Type {
                constraint: Some(constraint),
                ..
            }
            | GenericParameter::VariadicType {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_type_operand(substitution, parameter) else {
                    return Ok(Decision::Undecidable);
                };
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.decide_type_relation(TypeRelation::Assignable, argument, constraint)
            }
            GenericParameter::Static {
                constraint: Some(constraint),
                ..
            }
            | GenericParameter::VariadicStatic {
                constraint: Some(constraint),
                ..
            } => {
                let Some(argument) = self.substitution_static_operand(substitution, parameter)
                else {
                    return Ok(Decision::Undecidable);
                };
                let source = self.terms.push(TypeTerm::StaticValue { value: argument });
                let constraint = self.substitute_type_operand(module, substitution, *constraint)?;

                self.decide_type_relation(TypeRelation::Assignable, source, constraint)
            }
            GenericParameter::Type {
                constraint: None, ..
            }
            | GenericParameter::VariadicType {
                constraint: None, ..
            }
            | GenericParameter::Static {
                constraint: None, ..
            }
            | GenericParameter::VariadicStatic {
                constraint: None, ..
            } => Ok(Decision::Yes),
        }
    }

    /// Solve omitted generic arguments from their defaults when inference has no input.
    fn solve_defaulted_generic_arguments(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        parameters: &[VariableId],
        explicit_count: usize,
    ) -> CompilerResult<()> {
        for parameter in parameters.iter().skip(explicit_count) {
            self.solve_defaulted_generic_argument(module, substitution, *parameter)?;
        }

        Ok(())
    }

    /// Solve one omitted generic argument from its default.
    fn solve_defaulted_generic_argument(
        &mut self,
        module: ModuleId,
        substitution: &GenericSubstitution,
        parameter: VariableId,
    ) -> CompilerResult<()> {
        let variable = self.variable(parameter);
        let Some(VariableOutput::Generic(generic)) = &variable.output else {
            panic!("generic parameter {parameter:?} is not a generic slot");
        };

        match generic {
            GenericParameter::Type {
                default: Some(default),
                ..
            }
            | GenericParameter::VariadicType {
                default: Some(default),
                ..
            } => {
                let Some(argument) = self.substitution_type_variable(substitution, parameter)
                else {
                    return Ok(());
                };
                if !self.lower_type_bounds(argument).is_empty()
                    || self.variable_solution(argument).is_some()
                {
                    return Ok(());
                }
                let default = self.substitute_type_variable(module, substitution, *default)?;
                let term = TypeTerm::Variable(default);

                self.solve_type_variable(argument, term)?;
            }
            GenericParameter::Static {
                default: Some(default),
                ..
            }
            | GenericParameter::VariadicStatic {
                default: Some(default),
                ..
            } => {
                let Some(argument) = self.substitution_static_variable(substitution, parameter)
                else {
                    return Ok(());
                };
                if !self.lower_type_bounds(argument).is_empty()
                    || self.variable_solution(argument).is_some()
                {
                    return Ok(());
                }
                let default = self.substitute_static_variable(module, substitution, *default)?;
                let term = StaticTerm::Variable(default);

                self.solve_static_variable(argument, term)?;
            }
            GenericParameter::Type { .. }
            | GenericParameter::VariadicType { .. }
            | GenericParameter::Static { .. }
            | GenericParameter::VariadicStatic { .. } => {}
        };

        Ok(())
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
            let Some(expected) = self.solved_type_term(expected)? else {
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
                .merge(self.solve_contextual_type_assignability(origin, *argument, parameter)?);
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
            let void = self.terms.push(void);

            return self.solve_contextual_type_assignability(origin, void, expected);
        };

        self.solve_contextual_type_assignability(origin, return_type, expected)
    }
}
