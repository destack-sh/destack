use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CallableDispatch, CallableSignature, CallableTarget, CheckState, ConstructDecision,
    ConstructFailure, ConstructTargetResolution, ConstructTerm, FunctionTerm, GenericArgument,
    GenericInstance, NominalDefinition, Origin, Progress, TypeOperand, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

use super::CandidateCardinality;

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
        if let Some(selection) = self.selected_construct(construct) {
            return Ok(selection);
        }

        let Some(callee) = self.type_operand_term(construct.callee)? else {
            return Ok(CallableDispatch::pending());
        };
        let candidates = match self.construct_candidates(module, &callee)? {
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
        let candidate_cardinality = CandidateCardinality::from_len(candidates.len());

        // choose the first compatible declaration order candidate
        for candidate in candidates {
            let owner = Some(candidate.target.symbol());
            let instance = candidate.target.instance().cloned();
            let target = CallableTarget::Construct(candidate.target);
            let result = self.select_call_signature(
                origin,
                module,
                source,
                owner,
                instance,
                candidate.function,
                generic_arguments,
                arguments,
                argument_values,
                expected,
                target,
                candidate_cardinality,
            )?;

            match result {
                CallableDispatch::ConstructSelected { .. } => {
                    return Ok(result);
                }
                CallableDispatch::CallSelected { .. } => {
                    unreachable!(
                        "internal invariant: construct candidate selection produced a call selection"
                    );
                }
                CallableDispatch::Pending { .. } if candidate_cardinality.keeps_pending_probe() => {
                    return Ok(result);
                }
                CallableDispatch::Pending { .. } => {
                    saw_pending = true;
                }
                CallableDispatch::Invalid { .. } => {
                    return Ok(result);
                }
                CallableDispatch::CallRejected(_) | CallableDispatch::ConstructRejected(_) => {}
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
        term: &TypeTerm,
    ) -> CompilerResult<ConstructCandidates> {
        let candidates = match term {
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.class_construct_candidates(module, *symbol, arguments)?,
            TypeTerm::Shape(_) => ConstructCandidates::Absent,
            _ => ConstructCandidates::Absent,
        };

        Ok(candidates)
    }

    /// Return construct candidates from one class type.
    fn class_construct_candidates(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<ConstructCandidates> {
        let class = symbol;
        let Some(NominalDefinition::Class(definition)) = self.nominal_definition(module, class)?
        else {
            return Ok(ConstructCandidates::Absent);
        };
        let constructors = definition
            .methods
            .into_iter()
            .filter(|method| {
                matches!(
                    method.slot,
                    dir::MemberSlot::Constructor | dir::MemberSlot::New
                )
            })
            .collect::<Vec<_>>();

        if constructors.is_empty() {
            return Ok(ConstructCandidates::Absent);
        }
        let substitution = self.generic_substitution(class, arguments)?;
        let instance = (!arguments.is_empty()).then(|| GenericInstance {
            owner: class,
            arguments: arguments.to_vec().into(),
        });
        let mut candidates = Vec::with_capacity(constructors.len());

        // lower constructor method types to construct signatures
        for constructor in constructors {
            let Some(symbol) = constructor.symbol else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "class constructor method {:?} has no symbol",
                        constructor.source
                    ),
                });
            };
            let Some(term) = self.type_operand_term(constructor.ty)? else {
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
                    symbol: class,
                    constructor: Some(symbol),
                    instance: instance.clone(),
                },
                function,
            });
        }

        Ok(ConstructCandidates::Present(candidates.into()))
    }

    /// Return the already chosen decision for one construct expression.
    fn selected_construct(&self, construct: &ConstructTerm) -> Option<CallableDispatch> {
        let Some(decision) = self.inference.construct(construct.source) else {
            return None;
        };

        let selection = match decision {
            ConstructDecision::Resolved(resolution) => CallableDispatch::construct_selected(
                resolution.target,
                resolution.function,
                Progress::Unchanged,
            ),
            ConstructDecision::Rejected(failure) => CallableDispatch::construct_rejected(failure),
        };

        Some(selection)
    }
}
