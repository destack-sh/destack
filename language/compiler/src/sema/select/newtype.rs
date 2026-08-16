use destack_dir as dir;
use smallvec::SmallVec;

use destack_source::ModuleId;

use crate::sema::{
    BodyState, CallableArgument, Callee, CandidateOutcome, CandidateVerdict, CheckState,
    Expectation, ObligationCheck, Origin, Selection, SignatureMatch, SignatureRejection,
    SignatureSelection, TypeSubstitution, ValueUse,
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
    /// Backing selection is not provably unique.
    Ambiguous,
}

/// One decided newtype backing, replayed across construction sites.
#[derive(Debug, Clone)]
pub(in crate::sema) struct NewtypeInstance {
    /// The decided backing without per-site coercions.
    pub(in crate::sema) selection: NewtypeSignature,
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

        // select the backing once for closed operand lists, then replay it per site
        let selection_key = match arguments
            .iter()
            .map(|argument| argument.ty)
            .collect::<Option<SmallVec<[_; 8]>>>()
        {
            Some(argument_types) => {
                let mut operands = SmallVec::<[dir::GlobalTypeId; 8]>::new();
                operands.extend(type_arguments.iter().copied());
                operands.extend(argument_types);

                self.derive_selection_key(
                    origin,
                    Callee::Newtype(symbol, overload),
                    expectation,
                    &operands,
                )?
            }
            None => None,
        };

        // replay the decided backing when this site repeats an earlier selection
        if let Some(key) = &selection_key
            && let Some(Selection::Newtype(instance)) = self.check.selections.get(key).cloned()
            && let Some(signature) =
                self.apply_newtype_instance(origin, instance.selection, &arguments)?
        {
            return Ok(NewtypeMatch::Selected(signature));
        }

