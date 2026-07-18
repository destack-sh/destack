use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CallableArgument, CandidateOutcome, CandidatePass, CandidateVerdict, Origin,
    ProbeReason, SignatureRejection, SignatureSelection, TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

/// One newtype backing alternative and its matching signature.
#[derive(Debug, Clone, Copy)]
struct NewtypeCandidate {
    /// The reduced backing alternative.
    backing: dir::GlobalTypeId,
    /// The signature used for argument matching.
    signature: dir::GlobalTypeId,
}

/// Result of matching arguments against one newtype.
pub(in crate::check) enum NewtypeMatch {
    /// One backing alternative accepted the arguments.
    Selected {
        /// The durable nominal selection.
        selection: dir::NewtypeSelection,
        /// The selected parameter types.
        parameters: SmallVec<[dir::FunctionParameterType; 4]>,
        /// The instantiated nominal return type.
        return_type: dir::GlobalTypeId,
    },
    /// No backing alternative accepted the arguments.
    Rejected(NewtypeRejection),
}

/// Reason no backing alternative accepted the supplied arguments.
pub(in crate::check) enum NewtypeRejection {
    /// One signature produced a specific rejection.
    Signature(SignatureRejection),
    /// Candidate signatures produced overload diagnostics.
    Candidates(Vec<String>),
}

impl BodyState<'_, '_> {
    /// Match supplied arguments against one newtype's backing alternatives.
    pub(in crate::check) fn match_newtype(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<NewtypeMatch>> {
        let module = origin.module();
        let arguments = self.callable_arguments(module, argument_nodes)?;

        // require the nominal definition established by the declaration walk
        let Some(dir::Definition::Newtype(definition)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("newtype selection target {symbol:?} has no newtype definition"),
            });
        };
        let backing = definition.backing;

        // instantiate the nominal return from written or expected arguments
        let generic_parameters = self
            .symbol_template(symbol)?
            .map(|template| self.generic_template_parameters(template))
            .unwrap_or_default();
        let type_arguments = if type_arguments.is_empty() {
            answer!(self.expected_newtype_arguments(origin, symbol, expected_return)?)
        } else {
            type_arguments.to_vec()
        };
        let return_arguments = generic_parameters
            .iter()
            .copied()
            .map(|parameter| self.intern_type(module, dir::Type::Parameter(parameter)))
            .collect::<CompilerResult<Vec<_>>>()?;
        let return_arguments = self.intern_type_ids(module, &return_arguments)?;
        let return_type = self.intern_type(
            module,
            dir::Type::Instance(dir::GenericInstance {
                symbol,
                arguments: return_arguments,
            }),
        )?;

        // build one signature candidate per backing alternative
        let candidates = answer!(self.newtype_candidates(origin, backing, return_type)?);

        // keep the first viable candidate, or the first ambiguous candidate if none decide
        let is_single_candidate = candidates.len() == 1;
        let mut winner = None;
        let mut ambiguous = None;
        let mut notes = Vec::new();
        for candidate in candidates.iter().copied() {
            if is_single_candidate {
                winner = Some(candidate);
                break;
            }
            let (verdict, rejection) = self.probe_candidate_noted(
                ProbeReason::Signature,
                |state| {
                    state.match_newtype_candidate(
                        CandidatePass::Winnow,
                        origin,
                        candidate,
                        &generic_parameters,
                        &type_arguments,
                        &arguments,
                        expected_return,
                    )
                },
                |state, rejection| {
                    Ok(state.check.describe_signature_rejection(
                        module,
                        candidate.signature,
                        rejection,
                    ))
                },
            )?;
            match verdict {
                CandidateVerdict::Rejected => notes.extend(rejection),
                CandidateVerdict::Viable => {
                    winner = Some(candidate);
                    break;
                }
                CandidateVerdict::Ambiguous => {
                    ambiguous.get_or_insert(candidate);
                }
            }
        }

        // confirm the selected candidate outside speculative state
        let mut selected = None;
        let mut signature_rejection = None;
        if let Some(candidate) = winner.or(ambiguous) {
            let matched = self.match_newtype_candidate(
                CandidatePass::Confirm,
                origin,
                candidate,
                &generic_parameters,
                &type_arguments,
                &arguments,
                expected_return,
            )?;
            match answer!(matched) {
                CandidateOutcome::Accepted(signature) => selected = Some((candidate, signature)),
                CandidateOutcome::Rejected(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    signature_rejection = Some(rejection);
                }
                CandidateOutcome::Rejected(_) => {}
            }
        }

