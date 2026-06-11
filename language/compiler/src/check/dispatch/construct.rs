use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CallArgument, CallableApplicability, CallableDispatch, CallableSignature, CheckState,
    ConstructDecision, ConstructFailure, ConstructTargetResolution, ConstructTerm, Definition,
    Dependency, FunctionParameter, FunctionTerm, GenericArgument, Origin, TermId, TypeOperand,
    TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Construct signature candidate selected from a callee type.
pub(in crate::check) struct ConstructCandidate {
    /// The selected construct target.
    pub(in crate::check) target: ConstructTargetResolution,
    /// The constructor function signature.
    pub(in crate::check) function: TermId<FunctionTerm>,
}

/// Construct signatures extracted from a callee type.
pub(in crate::check) enum ConstructCandidates {
    /// Candidate extraction is waiting for dependency.
    Pending(SmallVec<[Dependency; 2]>),
    /// The callee has no construct signatures.
    Absent,
    /// The callee has one or more construct signatures.
    Present(SmallVec<[ConstructCandidate; 4]>),
}

/// Transient construct dispatch while reducing a construct expression.
pub(in crate::check) enum ConstructDispatch {
    /// Dispatch is waiting for dependency.
    Pending(SmallVec<[Dependency; 2]>),
    /// Construct dispatch failed.
    Rejected(ConstructFailure),
    /// One construct candidate resolved.
    Selected {
        /// The resolved construct target.
        target: ConstructTargetResolution,
        /// The resolved constructor signature.
        function: FunctionTerm,
    },
}

impl ConstructDispatch {
    /// Return one construct dispatch from checked signature dispatch.
    fn from_signature(target: ConstructTargetResolution, signature: CallableApplicability) -> Self {
        match signature {
            CallableApplicability::Pending(blockers) => Self::Pending(blockers),
            CallableApplicability::Rejected(_) => Self::Rejected(ConstructFailure::NoMatch),
            CallableApplicability::Applicable { instance, function } => Self::Selected {
                target: target.with_application(instance),
                function,
            },
        }
    }

    /// Return this construct dispatch as a callable dispatch.
    pub(in crate::check) fn into_callable(self) -> CallableDispatch {
        match self {
            Self::Pending(blockers) => CallableDispatch::Pending(blockers),
            Self::Rejected(failure) => CallableDispatch::ConstructRejected(failure),
            Self::Selected { target, function } => {
                CallableDispatch::ConstructSelected { target, function }
            }
        }
    }
}

/// Construct form read from a declaration definition.
enum ConstructDefinition {
    /// Class constructor methods.
    Class(Vec<ClassConstructor>),
    /// Newtype backing operand.
    Newtype(TypeOperand),
    /// Definition has no construct form.
    Absent,
}

/// Class constructor data needed by construct dispatch.
struct ClassConstructor {
    /// The source method node.
    source: dir::GlobalNodeIdAny,
    /// The constructor method symbol.
    symbol: Option<dir::GlobalSymbolId>,
    /// The constructor function type.
    ty: TypeOperand,
}

