use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Answer, BodyState, CallableArgument, Callee, CandidateOutcome, CheckState, Expectation, Origin,
    Selected, SignatureMatch, SignatureRejection, SignatureSelection, TypeSubstitution, ValueUse,
    Verdict,
};
use crate::{CompilerError, CompilerResult};

/// The number of rejected alternatives one report describes.
pub(in crate::sema) const REPORTED_REJECTIONS: usize = 4;

/// One newtype backing alternative and its matching signature.
#[derive(Debug, Clone, Copy)]
struct NewtypeCandidate {
    /// The reduced backing alternative.
    backing: dir::GlobalTypeId,
    /// The signature used for argument matching.
    signature: dir::GlobalTypeId,
}

/// Result of matching arguments against one newtype.
pub(in crate::sema) enum NewtypeMatch {
    /// One backing alternative accepted the arguments.
    Selected(NewtypeSignature),
    /// One backing alternative was selected but rejects the expected return.
    ReturnMismatch(NewtypeSignature),
    /// One backing alternative was selected but rejects the invocation.
    Invalid {
        /// The selected backing signature.
        signature: NewtypeSignature,
        /// The rejected invocation constraint.
        rejection: SignatureRejection,
    },
    /// No backing alternative accepted the arguments.
    Rejected(NewtypeRejection),
}

/// One selected newtype backing signature.
#[derive(Debug, Clone)]
pub(in crate::sema) struct NewtypeSignature {
    /// The selected newtype declaration and its generic arguments.
    pub(in crate::sema) selection: dir::Selection,
    /// The selected instantiated backing alternative.
    pub(in crate::sema) backing: dir::GlobalTypeId,
    /// The selected backing signature.
    pub(in crate::sema) signature: SignatureSelection,
}

/// Reason no backing alternative accepted the supplied arguments.
pub(in crate::sema) enum NewtypeRejection {
    /// One signature produced a specific rejection.
    Signature(SignatureRejection),
    /// No backing alternative accepted the arguments.
    NoMatch(Vec<String>),
    /// Several backing alternatives apply.
    Ambiguous,
}

/// One decided newtype backing, replayed across construction sites.
#[derive(Debug, Clone)]
pub(in crate::sema) struct NewtypeInstance {
    /// The decided backing without per-site coercions.
    pub(in crate::sema) selection: NewtypeSignature,
}

impl dir::TypeFold for NewtypeInstance {
    /// Map every type this construction carries.
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.selection.selection.map_types(map)?;
        self.selection.backing.map_types(map)?;
        self.selection.signature.map_types(map)
    }
}

/// How newtype backing alternatives are selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum NewtypeOverload {
    /// Select the first viable alternative in authored order.
    Ordered,
    /// Require exactly one independently viable alternative.
    Unambiguous,
}

