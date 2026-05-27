use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CallFailure, CallTarget, CallableSelection, CallableSignature, CheckState, ConstructDecision,
    ConstructFailure, ConstructResolution, FunctionParameter, FunctionTerm, GenericArgument,
    GenericInstance, Progress, Reduction, ShapeMember, TypeLiteralTerm, TypeOperand, TypeTerm,
    VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Runtime construct expression term.
///
/// ```ts
/// new User(name)
/// new Ctor()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ConstructTerm {
    /// The source construct expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The construct callee type.
    pub(in crate::check) callee: VariableId,
    /// The explicit construct generic arguments.
    pub(in crate::check) generic_arguments: SmallVec<[GenericArgument; 4]>,
    /// The argument expression types.
    pub(in crate::check) arguments: SmallVec<[TypeOperand; 4]>,
}

impl ConstructTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.push(self.callee);
        variables.extend(
            self.generic_arguments
                .iter()
                .flat_map(|argument| state.argument_variables(argument)),
        );
        variables.extend(
            self.arguments
                .iter()
                .flat_map(|argument| argument.referenced_variables(state)),
        );

        variables
    }
}

/// Construct signature candidate selected from a callee type.
pub(in crate::check) struct ConstructCandidate {
    /// The constructor symbol when the candidate is symbol backed.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The owner generic instance when the candidate comes from an applied nominal type.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The constructor function signature.
    pub(in crate::check) function: FunctionTerm,
}

/// Construct signatures extracted from a callee type.
pub(in crate::check) enum ConstructCandidates {
    /// Candidate extraction is waiting for solver input.
    Pending,
    /// The callee has no construct signatures.
    Absent,
    /// The callee has one or more construct signatures.
    Present(SmallVec<[ConstructCandidate; 4]>),
}

