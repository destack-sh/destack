use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CallableArgument, CandidateVerdict, MemoryRank, Origin, ProbeReason,
    SignatureMatch, SignatureRejection, TypeSubstitution, VariableDomain, answer,
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
    Selected(NewtypeSignature),
    /// One backing alternative was selected but rejects the expected return.
    ReturnMismatch(NewtypeSignature),
    /// One backing alternative was selected but rejects the invocation.
    Invalid {
        /// The selected backing signature.
        signature: NewtypeSignature,
        /// The rejected invocation judgment.
        rejection: SignatureRejection,
        /// The inference variables owned by the rejected invocation.
        variables: VariableDomain,
    },
    /// No backing alternative accepted the arguments.
    Rejected(NewtypeRejection),
}

/// One selected newtype backing signature.
pub(in crate::check) struct NewtypeSignature {
    /// The durable nominal selection.
    pub(in crate::check) selection: dir::NewtypeSelection,
    /// The selected parameter types.
    pub(in crate::check) parameters: SmallVec<[dir::FunctionParameterType; 4]>,
    /// The instantiated nominal return type.
    pub(in crate::check) return_type: dir::GlobalTypeId,
}

/// Reason no backing alternative accepted the supplied arguments.
pub(in crate::check) enum NewtypeRejection {
    /// One signature produced a specific rejection.
    Signature(SignatureRejection),
    /// No backing alternative accepted the arguments.
    NoMatch(Vec<String>),
    /// Backing selection is not provably unique.
    Ambiguous,
}

/// How newtype backing alternatives are selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum NewtypeOverload {
    /// Select the first viable alternative in authored order.
    Ordered,
    /// Require exactly one independently viable alternative.
    Unambiguous,
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
        overload: NewtypeOverload,
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
        let candidates =
            answer!(self.newtype_candidates(origin, backing, return_type, overload)?);

        // select according to the construction's ambiguity rule
        let is_single_candidate = candidates.len() == 1;
        let mut winner: Option<(MemoryRank, NewtypeCandidate)> = None;
        let mut indeterminate = None;
        let mut notes = Vec::new();
        for candidate in candidates.iter().copied() {
            if is_single_candidate {
                winner = Some((MemoryRank::Exact, candidate));
                break;
            }
            let mut rank = MemoryRank::Exact;
            let (verdict, rejection) = answer!(self.probe_candidate_noted(
                ProbeReason::Signature,
                |state| {
                    let outcome = state.match_newtype_candidate(
                        origin,
                        candidate,
                        &generic_parameters,
                        &type_arguments,
                        &arguments,
                        expected_return,
                    )?;
                    match outcome {
                        Answer::Ready(matched) => {
                            if let SignatureMatch::Selected(selection) = &matched {
                                rank = answer!(selection.memory_rank(origin, &arguments, state)?);
                            }

                            Ok(Answer::Ready(matched.into_candidate()))
                        }
                        Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                    }
                },
                |state, rejection| {
                    Ok(state.check.describe_signature_rejection(
                        module,
                        candidate.signature,
                        rejection,
                    ))
                },
            )?);
            match verdict {
                CandidateVerdict::Rejected => notes.extend(rejection),
                CandidateVerdict::Viable => {
                    if overload == NewtypeOverload::Unambiguous && winner.is_some() {
                        return Ok(Answer::Ready(NewtypeMatch::Rejected(
                            NewtypeRejection::Ambiguous,
                        )));
                    }
                    if overload == NewtypeOverload::Unambiguous {
                        winner = Some((rank, candidate));
                    } else if rank == MemoryRank::Exact {
                        winner = Some((rank, candidate));
                        break;
                    } else if winner.is_none_or(|(best, _)| rank < best) {
                        winner = Some((rank, candidate));
                    }
                }
                CandidateVerdict::Indeterminate => {
                    if overload == NewtypeOverload::Unambiguous {
                        return Ok(Answer::Ready(NewtypeMatch::Rejected(
                            NewtypeRejection::Ambiguous,
                        )));
                    }
                    indeterminate.get_or_insert(candidate);
                }
            }
        }
        let winner = winner.map(|(_, candidate)| candidate);

        // confirm the selected candidate outside speculative state
        let mut selected = None;
        let mut is_return_mismatch = false;
        let mut signature_rejection = None;
        if let Some(candidate) = winner.or(indeterminate) {
            let matched = self.match_newtype_candidate(
                origin,
                candidate,
                &generic_parameters,
                &type_arguments,
                &arguments,
                expected_return,
            )?;
            match answer!(matched) {
                SignatureMatch::Selected(signature) => {
                    selected = Some((candidate, signature, None));
                }
                SignatureMatch::ReturnMismatch(signature) => {
                    selected = Some((candidate, signature, None));
                    is_return_mismatch = true;
                }
                SignatureMatch::Invalid {
                    selection,
                    rejection,
                    variables,
                } if is_single_candidate => {
                    selected = Some((candidate, selection, Some((rejection, variables))));
                }
                SignatureMatch::Inapplicable(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    signature_rejection = Some(rejection);
                }
                SignatureMatch::Invalid { variables, .. } => {
                    self.check.poison_variables(variables)?;
                }
                SignatureMatch::Inapplicable(_) => {}
            }
        }

        // preserve one specific rejection or bounded candidate diagnostics
        let Some((candidate, signature, rejection)) = selected else {
            let rejection = match signature_rejection {
                Some(rejection) => NewtypeRejection::Signature(rejection),
                None => {
                    notes.truncate(4);
                    NewtypeRejection::NoMatch(notes)
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

        let signature = NewtypeSignature {
            selection,
            parameters: signature.parameters,
            return_type: signature.return_type,
        };
        let matched = match rejection {
            Some((rejection, variables)) => NewtypeMatch::Invalid {
                signature,
                rejection,
                variables,
            },
            None if is_return_mismatch => NewtypeMatch::ReturnMismatch(signature),
            None => NewtypeMatch::Selected(signature),
        };

        Ok(Answer::Ready(matched))
    }

    /// Build argument matching candidates from one newtype backing.
    fn newtype_candidates(
        &mut self,
        origin: Origin,
        backing: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
        overload: NewtypeOverload,
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
            if overload == NewtypeOverload::Ordered {
                backings.push(backing);
            }
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
        origin: Origin,
        candidate: NewtypeCandidate,
        generic_parameters: &[dir::GlobalGenericParameterId],
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[CallableArgument],
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<SignatureMatch>> {
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