impl BodyState<'_, '_> {
    /// Match supplied arguments against one newtype's backing alternatives.
    pub(in crate::sema) fn match_newtype(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
        overload: NewtypeOverload,
        use_: ValueUse,
    ) -> CompilerResult<NewtypeMatch> {
        let module = origin.module();
        self.register_argument_function_values(module, argument_nodes)?;
        let arguments = self.callable_arguments(module, argument_nodes, use_)?;

        // ask the canonical construction question once per equal ask
        let question = match arguments
            .iter()
            .map(|argument| argument.ty)
            .collect::<Option<SmallVec<[_; 8]>>>()
        {
            Some(argument_types) => {
                let mut operands = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                operands.extend(type_arguments.iter().copied());
                operands.extend(argument_types);

                self.check.selection_question(
                    origin,
                    Callee::Newtype(symbol, overload),
                    expectation.map(|expectation| expectation.target),
                    &operands,
                )?
            }
            None => None,
        };

        // replay the decided answer at this site's live roots
        if let Some((asked, canonical)) = &question
            && let Some(Answer::Selection(response)) = self.check.answers.get(asked).cloned()
        {
            let mark = self.check.infer.mark(&mut self.check.fulfill);
            let replayed = match self
                .check
                .instantiate_response(origin, canonical, &response)?
            {
                Selected::Newtype(instance) => {
                    self.apply_newtype_instance(origin, instance.selection, &arguments)?
                }
                _ => None,
            };
            match replayed {
                Some(signature) => {
                    self.check.infer.commit(mark, &mut self.check.fulfill);

                    return Ok(NewtypeMatch::Selected(signature));
                }
                None => {
                    let poison = self.check.intern_type(dir::Type::Error)?;
                    self.check
                        .infer
                        .rollback(mark, poison, &mut self.check.fulfill)?;
                }
            }
        }

        // the decision starts here; its queued checks feed the stored response
        let checks_before = self.check.fulfill.checks.count();

        // require the nominal definition established by the declaration walk
        let Some(dir::Definition::Newtype(definition)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("newtype selection target {symbol:?} has no newtype definition"),
            });
        };
        let backing = definition.backing;

        // instantiate the nominal return from written or expected arguments
        let template = self.symbol_template(symbol)?;
        let generic_parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let type_arguments = if type_arguments.is_empty() {
            self.expected_newtype_arguments(
                symbol,
                expectation.map(|expectation| expectation.target),
            )?
        } else {
            type_arguments.to_vec()
        };
        let return_type = self.nominal_return_type(symbol, &generic_parameters)?;

        // build one signature candidate per backing alternative
        let candidates = self.newtype_candidates(backing, return_type, template, overload)?;

        // select according to the construction's ambiguity rule
        let is_single_candidate = candidates.len() == 1;
        let mut selected_candidate = None;
        let mut notes = Vec::new();
        for candidate in candidates.iter().copied() {
            if is_single_candidate {
                selected_candidate = Some(candidate);
                break;
            }
            let (verdict, rejection) = self.probe_candidate_describing(
                |state| {
                    let outcome = state.match_newtype_candidate(
                        origin,
                        candidate,
                        &type_arguments,
                        &arguments,
                        expectation,
                    )?;
                    Ok(outcome.into_candidate())
                },
                |state, rejection| {
                    state
                        .check
                        .describe_signature_rejection(module, candidate.signature, rejection)
                },
            )?;
            match verdict {
                // keep a rejected alternative's description for the report
                Verdict::Fails => notes.extend(rejection),
                // a second viable alternative makes an unambiguous ask ambiguous
                Verdict::Holds => {
                    if overload == NewtypeOverload::Unambiguous && selected_candidate.is_some() {
                        return Ok(NewtypeMatch::Rejected(NewtypeRejection::Ambiguous));
                    }

                    selected_candidate = Some(candidate);
                    if overload != NewtypeOverload::Unambiguous {
                        break;
                    }
                }
                // an undecided alternative settles ordered asks and blocks unambiguous ones
                Verdict::Ambiguous => {
                    if overload == NewtypeOverload::Unambiguous {
                        return Ok(NewtypeMatch::Rejected(NewtypeRejection::Ambiguous));
                    }

                    selected_candidate = Some(candidate);

                    break;
                }
            }
        }

        // confirm the selected candidate outside speculative state
        let mut selected = None;
        let mut is_return_mismatch = false;
        let mut signature_rejection = None;
        if let Some(candidate) = selected_candidate {
            let matched = self.confirm_candidate(|state| {
                let matched = state.match_newtype_candidate(
                    origin,
                    candidate,
                    &type_arguments,
                    &arguments,
                    expectation,
                )?;

                Ok(match matched {
                    // a sole alternative keeps whatever it decided
                    matched if is_single_candidate => CandidateOutcome::Accepted(matched),
                    // an alternative that bound the arguments commits its inference
                    SignatureMatch::Selected(_) | SignatureMatch::ReturnMismatch(_) => {
                        CandidateOutcome::Accepted(matched)
                    }
                    // a refused alternative rolls back, keeping its reason
                    SignatureMatch::Invalid { rejection, .. }
                    | SignatureMatch::Inapplicable(rejection) => {
                        CandidateOutcome::Rejected(rejection)
                    }
                })
            })?;
            let Some(matched) = matched else {
                return Ok(NewtypeMatch::Rejected(NewtypeRejection::NoMatch(notes)));
            };
            match matched {
                // take the accepted construction
                SignatureMatch::Selected(signature) => {
                    selected = Some((candidate, signature, None));
                }
                // take a construction the expected result refused
                SignatureMatch::ReturnMismatch(signature) => {
                    selected = Some((candidate, signature, None));
                    is_return_mismatch = true;
                }
                // a sole alternative reports its own invocation rejection
                SignatureMatch::Invalid {
                    selection,
                    rejection,
                } if is_single_candidate => {
                    selected = Some((candidate, selection, Some(rejection)));
                }
                // a sole alternative reports its own precise refusal
                SignatureMatch::Inapplicable(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    signature_rejection = Some(rejection);
                }
                // leave a refused alternative to the shared report
                SignatureMatch::Invalid { .. } | SignatureMatch::Inapplicable(_) => {}
            }
        }

        // preserve one specific rejection or bounded candidate diagnostics
        let Some((candidate, signature, rejection)) = selected else {
            let rejection = match signature_rejection {
                Some(rejection) => NewtypeRejection::Signature(rejection),
                None => {
                    notes.truncate(REPORTED_REJECTIONS);
                    NewtypeRejection::NoMatch(notes)
                }
            };

            return Ok(NewtypeMatch::Rejected(rejection));
        };

        // substitute the exact backing with the selected generic arguments
        let substitution = TypeSubstitution {
            bindings: signature.generic_arguments.iter().copied().collect(),
            receiver: None,
        };
        let backing = self.substitute_type(candidate.backing, &substitution)?;

        // build the selected signature over the substituted backing
        let signature = NewtypeSignature {
            selection: dir::Selection::new(symbol, signature.generic_arguments.clone()),
            backing,
            signature,
        };

        // carry any rejection or return mismatch alongside the selection
        let matched = match rejection {
            Some(rejection) => NewtypeMatch::Invalid {
                signature,
                rejection,
            },
            None if is_return_mismatch => NewtypeMatch::ReturnMismatch(signature),
            None => NewtypeMatch::Selected(signature),
        };

        // remember the decision folded canonical over its ask
        if let NewtypeMatch::Selected(signature) = &matched
            && let Some((asked, canonical)) = &question
        {
            let mut stored = signature.clone();
            stored.signature.coercions = SmallVec::new();
            self.check.remember_answer(
                asked,
                canonical,
                checks_before,
                Selected::Newtype(NewtypeInstance { selection: stored }),
                Answer::Selection,
            )?;
        }

        Ok(matched)
    }

    /// Intern the nominal return applying one declaration over its own parameters.
    fn nominal_return_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        generic_parameters: &[dir::GlobalGenericParameterId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let arguments = generic_parameters
            .iter()
            .copied()
            .map(|parameter| self.intern_type(dir::Type::Parameter(parameter)))
            .collect::<CompilerResult<Vec<_>>>()?;
        let arguments = self.intern_type_ids(&arguments)?;

        self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))
    }

    /// Replay one decided newtype selection, converting each argument.
    fn apply_newtype_instance(
        &mut self,
        origin: Origin,
        mut instance: NewtypeSignature,
        arguments: &[CallableArgument],
    ) -> CompilerResult<Option<NewtypeSignature>> {
        // convert each argument against the decided backing signature
        let Some(applied) = self.apply_signature_instance(origin, instance.signature, arguments)?
        else {
            return Ok(None);
        };
        instance.signature = applied;

        Ok(Some(instance))
    }

    /// Derive and record one newtype's constructable backing alternatives.
    pub(in crate::sema) fn derive_newtype_constructors(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // read the raw declared entry without forcing constructor derivation
        let Some(dir::Definition::Newtype(definition)) = self.definition_maybe(symbol) else {
            return Ok(());
        };

        // leave already derived entries alone
        if !definition.constructors.is_empty() {
            return Ok(());
        }
        let backing = definition.backing;

        // instantiate the nominal return over its own parameters
        let template = self.symbol_template(symbol)?;
        let generic_parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let return_type = self.nominal_return_type(symbol, &generic_parameters)?;

        // derive one constructor per backing alternative in selection order
        let candidates =
            self.newtype_candidates(backing, return_type, template, NewtypeOverload::Ordered)?;
        let constructors = candidates
            .iter()
            .map(|candidate| dir::NewtypeConstructor {
                backing: candidate.backing,
                ty: candidate.signature,
            })
            .collect();

        // write the entries onto the checked definition
        let Some(dir::Definition::Newtype(definition)) = self.definition_mut(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("newtype {symbol:?} lost its definition during derivation"),
            });
        };
        definition.constructors = constructors;

        Ok(())
    }

    /// Build argument matching candidates from one newtype backing.
    fn newtype_candidates(
        &mut self,
        backing: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
        template: Option<dir::GlobalGenericTemplateId>,
        overload: NewtypeOverload,
    ) -> CompilerResult<SmallVec<[NewtypeCandidate; 2]>> {
        // try each union arm before the complete union domain
        let mut backings = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        if let dir::Type::Union(union) = self.ty(backing)? {
            let elements = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                self.type_ids(backing.module_id, union.elements)?,
            );
            backings.reserve(elements.len() + 1);
            backings.extend(elements);
            if overload == NewtypeOverload::Ordered {
                backings.push(backing);
            }
        } else {
            backings.push(backing);
        }

        // map scalar and tuple backings onto ordinary callable signatures
        let mut candidates = SmallVec::<[NewtypeCandidate; 2]>::with_capacity(backings.len());
        for backing in backings {
            let parameters = match self.ty(backing)? {
                dir::Type::Tuple(tuple) => self
                    .tuple_elements(backing.module_id, tuple.elements)?
                    .iter()
                    .map(|element| dir::FunctionParameterType {
                        name: None,
                        ty: element.ty,
                        is_optional: element.is_optional,
                        is_rest: element.is_rest,
                    })
                    .collect::<SmallVec<[_; 4]>>(),
                _ => SmallVec::from_slice(&[dir::FunctionParameterType {
                    name: None,
                    ty: backing,
                    is_optional: false,
                    is_rest: false,
                }]),
            };
            let parameters = self.intern_parameters(&parameters)?;
            let function = dir::FunctionSignatureType {
                asynchrony: dir::Asynchrony::Sync,
                template,
                this_parameter: None,
                parameters,
                return_type: Some(return_type),
                is_generator: false,
                is_construct: false,
            };
            let signature = self.intern_signature(function)?;
            candidates.push(NewtypeCandidate { backing, signature });
        }

        Ok(candidates)
    }

    /// Match one newtype backing candidate against supplied arguments.
    fn match_newtype_candidate(
        &mut self,
        origin: Origin,
        candidate: NewtypeCandidate,
        type_arguments: &[dir::GlobalTypeId],
        arguments: &[CallableArgument],
        expectation: Option<Expectation>,
    ) -> CompilerResult<SignatureMatch> {
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
            candidate.signature.module_id,
            None,
            &[],
            type_arguments,
            &function,
            function.return_type,
            None,
            arguments,
            expectation,
        )
    }

    /// Return implicit newtype arguments from a same-symbol expected result.
    fn expected_newtype_arguments(
        &mut self,
        symbol: dir::GlobalSymbolId,
        expected_return: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let Some(expected_return) = expected_return else {
            return Ok(Vec::new());
        };
        let dir::Type::Application(instance) = self.ty(expected_return)? else {
            return Ok(Vec::new());
        };
        if instance.symbol != symbol {
            return Ok(Vec::new());
        }

        Ok(self
            .type_ids(expected_return.module_id, instance.arguments)?
            .to_vec())
    }
}

impl CheckState<'_> {
    /// Record each declared newtype's constructor entries.
    pub(in crate::sema) fn derive_module_constructors(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<()> {
        // declarations leave derived entries to their checking pass
        if self.is_declaration() {
            return Ok(());
        }

        // derive entries beside each declared newtype
        let mut newtypes = Vec::new();
        for (symbol, definition) in self.module(module).iter_definitions() {
            if matches!(definition, dir::Definition::Newtype(_)) {
                newtypes.push(symbol);
            }
        }

        for symbol in newtypes {
            self.body().derive_newtype_constructors(symbol)?;
        }

        Ok(())
    }
}
