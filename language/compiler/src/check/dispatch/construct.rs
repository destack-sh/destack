use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    CallableDispatch, CallableSignature, CallableTarget, CheckState, ConstructDecision,
    ConstructFailure, ConstructTargetResolution, ConstructTerm, Definition, FunctionParameter,
    FunctionTerm, GenericArgument, GenericInstance, Origin, TypeOperand, TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

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

/// Construct source read from a definition before inference mutation.
enum DefinitionConstructSource {
    /// Class constructor methods.
    Class(Vec<crate::check::MethodDefinition>),
    /// Newtype backing operand.
    Newtype(TypeOperand),
    /// Definition has no construct form.
    Absent,
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

        let Some(callee) = self.reduce_type_operand(origin, construct.callee)? else {
            return Ok(CallableDispatch::pending());
        };
        let candidates = match self.construct_candidates(module, callee)? {
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
            &construct.argument_types,
            &construct.arguments,
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
        argument_types: &[TypeOperand],
        arguments: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let mut pending_candidate = None;
        let mut pending_count = 0;
        // choose the first compatible declaration order candidate
        for (index, candidate) in candidates.iter().enumerate() {
            let probe = self.inference.begin_probe();
            let result = self.select_one_construct_candidate(
                origin,
                module,
                source,
                candidate,
                generic_arguments,
                argument_types,
                arguments,
                expected,
            )?;

            match result {
                CallableDispatch::ConstructSelected { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                CallableDispatch::CallSelected { .. } => {
                    unreachable!("construct candidate selection produced a call selection");
                }
                CallableDispatch::Pending => {
                    self.inference.drop_probe(probe);
                    pending_count += 1;
                    pending_candidate = Some(index);
                }
                CallableDispatch::Invalid => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                CallableDispatch::CallRejected(_) | CallableDispatch::ConstructRejected(_) => {
                    self.inference.drop_probe(probe);
                }
            }
        }

        // commit the unique unresolved candidate
        if let (1, Some(index)) = (pending_count, pending_candidate) {
            let probe = self.inference.begin_probe();
            let result = self.select_one_construct_candidate(
                origin,
                module,
                source,
                &candidates[index],
                generic_arguments,
                argument_types,
                arguments,
                expected,
            )?;

            self.inference.commit_probe(probe)?;

            return Ok(result);
        }

        // preserve unresolved ambiguous overload input
        if pending_count > 1 {
            Ok(CallableDispatch::pending())
        } else {
            Ok(CallableDispatch::construct_rejected(
                ConstructFailure::NoMatch,
            ))
        }
    }

    /// Select one construct candidate inside the active inference probe.
    fn select_one_construct_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        candidate: &ConstructCandidate,
        generic_arguments: &[GenericArgument],
        argument_types: &[TypeOperand],
        arguments: &[dir::GlobalNodeId<dir::Expression>],
        expected: Option<VariableId>,
    ) -> CompilerResult<CallableDispatch> {
        let owner = Some(candidate.target.symbol());
        let instance = candidate.target.instance().cloned();
        let target = CallableTarget::Construct(candidate.target.clone());

        self.select_callable_signature(
            origin,
            module,
            source,
            owner,
            instance,
            candidate.function.clone(),
            generic_arguments,
            argument_types,
            arguments,
            expected,
            target,
        )
    }

    /// Return construct candidates from one callee type.
    pub(in crate::check) fn construct_candidates(
        &mut self,
        module: ModuleId,
        callee: TypeOperand,
    ) -> CompilerResult<ConstructCandidates> {
        let Some(term) = self.type_operand_term(callee)? else {
            return Ok(ConstructCandidates::Pending);
        };
        let candidates = match term {
            TypeTerm::Reference {
                origin,
                symbol,
                arguments,
            } => self.reference_construct_candidates(module, origin, symbol, &arguments)?,
            TypeTerm::Shape(_) => ConstructCandidates::Absent,
            _ => ConstructCandidates::Absent,
        };

        Ok(candidates)
    }