        // require the nominal definition established by the declaration walk
        let Some(dir::Definition::Newtype(definition)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("newtype selection target {symbol:?} has no newtype definition"),
            });
        };
        let backing = definition.backing;

        // instantiate the nominal return from written or expected arguments
        let generic_parameters = match self.symbol_template(symbol)? {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let template = self.symbol_template(symbol)?;
        let type_arguments = if type_arguments.is_empty() {
            self.expected_newtype_arguments(
                symbol,
                expectation.map(|expectation| expectation.target),
            )?
        } else {
            type_arguments.to_vec()
        };
        let return_arguments = generic_parameters
            .iter()
            .copied()
            .map(|parameter| self.intern_type(dir::Type::Parameter(parameter)))
            .collect::<CompilerResult<Vec<_>>>()?;
        let return_arguments = self.intern_type_ids(&return_arguments)?;
        let return_type = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments: return_arguments,
        }))?;

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
                CandidateVerdict::Rejected => notes.extend(rejection),
                CandidateVerdict::Viable => {
                    if overload == NewtypeOverload::Unambiguous && selected_candidate.is_some() {
                        return Ok(NewtypeMatch::Rejected(NewtypeRejection::Ambiguous));
                    }
                    if overload == NewtypeOverload::Unambiguous {
                        selected_candidate = Some(candidate);
                    } else {
                        selected_candidate = Some(candidate);

                        break;
                    }
                }
                CandidateVerdict::Indeterminate => {
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
                    matched if is_single_candidate => CandidateOutcome::Accepted(matched),
                    SignatureMatch::Selected(selection) => {
                        CandidateOutcome::Accepted(SignatureMatch::Selected(selection))
                    }
                    SignatureMatch::ReturnMismatch(selection) => {
                        CandidateOutcome::Accepted(SignatureMatch::ReturnMismatch(selection))
                    }
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
                } if is_single_candidate => {
                    selected = Some((candidate, selection, Some(rejection)));
                }
                SignatureMatch::Inapplicable(rejection)
                    if is_single_candidate && rejection.is_precise() =>
                {
                    signature_rejection = Some(rejection);
                }
                SignatureMatch::Invalid { .. } => {}
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

        // decide the closed selection for replay at later sites
        if let NewtypeMatch::Selected(signature) = &matched
            && let Some(key) = selection_key
            && self.newtype_signature_is_closed(signature)?
        {
            let mut stored = signature.clone();
            stored.signature.coercions = SmallVec::new();
            self.check.selections.insert(
                key,
                Selection::Newtype(NewtypeInstance { selection: stored }),
            );
        }

        Ok(matched)
    }

    /// Replay one decided newtype selection, converting each argument.
    fn apply_newtype_instance(
        &mut self,
        origin: Origin,
        instance: NewtypeSignature,
        arguments: &[CallableArgument],
    ) -> CompilerResult<Option<NewtypeSignature>> {
        // convert each argument against the decided parameter list
        let mut signature = instance;
        for (index, argument) in arguments.iter().copied().enumerate() {
            let parameter = signature
                .signature
                .parameters
                .get(index)
                .or_else(|| signature.signature.parameters.last());
            let Some(parameter) = parameter else {
                return Ok(None);
            };
            let parameter_type = parameter.argument_type;
            let conversion =
                self.match_signature_argument(origin, index, argument, parameter_type)?;
            match conversion {
                Ok(Some(coercion)) => {
                    signature
                        .signature
                        .coercions
                        .push((argument.source, coercion));
                }
                Ok(None) => {}
                Err(_) => return Ok(None),
            }
        }

        Ok(Some(signature))
    }

    /// Return whether one newtype selection embeds no open inference variables.
    fn newtype_signature_is_closed(
        &mut self,
        signature: &NewtypeSignature,
    ) -> CompilerResult<bool> {
        // collect every type the selection embeds
        let mut types = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        types.push(signature.signature.return_type);
        types.push(signature.backing);
        types.extend(
            signature
                .signature
                .parameters
                .iter()
                .map(|parameter| parameter.parameter.ty),
        );
        types.extend(dir::GenericArgumentBinding::values(
            &signature.selection.arguments,
        ));

        // keep the whole selection open for one open variable
        for ty in types {
            if self.type_flags(ty)?.has_variable() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Derive and record one newtype's constructable backing alternatives.
    pub(in crate::sema) fn derive_newtype_constructors(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<ObligationCheck> {
        // read the raw declared entry without forcing constructor derivation
        let Some(dir::Definition::Newtype(definition)) = self.definition_maybe(symbol) else {
            return Ok(ObligationCheck::holds());
        };

        // leave already derived entries alone
        if !definition.constructors.is_empty() {
            return Ok(ObligationCheck::holds());
        }
        let backing = definition.backing;

        // instantiate the nominal return over its own parameters
        let template = self.symbol_template(symbol)?;
        let generic_parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let return_arguments = generic_parameters
            .iter()
            .copied()
            .map(|parameter| self.intern_type(dir::Type::Parameter(parameter)))
            .collect::<CompilerResult<Vec<_>>>()?;
        let return_arguments = self.intern_type_ids(&return_arguments)?;
        let return_type = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments: return_arguments,
        }))?;

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
        if let Some(dir::Definition::Newtype(definition)) = self.definition_mut(symbol) {
            definition.constructors = constructors;
        }

        Ok(ObligationCheck::holds())
    }

    /// Build argument matching candidates from one newtype backing.
    fn newtype_candidates(
        &mut self,
        backing: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
        template: Option<dir::GlobalGenericTemplateId>,
        overload: NewtypeOverload,
    ) -> CompilerResult<SmallVec<[NewtypeCandidate; 2]>> {
        let mut backings = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[backing]);

        // try each union arm before the complete union domain
        if let dir::Type::Union(union) = self.ty(backing)? {
            let elements = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                self.type_ids(backing.module_id, union.elements)?,
            );
            backings.clear();
            backings.reserve(elements.len() + 1);
            backings.extend(elements);
            if overload == NewtypeOverload::Ordered {
                backings.push(backing);
            }
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

        // NOTE #Suspicious: the derived check is discarded, so its failures never reach a report
        for symbol in newtypes {
            self.body().derive_newtype_constructors(symbol)?;
        }

        Ok(())
    }
}