impl CheckState<'_> {
    /// Select one runtime construct target.
    pub(in crate::check) fn select_construct_target(
        &mut self,
        origin: Origin,
        module: ModuleId,
        construct: TermId<ConstructTerm>,
        expected: Option<VariableId>,
    ) -> CompilerResult<ConstructDispatch> {
        let source = self.inference.term(construct).source;

        if let Some(selection) = self.selected_construct(source) {
            return Ok(selection);
        }

        let callee = self.inference.term(construct).callee;
        let Answer::Ready(callee) = self.reduce_type_operand(origin, callee)? else {
            return Ok(ConstructDispatch::Pending(callee.dependencies(self)));
        };
        let candidates = match self.construct_candidates(module, callee)? {
            ConstructCandidates::Pending(blockers) => {
                return Ok(ConstructDispatch::Pending(blockers));
            }
            ConstructCandidates::Absent => {
                return Ok(ConstructDispatch::Rejected(
                    ConstructFailure::NotConstructible,
                ));
            }
            ConstructCandidates::Present(candidates) => candidates,
        };
        let construct = self.inference.term(construct);
        let generic_arguments = construct
            .generic_arguments
            .iter()
            .copied()
            .collect::<SmallVec<[GenericArgument; 2]>>();
        let arguments = construct
            .arguments
            .iter()
            .copied()
            .collect::<SmallVec<[CallArgument; 4]>>();

        self.select_construct_candidate(
            origin,
            module,
            source,
            candidates,
            &generic_arguments,
            &arguments,
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
        arguments: &[CallArgument],
        expected: Option<VariableId>,
    ) -> CompilerResult<ConstructDispatch> {
        // run non-overloaded construction without speculative state
        if let [candidate] = candidates.as_slice() {
            return self.select_one_construct_candidate(
                origin,
                module,
                source,
                candidate,
                generic_arguments,
                arguments,
                expected,
            );
        }

        // choose the first compatible declaration order candidate
        for candidate in &candidates {
            let probe = self.inference.begin_probe();
            let result = self.select_one_construct_candidate(
                origin,
                module,
                source,
                candidate,
                generic_arguments,
                arguments,
                expected,
            )?;

            match result {
                ConstructDispatch::Selected { .. } => {
                    self.inference.commit_probe(probe)?;

                    return Ok(result);
                }
                ConstructDispatch::Pending(blockers) => {
                    let has_external_dependency =
                        probe.has_external_dependency(&blockers, &self.inference);
                    self.inference.drop_probe(probe)?;

                    // block declaration order only on state outside this candidate
                    if has_external_dependency {
                        return Ok(ConstructDispatch::Pending(blockers));
                    }
                }
                ConstructDispatch::Rejected(_) => {
                    self.inference.drop_probe(probe)?;
                }
            }
        }

        Ok(ConstructDispatch::Rejected(ConstructFailure::NoMatch))
    }

    /// Select one construct candidate inside the active inference probe.
    fn select_one_construct_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        candidate: &ConstructCandidate,
        generic_arguments: &[GenericArgument],
        arguments: &[CallArgument],
        expected: Option<VariableId>,
    ) -> CompilerResult<ConstructDispatch> {
        let target_symbol = Some(candidate.target.symbol());
        let instance = candidate.target.instance().cloned();
        let signature = self.select_callable_signature(
            origin,
            module,
            source,
            target_symbol,
            instance,
            candidate.function,
            generic_arguments,
            arguments,
            expected,
            None,
        )?;

        let dispatch = ConstructDispatch::from_signature(candidate.target.clone(), signature);

        Ok(dispatch)
    }

    /// Return construct candidates from one callee type.
    pub(in crate::check) fn construct_candidates(
        &mut self,
        module: ModuleId,
        callee: TypeOperand,
    ) -> CompilerResult<ConstructCandidates> {
        let Some(term) = self.type_operand_term_id(callee)? else {
            return Ok(ConstructCandidates::Pending(callee.dependencies(self)));
        };
        let candidates = match self.inference.term(term) {
            TypeTerm::Reference {
                origin,
                symbol,
                arguments,
            } => {
                let origin = *origin;
                let symbol = *symbol;
                let arguments = arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[GenericArgument; 2]>>();

                self.reference_construct_candidates(module, origin, symbol, &arguments)?
            }
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
        let Some(definition) = self.definitions.definition(symbol) else {
            return Ok(ConstructCandidates::Absent);
        };
        let construct_definition = match definition {
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
                    .map(|method| ClassConstructor {
                        source: method.source,
                        symbol: method.symbol,
                        ty: method.ty,
                    })
                    .collect::<Vec<_>>();

                ConstructDefinition::Class(constructors)
            }
            // copy newtype backing operand
            Definition::Newtype(definition) => ConstructDefinition::Newtype(definition.value),
            // no candidates
            _ => ConstructDefinition::Absent,
        };

        match construct_definition {
            // select class constructors
            ConstructDefinition::Class(constructors) => {
                self.class_construct_candidates(module, origin, symbol, arguments, constructors)
            }
            // select newtype wrapper constructor
            ConstructDefinition::Newtype(backing) => {
                self.newtype_construct_candidates(module, origin, symbol, arguments, backing)
            }
            // no candidates
            ConstructDefinition::Absent => Ok(ConstructCandidates::Absent),
        }
    }

    /// Return construct candidates from one class definition.
    fn class_construct_candidates(
        &mut self,
        module: ModuleId,
        origin: Origin,
        class: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        constructors: Vec<ClassConstructor>,
    ) -> CompilerResult<ConstructCandidates> {
        if constructors.is_empty() {
            return Ok(ConstructCandidates::Absent);
        }
        let substitution = self.generic_substitution(class, arguments)?;
        let instance = self.generic_instance_from_substitution(class, &substitution)?;
        let mut candidates = Vec::with_capacity(constructors.len());

        // convert constructor method types to construct signatures
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
                self.callable_signature(origin, module, constructor.ty)?
            else {
                continue;
            };
            let function = if substitution.is_empty() {
                function
            } else {
                let function = self.substitute_function_term(module, function, &substitution)?;

                self.inference.push_term(function)
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
            arguments: arguments.iter().copied().collect(),
        });
        let instance = self.generic_instance_from_substitution(symbol, &substitution)?;
        let function = FunctionTerm {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Default::default(),
            this_parameter: None,
            parameters: SmallVec::from_vec(vec![FunctionParameter {
                ty: backing,
                static_parameter: None,
                is_inferred: false,
                is_optional: false,
                is_rest: false,
            }]),
            return_type: Some(result.into()),
            is_generator: false,
        };
        let function = self.inference.push_term(function);

        Ok(ConstructCandidates::Present(SmallVec::from_vec(vec![
            ConstructCandidate {
                target: ConstructTargetResolution::Newtype { symbol, instance },
                function,
            },
        ])))
    }

    /// Return the already chosen decision for one construct expression.
    fn selected_construct(&self, source: dir::GlobalNodeIdAny) -> Option<ConstructDispatch> {
        let Some(decision) = self.inference.construct(source) else {
            return None;
        };

        let selection = match decision {
            ConstructDecision::Resolved(resolution) => ConstructDispatch::Selected {
                target: resolution.target.clone(),
                function: resolution.function.clone(),
            },
            ConstructDecision::Rejected(failure) => ConstructDispatch::Rejected(*failure),
        };

        Some(selection)
    }
}