    /// Return construct candidates from one nominal reference.
    fn reference_construct_candidates(
        &mut self,
        module: ModuleId,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<ConstructCandidates> {
        let Some(definition) = self.definition(module, symbol)? else {
            return Ok(ConstructCandidates::Absent);
        };
        let source = match definition {
            // collect class constructor methods
            Definition::Class(definition) => {
                let constructors = definition
                    .methods
                    .iter()
                    .filter(|method| {
                        matches!(
                            method.slot,
                            dir::MemberSlot::Constructor | dir::MemberSlot::New
                        )
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                DefinitionConstructSource::Class(constructors)
            }
            // copy newtype backing operand
            Definition::Newtype(definition) => DefinitionConstructSource::Newtype(definition.value),
            // no candidates
            _ => DefinitionConstructSource::Absent,
        };

        match source {
            // select class constructors
            DefinitionConstructSource::Class(constructors) => {
                self.class_construct_candidates(module, symbol, arguments, constructors)
            }
            // select newtype wrapper constructor
            DefinitionConstructSource::Newtype(backing) => {
                self.newtype_construct_candidates(module, origin, symbol, arguments, backing)
            }
            // no candidates
            DefinitionConstructSource::Absent => Ok(ConstructCandidates::Absent),
        }
    }

    /// Return construct candidates from one class definition.
    fn class_construct_candidates(
        &mut self,
        module: ModuleId,
        class: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        constructors: Vec<crate::check::MethodDefinition>,
    ) -> CompilerResult<ConstructCandidates> {
        if constructors.is_empty() {
            return Ok(ConstructCandidates::Absent);
        }
        let substitution = self.generic_substitution(class, arguments)?;
        let instance = if arguments.is_empty() {
            None
        } else {
            let Some(template) = self.inference.symbol_generic_template(class) else {
                return Err(CompilerError::Internal {
                    message: format!("generic arguments supplied for non-generic class {class:?}"),
                });
            };

            Some(GenericInstance::new(template, arguments.to_vec().into()))
        };
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
            let CallableSignature::Present(function) =
                self.callable_signature(module, constructor.ty)?
            else {
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

    /// Return construct candidates from one newtype definition.
    fn newtype_construct_candidates(
        &mut self,
        module: ModuleId,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        backing: TypeOperand,
    ) -> CompilerResult<ConstructCandidates> {
        let substitution = self.generic_substitution(symbol, arguments)?;
        let backing = self.substitute_type_operand(module, &substitution, backing)?;
        let result = self.inference.push_term(TypeTerm::Reference {
            origin,
            symbol,
            arguments: arguments.to_vec(),
        });
        let instance = if arguments.is_empty() {
            None
        } else {
            let Some(template) = self.inference.symbol_generic_template(symbol) else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "generic arguments supplied for non-generic newtype {symbol:?}"
                    ),
                });
            };

            Some(GenericInstance::new(template, arguments.to_vec().into()))
        };
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Default::default(),
            this_parameter: None,
            parameters: SmallVec::from_vec(vec![FunctionParameter {
                ty: backing,
                static_parameter: None,
                is_optional: false,
                is_rest: false,
            }]),
            return_type: Some(result.into()),
            is_generator: false,
        };

        Ok(ConstructCandidates::Present(SmallVec::from_vec(vec![
            ConstructCandidate {
                target: ConstructTargetResolution::Newtype { symbol, instance },
                function,
            },
        ])))
    }

    /// Return the already chosen decision for one construct expression.
    fn selected_construct(&self, construct: &ConstructTerm) -> Option<CallableDispatch> {
        let Some(decision) = self.inference.construct(construct.source) else {
            return None;
        };

        let selection = match decision {
            ConstructDecision::Resolved(resolution) => {
                CallableDispatch::construct_selected(resolution.target, resolution.function)
            }
            ConstructDecision::Rejected(failure) => CallableDispatch::construct_rejected(failure),
        };

        Some(selection)
    }
}