        // preserve one specific rejection or bounded candidate diagnostics
        let Some((candidate, signature)) = selected else {
            let rejection = match signature_rejection {
                Some(rejection) => NewtypeRejection::Signature(rejection),
                None => {
                    notes.truncate(4);
                    NewtypeRejection::Candidates(notes)
                }
            };

            return Ok(Answer::Ready(NewtypeMatch::Rejected(rejection)));
        };

        // substitute the exact backing with the selected generic arguments
        let substitution = TypeSubstitution {
            parameters: signature
                .generic_arguments
                .iter()
                .map(|binding| binding.parameter)
                .collect(),
            arguments: signature
                .generic_arguments
                .iter()
                .map(|binding| binding.argument)
                .collect(),
            receiver: None,
        };
        let backing = self.substitute_type(module, candidate.backing, &substitution)?;
        let selection = dir::NewtypeSelection {
            symbol,
            backing,
            generic_arguments: signature.generic_arguments.clone(),
        };

        Ok(Answer::Ready(NewtypeMatch::Selected {
            selection,
            parameters: signature.parameters,
            return_type: signature.return_type,
        }))
    }

    /// Build argument matching candidates from one newtype backing.
    fn newtype_candidates(
        &mut self,
        origin: Origin,
        backing: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[NewtypeCandidate; 2]>>> {
        let backing = answer!(self.reduce_type_head(origin, backing)?);
        let mut backings = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[backing]);

        // try each union arm before the complete union domain
        if let dir::Type::Union(union) = self.ty(backing)? {
            let elements = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                self.type_ids(backing.module_id, union.elements)?,
            );
            backings.clear();
            backings.reserve(elements.len() + 1);
            for element in elements {
                backings.push(answer!(self.reduce_type_head(origin, element)?));
            }
            backings.push(backing);
        }

        // map scalar and tuple backings onto ordinary callable signatures
        let module = origin.module();
        let mut candidates = SmallVec::<[NewtypeCandidate; 2]>::with_capacity(backings.len());
        for backing in backings {
            let parameters = match self.ty(backing)? {
                dir::Type::Tuple(tuple) => self
                    .tuple_elements(backing.module_id, tuple.elements)?
                    .iter()
                    .map(|element| dir::FunctionParameterType {
                        ty: element.ty,
                        is_optional: element.is_optional,
                        is_rest: element.is_rest,
                    })
                    .collect::<SmallVec<[_; 4]>>(),
                _ => SmallVec::from_slice(&[dir::FunctionParameterType {
                    ty: backing,
                    is_optional: false,
                    is_rest: false,
                }]),
            };
            let parameters = self.intern_parameters(module, &parameters)?;
            let function = dir::FunctionSignatureType {
                asynchrony: dir::Asynchrony::Sync,
                template: None,
                this_parameter: None,
                parameters,
                return_type: Some(return_type),
                is_generator: false,
            };
            let signature = self.intern_signature(module, function)?;
            candidates.push(NewtypeCandidate { backing, signature });
        }

        Ok(Answer::Ready(candidates))
    }

    /// Match one newtype backing candidate against supplied arguments.
    fn match_newtype_candidate(
        &mut self,
        pass: CandidatePass,
        origin: Origin,
        candidate: NewtypeCandidate,
        generic_parameters: &[dir::GlobalGenericParameterId],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[CallableArgument],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<CandidateOutcome<SignatureSelection, SignatureRejection>>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let Some(function) = self.signature_head(candidate.signature)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "newtype candidate signature {:?} is not callable",
                    candidate.signature
                ),
            });
        };

        self.match_signature(
            pass,
            origin,
            module,
            candidate.signature.module_id,
            source,
            generic_parameters,
            None,
            &[],
            type_arguments,
            &function,
            function.return_type,
            None,
            arguments,
            expected_return,
        )
    }

    /// Return implicit newtype arguments from a same-symbol expected result.
    fn expected_newtype_arguments(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Vec<dir::GlobalTypeId>>> {
        let Some(expected_return) = expected_return else {
            return Ok(Answer::Ready(Vec::new()));
        };
        let expected_return = answer!(self.reduce_type_head(origin, expected_return)?);
        let dir::Type::Instance(instance) = self.ty(expected_return)? else {
            return Ok(Answer::Ready(Vec::new()));
        };
        if instance.symbol != symbol {
            return Ok(Answer::Ready(Vec::new()));
        }

        Ok(Answer::Ready(
            self.type_ids(expected_return.module_id, instance.arguments)?
                .to_vec(),
        ))
    }
}
