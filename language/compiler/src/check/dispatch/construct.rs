use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallFailure, CallTarget, CallableSelection, CallableSignature, CheckState, ConstructSelection,
    ConstructTerm, FunctionParameter, FunctionTerm, GenericArgument, GenericInstance, Origin,
    Progress, ShapeMember, TypeTerm, VariableId,
};

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
    /// Select one runtime construct target.
    pub(in crate::check) fn select_construct_target(
        &mut self,
        origin: Origin,
        module: ModuleId,
        construct: &ConstructTerm,
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableSelection> {
        if let Some(selection) = self.selected_construct(construct)? {
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
            let target = match candidate.symbol {
                Some(symbol) => CallTarget::Constructor { symbol },
                None => CallTarget::Value,
            };
            let result = self.select_call_signature(
                origin,
                module,
                construct.source,
                candidate.symbol,
                candidate.instance,
                candidate.function,
                &construct.generic_arguments,
                &construct.arguments,
                expected,
                target,
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
                origin: _,
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
        let substitution = self.generic_substitution(module, symbol, arguments)?;
        let instance = (!arguments.is_empty()).then(|| GenericInstance {
            symbol,
            arguments: arguments.to_vec().into(),
        });
        let mut candidates = Vec::with_capacity(constructors.len());

        // lower constructor symbol types to construct signatures
        for constructor in constructors {
            let variable = self.member_type_variable(module, constructor);
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
        if self.construct_symbol_kind(symbol) == dir::SymbolKind::Newtype {
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
            return_type: Some(callee.into()),
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
            return_type: Some(callee.into()),
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
                .map(|element| FunctionParameter {
                    ty: element.ty,
                    is_optional: element.is_optional,
                    is_rest: element.is_rest,
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
        if !self.modules.contains_key(&symbol.module_id) {
            return Ok(None);
        };
        let Some(source) = self.local_symbol_source_node(symbol) else {
            return Ok(None);
        };
        if source.ty != dir::NodeType::Declaration {
            return Ok(None);
        }
        let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(source.id);
        let declaration = self
            .module(symbol.module_id)
            .view()
            .get(declaration_id)
            .clone();
        let backing = match declaration {
            dir::Declaration::Type(declaration) if declaration.is_nominal => {
                Some(self.intern_local_node_type_variable(symbol.module_id, declaration.value))
            }
            _ => None,
        };

        Ok(backing)
    }

    /// Return the declaration kind for one construct symbol.
    fn construct_symbol_kind(&self, symbol: dir::GlobalSymbolId) -> dir::SymbolKind {
        let module = self.module(symbol.module_id);
        let bindings = module.binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);

        symbol.kind
    }

    /// Return construct candidates from one shape term.
    fn shape_construct_candidates(
        &mut self,
        module: ModuleId,
        members: &[ShapeMember],
    ) -> CompilerResult<ConstructCandidates> {
        for member in members {
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
    fn selected_construct(
        &self,
        construct: &ConstructTerm,
    ) -> CompilerResult<Option<CallableSelection>> {
        let Some(decision) = self.solutions.construct.get(&construct.source).cloned() else {
            return Ok(None);
        };

        let selection = match decision {
            ConstructSelection::Resolved(resolution) => {
                let target = match resolution.symbol {
                    Some(symbol) => CallTarget::Constructor { symbol }
                        .into_resolution_target(resolution.instance),
                    None => CallTarget::Value.into_resolution_target(None),
                };

                CallableSelection::resolved(target, resolution.function, Progress::Unchanged)
            }
            ConstructSelection::Rejected(failure) => CallableSelection::rejected(failure.into()),
        };

        Ok(Some(selection))
    }
}