impl CheckState<'_> {
    /// Reduce one runtime construct expression to its return type.
    pub(in crate::check) fn reduce_construct_term(
        &mut self,
        module: ModuleId,
        construct: &ConstructTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let result = self.resolve_construct(module, construct, None)?;
        let progress = result.progress();
        let function = match &result {
            CallableSelection::Resolved {
                target, function, ..
            } => {
                let (symbol, instance) = target.as_constructor_target();

                self.record_construct_resolution(construct, symbol, instance, function)?;

                function
            }
            CallableSelection::Rejected(failure) => {
                let failure = construct_failure_from_call_failure(failure);

                self.record_construct_rejection(construct, failure)?;

                return Ok(Reduction::progress(progress));
            }
            CallableSelection::Pending { .. } => return Ok(Reduction::progress(progress)),
        };

        let term = match function.return_type {
            Some(return_type) => TypeTerm::Variable(return_type),
            None => TypeTerm::Literal(TypeLiteralTerm::Void),
        };
        Ok(Reduction {
            value: Some(term),
            progress,
        })
    }

    /// Expect resolved construct candidates to produce the expected result.
    pub(in crate::check) fn expect_construct_term(
        &mut self,
        module: ModuleId,
        construct: &ConstructTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.resolve_construct(module, construct, Some(result))?;
        let progress = resolved.progress();
        let progress = match &resolved {
            CallableSelection::Resolved {
                target, function, ..
            } => {
                let (symbol, instance) = target.as_constructor_target();

                self.record_construct_resolution(construct, symbol, instance, function)?;

                match function.return_type {
                    Some(return_type) => {
                        progress.merge(self.solve_type_assignability(return_type, result)?)
                    }
                    None => progress,
                }
            }
            CallableSelection::Rejected(failure) => {
                let failure = construct_failure_from_call_failure(failure);

                self.record_construct_rejection(construct, failure)?;

                progress
            }
            CallableSelection::Pending { .. } => progress,
        };

        Ok(progress)
    }

    /// Resolve one runtime construct target.
    fn resolve_construct(
        &mut self,
        module: ModuleId,
        construct: &ConstructTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableSelection> {
        if let Some(selection) = self.recorded_construct_selection(construct)? {
            return Ok(selection);
        }

        let Some(callee) = self.solved_type_term(construct.callee)? else {
            return Ok(CallableSelection::pending());
        };
        let candidates = match self.construct_candidates(module, construct.callee, &callee)? {
            ConstructCandidates::Pending => return Ok(CallableSelection::pending()),
            ConstructCandidates::Absent => {
                return Ok(CallableSelection::rejected(CallFailure::NotCallable));
            }
            ConstructCandidates::Present(candidates) => candidates,
        };
        let mut saw_pending = false;

        // choose the first compatible declaration-order candidate
        for candidate in candidates {
            let result = self.resolve_call_signature(
                module,
                construct.source,
                candidate.symbol,
                candidate.instance,
                candidate.function,
                &construct.generic_arguments,
                &construct.arguments,
                expected,
                match candidate.symbol {
                    Some(symbol) => CallTarget::Constructor { symbol },
                    None => CallTarget::Value,
                },
            )?;
            match result {
                CallableSelection::Resolved { .. } => return Ok(result),
                CallableSelection::Pending { .. } => saw_pending = true,
                CallableSelection::Rejected(_) => {}
            }
        }

        if saw_pending {
            Ok(CallableSelection::pending())
        } else {
            Ok(CallableSelection::rejected(CallFailure::NoMatch))
        }
    }

    /// Return construct candidates from one callee type.
    pub(in crate::check) fn construct_candidates(
        &mut self,
        module: ModuleId,
        callee: VariableId,
        term: &TypeTerm,
    ) -> CompilerResult<ConstructCandidates> {
        let candidates = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(ConstructCandidates::Pending);
                };

                return self.construct_candidates(module, *variable, &term);
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.nominal_construct_candidates(module, callee, *symbol, arguments)?,
            TypeTerm::Shape { members } => self.shape_construct_candidates(module, members)?,
            _ => ConstructCandidates::Absent,
        };

        Ok(candidates)
    }

    /// Return construct candidates from one nominal type.
    fn nominal_construct_candidates(
        &mut self,
        module: ModuleId,
        callee: VariableId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<ConstructCandidates> {
        let constructors = self.visible_role_member_symbols(
            symbol,
            &[dir::MemberSlot::Constructor, dir::MemberSlot::New],
        )?;
        if constructors.is_empty() {
            return self.implicit_construct_candidates(callee, symbol, arguments);
        }
        let substitution = self.generic_substitution(symbol, arguments)?;
        let instance = (!arguments.is_empty()).then(|| GenericInstance {
            symbol,
            arguments: arguments.to_vec().into(),
        });
        let mut candidates = Vec::with_capacity(constructors.len());

        // lower constructor symbol types to construct signatures
        for constructor in constructors {
            let variable = self.member_type_variable(module, constructor)?;
            let Some(term) = self.solved_type_term(variable)? else {
                return Ok(ConstructCandidates::Pending);
            };
            let CallableSignature::Present(function) =
                self.call_signature(variable.module, &term)?
            else {
                continue;
            };
            let function = if substitution.is_empty() {
                function
            } else {
                function.substitute(module, &substitution, self)?
            };

            candidates.push(ConstructCandidate {
                symbol: Some(constructor),
                instance: instance.clone(),
                function,
            });
        }

        Ok(ConstructCandidates::Present(candidates.into()))
    }

    /// Return implicit constructor candidates for a nominal type.
    fn implicit_construct_candidates(
        &mut self,
        callee: VariableId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<ConstructCandidates> {
        if self.construct_symbol_kind(symbol)? == dir::SymbolKind::Newtype {
            return self.newtype_constructor_candidate(callee, symbol, arguments);
        }

        let instance = (!arguments.is_empty()).then(|| GenericInstance {
            symbol,
            arguments: arguments.to_vec().into(),
        });
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: SmallVec::new(),
            this_parameter: None,
            parameters: Vec::new().into(),
            return_type: Some(callee),
            is_generator: false,
        };
        let candidate = ConstructCandidate {
            symbol: None,
            instance,
            function,
        };

        Ok(ConstructCandidates::Present(vec![candidate].into()))
    }

    /// Return an implicit constructor candidate for a newtype backing type.
    fn newtype_constructor_candidate(
        &mut self,
        callee: VariableId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<ConstructCandidates> {
        let Some(backing) = self.newtype_backing_type(symbol)? else {
            return Ok(ConstructCandidates::Pending);
        };
        let parameters = self.newtype_constructor_parameters(backing)?;
        let instance = (!arguments.is_empty()).then(|| GenericInstance {
            symbol,
            arguments: arguments.to_vec().into(),
        });
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: SmallVec::new(),
            this_parameter: None,
            parameters,
            return_type: Some(callee),
            is_generator: false,
        };
        let candidate = ConstructCandidate {
            symbol: None,
            instance,
            function,
        };

        Ok(ConstructCandidates::Present(vec![candidate].into()))
    }

    /// Return the constructor parameters implied by a newtype backing type.
    fn newtype_constructor_parameters(
        &mut self,
        backing: VariableId,
    ) -> CompilerResult<SmallVec<[FunctionParameter; 4]>> {
        let Some(term) = self.solved_type_term(backing)? else {
            let parameter = FunctionParameter::required(backing);

            return Ok(vec![parameter].into());
        };
        let parameters = match term {
            TypeTerm::Tuple { elements, .. } => elements
                .iter()
                .map(|element| {
                    let element = element;
                    let parameter = FunctionParameter {
                        ty: element.ty,
                        is_optional: element.is_optional,
                        is_rest: element.is_rest,
                    };

                    parameter
                })
                .collect(),
            _ => vec![FunctionParameter::required(backing)],
        };

        Ok(parameters.into())
    }

    /// Return the backing type variable for a newtype symbol.
    fn newtype_backing_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<VariableId>> {
        if !self.inputs.contains_key(&symbol.module_id) {
            return Ok(None);
        };
        let Some(source) = self.symbol_source_node(symbol.module_id, symbol) else {
            return Ok(None);
        };
        if source.ty != dir::NodeType::Declaration {
            return Ok(None);
        }
        let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(source.id);
        let declaration = self
            .input(symbol.module_id)
            .view()
            .get(declaration_id)
            .clone();
        let backing = match declaration {
            dir::Declaration::Type(declaration) if declaration.is_nominal => {
                Some(self.intern_local_type_variable(symbol.module_id, declaration.value))
            }
            _ => None,
        };

        Ok(backing)
    }

    /// Return the declaration kind for one construct symbol.
    fn construct_symbol_kind(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::SymbolKind> {
        let Some(module) = self.inputs.get(&symbol.module_id) else {
            return Err(CompilerError::Internal {
                message: format!("construct symbol {symbol:?} is outside the check component"),
            });
        };
        let bindings = module.binding_table();
        let Some(symbol) = bindings.get_symbol_maybe(symbol.local_id) else {
            return Err(CompilerError::Internal {
                message: "construct symbol is not visible in its module".to_string(),
            });
        };

        Ok(symbol.kind)
    }

    /// Return construct candidates from one shape term.
    fn shape_construct_candidates(
        &mut self,
        module: ModuleId,
        members: &[ShapeMember],
    ) -> CompilerResult<ConstructCandidates> {
        for member in members {
            let member = member;
            let ShapeMember::ConstructSignature { ty } = member else {
                continue;
            };
            let Some(term) = self.type_operand_term(*ty)? else {
                return Ok(ConstructCandidates::Pending);
            };
            let CallableSignature::Present(function) = self.call_signature(module, &term)? else {
                return Ok(ConstructCandidates::Absent);
            };
            let candidate = ConstructCandidate {
                symbol: None,
                instance: None,
                function,
            };

            return Ok(ConstructCandidates::Present(vec![candidate].into()));
        }

        Ok(ConstructCandidates::Absent)
    }

    /// Return the already chosen decision for one construct expression.
    fn recorded_construct_selection(
        &self,
        construct: &ConstructTerm,
    ) -> CompilerResult<Option<CallableSelection>> {
        let Some(decision) = self.solutions.construct.get(&construct.source).cloned() else {
            return Ok(None);
        };

        let selection = match decision {
            ConstructDecision::Resolved(resolution) => {
                let target = match resolution.symbol {
                    Some(symbol) => CallTarget::Constructor { symbol }
                        .into_resolution_target(resolution.instance),
                    None => CallTarget::Value.into_resolution_target(None),
                };

                CallableSelection::resolved(target, resolution.function, Progress::Unchanged)
            }
            ConstructDecision::Rejected(failure) => {
                CallableSelection::rejected(call_failure_from_construct_failure(failure))
            }
        };

        Ok(Some(selection))
    }

    /// Record one rejected construct expression for diagnostics.
    fn record_construct_rejection(
        &mut self,
        construct: &ConstructTerm,
        failure: ConstructFailure,
    ) -> CompilerResult<()> {
        let decision = ConstructDecision::Rejected(failure);

        self.record_construct_decision(construct.source, decision);

        Ok(())
    }

    /// Record one resolved construct expression for commit.
    fn record_construct_resolution(
        &mut self,
        construct: &ConstructTerm,
        symbol: Option<dir::GlobalSymbolId>,
        instance: Option<&GenericInstance>,
        function: &FunctionTerm,
    ) -> CompilerResult<()> {
        let resolution = ConstructResolution {
            source: construct.source,
            symbol,
            instance: instance.cloned(),
            function: function.clone(),
        };
        let decision = ConstructDecision::Resolved(resolution);

        self.record_construct_decision(construct.source, decision);

        Ok(())
    }
}

/// Convert call failure detail to construct failure detail.
fn construct_failure_from_call_failure(failure: &CallFailure) -> ConstructFailure {
    match failure {
        CallFailure::NotCallable => ConstructFailure::NotConstructible,
        CallFailure::NoMatch | CallFailure::ArgumentType { .. } => ConstructFailure::NoMatch,
    }
}

/// Convert construct failure detail to call failure detail.
fn call_failure_from_construct_failure(failure: ConstructFailure) -> CallFailure {
    match failure {
        ConstructFailure::NotConstructible => CallFailure::NotCallable,
        ConstructFailure::NoMatch => CallFailure::NoMatch,
    }
}
