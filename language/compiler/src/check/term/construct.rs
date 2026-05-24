use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    ArgumentTerm, CallTarget, CallableSelection, CallableSignature, CheckComponentState,
    ConstructFailure, ConstructOutcome, ConstructResolution, FunctionTerm, GenericInstance,
    Progress, ShapeMemberTerm, TypeLiteralTerm, TypeTerm, VariableId,
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
    /// The constructed expression type.
    pub(in crate::check) callee: VariableId,
    /// The explicit construct generic arguments.
    pub(in crate::check) generic_arguments: Vec<ArgumentTerm>,
    /// The argument expression types.
    pub(in crate::check) arguments: Vec<VariableId>,
}

impl ConstructTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> smallvec::SmallVec<[VariableId; 4]> {
        let mut variables = smallvec::SmallVec::new();

        variables.push(self.callee);
        variables.extend(self.generic_arguments.iter().map(ArgumentTerm::variable));
        variables.extend(self.arguments.iter().copied());

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
    Present(Vec<ConstructCandidate>),
}

impl CheckComponentState<'_> {
    /// Reduce one runtime construct to its return type.
    pub(in crate::check) fn reduce_construct_type(
        &mut self,
        module: ModuleId,
        construct: &ConstructTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let result = self.resolve_construct(module, construct, None)?;
        let function = match &result {
            CallableSelection::Resolved { target, function } => {
                let (symbol, instance) = target.as_construct_target();

                self.record_construct_resolution(construct, symbol, instance, function)?;

                function
            }
            CallableSelection::NotCallable => {
                self.record_construct_rejection(construct, ConstructFailure::NotConstructible)?;

                return Ok(None);
            }
            CallableSelection::NoMatch => {
                self.record_construct_rejection(construct, ConstructFailure::NoMatch)?;

                return Ok(None);
            }
            CallableSelection::Pending => return Ok(None),
        };

        let term = match function.return_type {
            Some(return_type) => TypeTerm::Variable(return_type),
            None => TypeTerm::Literal(TypeLiteralTerm::Void),
        };
        let term = self.push_solved_type_variable(module, term)?;

        Ok(Some(TypeTerm::Variable(term)))
    }

    /// Apply an expected construct result to resolved construct candidates.
    pub(in crate::check) fn expect_construct_result(
        &mut self,
        module: ModuleId,
        construct: &ConstructTerm,
        result: VariableId,
    ) -> CompilerResult<Progress> {
        let resolved = self.resolve_construct(module, construct, Some(result))?;
        let progress = match &resolved {
            CallableSelection::Resolved { target, function } => {
                let (symbol, instance) = target.as_construct_target();

                self.record_construct_resolution(construct, symbol, instance, function)?;

                match function.return_type {
                    Some(return_type) => self.relate_type_assignable(return_type, result)?,
                    None => Progress::Unchanged,
                }
            }
            CallableSelection::NotCallable => {
                self.record_construct_rejection(construct, ConstructFailure::NotConstructible)?;

                Progress::Unchanged
            }
            CallableSelection::NoMatch => {
                self.record_construct_rejection(construct, ConstructFailure::NoMatch)?;

                Progress::Unchanged
            }
            CallableSelection::Pending => Progress::Unchanged,
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
        let Some(callee) = self.solved_type_term(construct.callee)? else {
            return Ok(CallableSelection::Pending);
        };
        let candidates = match self.construct_candidates(module, construct.callee, &callee)? {
            ConstructCandidates::Pending => return Ok(CallableSelection::Pending),
            ConstructCandidates::Absent => return Ok(CallableSelection::NotCallable),
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
                    Some(symbol) => CallTarget::Construct { symbol },
                    None => CallTarget::Value,
                },
            )?;
            match result {
                CallableSelection::Resolved { .. } => return Ok(result),
                CallableSelection::Pending => saw_pending = true,
                CallableSelection::NoMatch | CallableSelection::NotCallable => {}
            }
        }

        if saw_pending {
            Ok(CallableSelection::Pending)
        } else {
            Ok(CallableSelection::NoMatch)
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
        arguments: &[crate::check::ArgumentTerm],
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
            arguments: arguments.to_vec(),
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

        Ok(ConstructCandidates::Present(candidates))
    }

    /// Return implicit constructor candidates for a nominal type.
    fn implicit_construct_candidates(
        &mut self,
        callee: VariableId,
        symbol: dir::GlobalSymbolId,
        arguments: &[crate::check::ArgumentTerm],
    ) -> CompilerResult<ConstructCandidates> {
        if self.symbol_form(symbol)? == dir::SymbolForm::Newtype {
            return self.newtype_construct_candidate(callee, symbol, arguments);
        }

        let instance = (!arguments.is_empty()).then(|| GenericInstance {
            symbol,
            arguments: arguments.to_vec(),
        });
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters: Vec::new(),
            return_type: Some(callee),
            is_generator: false,
        };
        let candidate = ConstructCandidate {
            symbol: None,
            instance,
            function,
        };
        Ok(ConstructCandidates::Present(vec![candidate]))
    }

    /// Return an implicit constructor candidate for a newtype backing type.
    fn newtype_construct_candidate(
        &mut self,
        callee: VariableId,
        symbol: dir::GlobalSymbolId,
        arguments: &[crate::check::ArgumentTerm],
    ) -> CompilerResult<ConstructCandidates> {
        let Some(backing) = self.newtype_backing_type(symbol)? else {
            return Ok(ConstructCandidates::Pending);
        };
        let parameters = self.newtype_construct_parameters(backing)?;
        let instance = (!arguments.is_empty()).then(|| GenericInstance {
            symbol,
            arguments: arguments.to_vec(),
        });
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Vec::new(),
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

        Ok(ConstructCandidates::Present(vec![candidate]))
    }

    /// Return the constructor parameters implied by a newtype backing type.
    fn newtype_construct_parameters(
        &mut self,
        backing: VariableId,
    ) -> CompilerResult<Vec<VariableId>> {
        let Some(term) = self.solved_type_term(backing)? else {
            return Ok(vec![backing]);
        };
        let parameters = match term {
            TypeTerm::Tuple { elements, .. } => elements.iter().map(|element| element.ty).collect(),
            _ => vec![backing],
        };

        Ok(parameters)
    }

    /// Return the backing type variable for a newtype symbol.
    fn newtype_backing_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<VariableId>> {
        let Some(module) = self.modules.get_mut(&symbol.module_id) else {
            return Ok(None);
        };
        let Some(source) = module.symbol_source_node(symbol) else {
            return Ok(None);
        };
        if source.ty != dir::NodeType::Declaration {
            return Ok(None);
        }
        let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(source.id);
        let declaration = module.input.view().get(declaration_id).clone();
        let backing = match declaration {
            dir::Declaration::Type(declaration) if declaration.is_nominal => {
                Some(module.type_expression_variable(declaration.value))
            }
            _ => None,
        };

        Ok(backing)
    }

    /// Return the declaration form for one symbol.
    fn symbol_form(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<dir::SymbolForm> {
        let Some(module) = self.modules.get(&symbol.module_id) else {
            return Err(CompilerError::Internal {
                message: format!("construct symbol {symbol:?} is outside the check component"),
            });
        };
        let bindings = module.binding_table();
        let form = bindings.get_symbol(symbol.local_id).form;

        Ok(form)
    }

    /// Return construct candidates from one shape term.
    fn shape_construct_candidates(
        &mut self,
        module: ModuleId,
        members: &[ShapeMemberTerm],
    ) -> CompilerResult<ConstructCandidates> {
        for member in members {
            let ShapeMemberTerm::ConstructSignature { ty } = member else {
                continue;
            };
            let Some(term) = self.solved_type_term(*ty)? else {
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

            return Ok(ConstructCandidates::Present(vec![candidate]));
        }

        Ok(ConstructCandidates::Absent)
    }

    /// Record one rejected construct for diagnostics.
    fn record_construct_rejection(
        &mut self,
        construct: &ConstructTerm,
        failure: ConstructFailure,
    ) -> CompilerResult<()> {
        let outcome = ConstructOutcome::Rejected(failure);

        self.module_mut(construct.source.module_id)?
            .record_construct_outcome(construct.source, outcome);

        Ok(())
    }

    /// Record one resolved construct for commit.
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
        let outcome = ConstructOutcome::Resolved(resolution);

        self.module_mut(construct.source.module_id)?
            .record_construct_outcome(construct.source, outcome);

        Ok(())
    }
}
