use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallableDispatch, CallableSignature, CallableTarget, CheckState, ConstructDecision,
    ConstructFailure, ConstructTargetResolution, ConstructTerm, FunctionParameter, FunctionTerm,
    GenericApplication, GenericArgument, Origin, Progress, TypeOperand, TypeTerm, VariableId,
};

/// Construct signature candidate selected from a callee type.
pub(in crate::check) struct ConstructCandidate {
    /// The selected construct target.
    pub(in crate::check) target: ConstructTargetResolution,
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
    ) -> CompilerResult<CallableDispatch> {
        if let Some(selection) = self.selected_construct(construct)? {
            return Ok(selection);
        }

        let Some(callee) = self.type_operand_term(construct.callee)? else {
            return Ok(CallableDispatch::pending());
        };
        let candidates = match self.construct_candidates(module, construct.callee, &callee)? {
            ConstructCandidates::Pending => return Ok(CallableDispatch::pending()),
            ConstructCandidates::Absent => {
                return Ok(CallableDispatch::construct_rejected(
                    ConstructFailure::NotConstructible,
                ));
            }
            ConstructCandidates::Present(candidates) => candidates,
        };
        let mut saw_pending = false;

        // choose the first compatible declaration-order candidate
        for candidate in candidates {
            let probe = self.begin_inference_probe();
            let owner = Some(candidate.target.symbol());
            let application = candidate.target.application().cloned();
            let target = CallableTarget::Construct(candidate.target);
            let result = self.select_call_signature(
                origin,
                module,
                construct.source,
                owner,
                application,
                candidate.function,
                &construct.generic_arguments,
                &construct.arguments,
                &[],
                expected,
                target,
            )?;
            match result {
                CallableDispatch::ConstructSelected { .. } => {
                    self.commit_inference_probe(probe);

                    return Ok(result);
                }
                CallableDispatch::CallSelected { .. } => {
                    panic!("construct dispatch produced a call selection");
                }
                CallableDispatch::Pending { .. } => {
                    self.drop_inference_probe(probe);
                    saw_pending = true;
                }
                CallableDispatch::CallRejected(_) | CallableDispatch::ConstructRejected(_) => {
                    self.drop_inference_probe(probe);
                }
            }
        }

        if saw_pending {
            Ok(CallableDispatch::pending())
        } else {
            Ok(CallableDispatch::construct_rejected(
                ConstructFailure::NoMatch,
            ))
        }
    }

    /// Return construct candidates from one callee type.
    pub(in crate::check) fn construct_candidates(
        &mut self,
        module: ModuleId,
        callee: TypeOperand,
        term: &TypeTerm,
    ) -> CompilerResult<ConstructCandidates> {
        let candidates = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.type_solution(*variable)? else {
                    return Ok(ConstructCandidates::Pending);
                };

                return self.construct_candidates(module, (*variable).into(), &term);
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.nominal_construct_candidates(module, callee, *symbol, arguments)?,
            TypeTerm::Shape { .. } => ConstructCandidates::Absent,
            _ => ConstructCandidates::Absent,
        };

        Ok(candidates)
    }

    /// Return construct candidates from one nominal type.
    fn nominal_construct_candidates(
        &mut self,
        module: ModuleId,
        callee: TypeOperand,
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
        let instance = (!arguments.is_empty()).then(|| GenericApplication {
            owner: symbol,
            arguments: arguments.to_vec().into(),
        });
        let mut candidates = Vec::with_capacity(constructors.len());

        // lower constructor symbol types to construct signatures
        for constructor in constructors {
            let operand = self.member_type_operand(constructor);
            let Some(term) = self.type_operand_term(operand)? else {
                return Ok(ConstructCandidates::Pending);
            };
            let CallableSignature::Present(function) = self.call_signature(module, &term)? else {
                continue;
            };
            let function = if substitution.is_empty() {
                function
            } else {
                function.substitute(module, &substitution, self)?
            };

            candidates.push(ConstructCandidate {
                target: ConstructTargetResolution::Class {
                    symbol,
                    constructor: Some(constructor),
                    application: instance.clone(),
                },
                function,
            });
        }

        Ok(ConstructCandidates::Present(candidates.into()))
    }

    /// Return implicit constructor candidates for a nominal type.
    fn implicit_construct_candidates(
        &mut self,
        callee: TypeOperand,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<ConstructCandidates> {
        let kind = self.construct_symbol_kind(symbol);
        if kind == dir::SymbolKind::Newtype {
            return self.newtype_constructor_candidate(callee, symbol, arguments);
        }
        if kind != dir::SymbolKind::Class {
            return Ok(ConstructCandidates::Absent);
        }

        let instance = (!arguments.is_empty()).then(|| GenericApplication {
            owner: symbol,
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
            target: ConstructTargetResolution::Class {
                symbol,
                constructor: None,
                application: instance,
            },
            function,
        };

        Ok(ConstructCandidates::Present(vec![candidate].into()))
    }

    /// Return an implicit constructor candidate for a newtype backing type.
    fn newtype_constructor_candidate(
        &mut self,
        callee: TypeOperand,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<ConstructCandidates> {
        let Some(backing) = self.newtype_backing_type(symbol)? else {
            return Ok(ConstructCandidates::Pending);
        };
        let parameters = self.newtype_constructor_parameters(backing)?;
        let instance = (!arguments.is_empty()).then(|| GenericApplication {
            owner: symbol,
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
            target: ConstructTargetResolution::Newtype {
                symbol,
                application: instance,
            },
            function,
        };

        Ok(ConstructCandidates::Present(vec![candidate].into()))
    }

    /// Return the constructor parameters implied by a newtype backing type.
    fn newtype_constructor_parameters(
        &mut self,
        backing: TypeOperand,
    ) -> CompilerResult<SmallVec<[FunctionParameter; 4]>> {
        let Some(term) = self.type_operand_term(backing)? else {
            let parameter = FunctionParameter::required(backing);

            return Ok(vec![parameter].into());
        };
        let parameters = match term {
            TypeTerm::Tuple { elements, .. } => elements
                .iter()
                .map(|element| FunctionParameter {
                    ty: element.ty,
                    static_slot: None,
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
    ) -> CompilerResult<Option<TypeOperand>> {
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
                Some(self.require_local_node_type(symbol.module_id, declaration.value))
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

    /// Return the already chosen decision for one construct expression.
    fn selected_construct(
        &self,
        construct: &ConstructTerm,
    ) -> CompilerResult<Option<CallableDispatch>> {
        let Some(decision) = self.inference.construct(construct.source) else {
            return Ok(None);
        };

        let selection = match decision {
            ConstructDecision::Resolved(resolution) => CallableDispatch::construct_selected(
                resolution.target,
                resolution.function,
                Progress::Unchanged,
            ),
            ConstructDecision::Rejected(failure) => CallableDispatch::construct_rejected(failure),
        };

        Ok(Some(selection))
    }
}
