use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CallableDispatch, CallableSignature, CallableTarget, CheckState, ConstructDecision,
    ConstructFailure, ConstructTargetResolution, ConstructTerm, FunctionParameter, FunctionTerm,
    GenericApplication, GenericArgument, Origin, Progress, TypeOperand, TypeTerm, VariableId,
};

use super::CandidateSet;

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

        self.select_construct_candidate(
            origin,
            module,
            construct.source,
            candidates,
            &construct.generic_arguments,
            &construct.arguments,
            &[],
            expected,
        )
    }

    /// Select the first applicable construct candidate.
    pub(in crate::check) fn select_construct_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        candidates: SmallVec<[ConstructCandidate; 4]>,
        generic_arguments: &[GenericArgument],
        arguments: &[TypeOperand],
        argument_values: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let mut saw_pending = false;
        let candidate_set = CandidateSet::from_len(candidates.len());

        // choose the first compatible declaration order candidate
        for candidate in candidates {
            let probe = self.begin_inference_probe();
            let owner = Some(candidate.target.symbol());
            let application = candidate.target.application().cloned();
            let target = CallableTarget::Construct(candidate.target);
            let result = self.select_call_signature(
                origin,
                module,
                source,
                owner,
                application,
                candidate.function,
                generic_arguments,
                arguments,
                argument_values,
                expected,
                target,
                candidate_set,
            )?;

            // keep writes only for selected or uniquely pending candidates
            match result {
                CallableDispatch::ConstructSelected { .. } => {
                    self.commit_inference_probe(probe)?;

                    return Ok(result);
                }
                CallableDispatch::CallSelected { .. } => {
                    panic!("construct candidate selection produced a call selection");
                }
                CallableDispatch::Pending { .. } if candidate_set.keeps_pending_probe() => {
                    self.commit_inference_probe(probe)?;

                    return Ok(result);
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

        // preserve unresolved overload input
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
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.nominal_construct_candidates(module, callee, *symbol, arguments)?,
            TypeTerm::Shape(_) => ConstructCandidates::Absent,
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
        let candidates = match self.symbol_kind(module, symbol) {
            Some(dir::SymbolKind::Class) => {
                self.class_construct_candidates(module, callee, symbol, arguments)?
            }
            Some(dir::SymbolKind::Newtype) => {
                self.newtype_construct_candidate(module, callee, symbol, arguments)?
            }
            _ => ConstructCandidates::Absent,
        };

        Ok(candidates)
    }

    /// Return construct candidates from one class type.
    fn class_construct_candidates(
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
            return Ok(self.default_class_construct_candidate(callee, symbol, arguments));
        }
        let substitution = self.generic_substitution(module, symbol, arguments)?;
        let instance = (!arguments.is_empty()).then(|| GenericApplication {
            owner: symbol,
            arguments: arguments.to_vec().into(),
        });
        let mut candidates = Vec::with_capacity(constructors.len());

        // lower constructor symbol types to construct signatures
        for constructor in constructors {
            let operand = self.member_type_operand(module, constructor);
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

    /// Return the default constructor candidate for one class.
    fn default_class_construct_candidate(
        &self,
        callee: TypeOperand,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> ConstructCandidates {
        let instance = (!arguments.is_empty()).then(|| GenericApplication {
            owner: symbol,
            arguments: arguments.to_vec().into(),
        });
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Vec::new(),
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

        ConstructCandidates::Present(vec![candidate].into())
    }

    /// Return the construct candidate for one newtype.
    fn newtype_construct_candidate(
        &mut self,
        module: ModuleId,
        callee: TypeOperand,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<ConstructCandidates> {
        let Some(representation) = self.newtype_representation(module, symbol)? else {
            return Ok(ConstructCandidates::Absent);
        };
        let generic_parameters = self
            .generic_slots_for_owner(module, symbol)
            .map(|(variable, _)| variable)
            .collect::<Vec<_>>();
        let substitution = self.generic_substitution(module, symbol, arguments)?;
        let backing = if substitution.is_empty() {
            representation.backing
        } else {
            self.substitute_type_operand(module, &substitution, representation.backing)?
        };
        let instance = (!arguments.is_empty()).then(|| GenericApplication {
            owner: symbol,
            arguments: arguments.to_vec().into(),
        });
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters,
            this_parameter: None,
            parameters: vec![FunctionParameter::required(backing)].into(),
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
