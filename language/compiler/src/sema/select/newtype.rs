use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Answer, ArgumentValue, CallableArgument, Callee, CheckOutcome, CheckState, Expectation, Origin,
    OverloadRule, OverloadSelection, SignatureMatch, SignatureSelection, TypeSubstitution,
    ValueUse,
};
use crate::{CompilerError, CompilerResult};

/// The number of rejected alternatives one report describes.
pub(in crate::sema) const REPORTED_REJECTIONS: usize = 4;

/// One newtype backing alternative and its matching signature.
#[derive(Debug, Clone, Copy)]
struct NewtypeCandidate {
    /// The reduced backing alternative.
    backing: dir::GlobalTypeId,
    /// The backing union arm this alternative selects, none for the whole backing.
    arm: Option<u32>,
    /// The signature used for argument matching.
    signature: dir::GlobalTypeId,
}

/// Result of matching arguments against one newtype.
#[allow(clippy::large_enum_variant)]
pub(in crate::sema) enum NewtypeMatch {
    /// One backing alternative accepts the arguments, with the outcome the site commits.
    Selected(NewtypeSignature, CheckOutcome),
    /// Every backing alternative rejects the arguments, described for the report.
    Rejected(Vec<String>),
    /// The sole backing alternative refuses the arguments with a reason already reported.
    Refused,
    /// Several backing alternatives accept an exclusive ask.
    Ambiguous,
}

/// One selected newtype backing signature.
#[derive(Debug, Clone)]
pub(in crate::sema) struct NewtypeSignature {
    /// The selected newtype declaration and its generic arguments.
    pub(in crate::sema) key: dir::InstanceKey,
    /// The selected instantiated backing alternative.
    pub(in crate::sema) backing: dir::GlobalTypeId,
    /// The position of the backing union arm the alternative enters, absent for the whole backing.
    pub(in crate::sema) arm: Option<u32>,
    /// The selected backing signature.
    pub(in crate::sema) signature: SignatureSelection,
}

impl CheckState<'_> {
    /// Match supplied arguments against one newtype's backing alternatives.
    pub(in crate::sema) fn match_newtype(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        argument_nodes: &[dir::LocalNodeId<dir::Argument>],
        type_arguments: &[dir::GlobalTypeId],
        expectation: Option<Expectation>,
        rule: OverloadRule,
        use_: ValueUse,
    ) -> CompilerResult<NewtypeMatch> {
        let module = origin.module();
        let arguments = self.callable_arguments(module, argument_nodes, use_)?;

        // ask the canonical construction goal once per equal ask
        let mut operands = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        operands.extend(type_arguments.iter().copied());
        let goal = match arguments
            .iter()
            .try_fold(&mut operands, |operands, argument| {
                let ArgumentValue::Typed(ty) = argument.value else {
                    return None;
                };
                operands.push(ty);
                Some(operands)
            }) {
            Some(_) => self.selection_goal(
                origin,
                Callee::Newtype(symbol, rule),
                expectation.and_then(Expectation::contextual_target),
                &operands,
            )?,
            None => None,
        };

        // require the nominal definition established by the declaration walk
        let declared = self.definition(symbol)?;
        let Some(dir::Definition::Newtype(definition)) = declared.as_deref() else {
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
                expectation.and_then(Expectation::contextual_target),
            )?
        } else {
            type_arguments.to_vec()
        };
        let return_type = self.nominal_return_type(symbol, &generic_parameters)?;

        // build one signature candidate per backing alternative
        let candidates = self.newtype_candidates(backing, return_type, template, rule)?;

        // select the remembered backing alone, else under the construction's rule
        let remembered = match &goal {
            Some(goal) => match self.answers.get(goal) {
                Some(Answer::Selection(position)) if *position < candidates.len() => {
                    Some(*position)
                }
                _ => None,
            },
            None => None,
        };
        let asked = match remembered {
            Some(position) => &candidates[position..=position],
            None => candidates.as_slice(),
        };
        let selection = self.select_callable(
            origin,
            asked,
            rule,
            |candidate| candidate.signature,
            |state, candidate| {
                state.match_newtype_candidate(
                    origin,
                    *candidate,
                    &type_arguments,
                    &arguments,
                    expectation,
                )
            },
        )?;
        let (position, candidate, signature, outcome) = match selection {
            OverloadSelection::Selected {
                position,
                candidate,
                signature,
                outcome,
            } => (
                remembered.unwrap_or(position),
                candidate,
                signature,
                outcome,
            ),
            OverloadSelection::Rejected(notes) => return Ok(NewtypeMatch::Rejected(notes)),
            OverloadSelection::Refused => return Ok(NewtypeMatch::Refused),
            OverloadSelection::Ambiguous => return Ok(NewtypeMatch::Ambiguous),
        };

        // remember the winning backing for equal asks
        if outcome == CheckOutcome::Holds
            && let Some(goal) = &goal
        {
            self.answers
                .entry(goal.clone())
                .or_insert(Answer::Selection(position));
        }

        // substitute the exact backing with the selected generic arguments
        let substitution = TypeSubstitution {
            bindings: signature.generic_arguments.iter().copied().collect(),
            receiver: None,
        };
        let backing = self.substitute_type(candidate.backing, &substitution)?;
        let signature = NewtypeSignature {
            key: dir::InstanceKey::new(symbol, signature.generic_arguments.clone()),
            backing,
            arm: candidate.arm,
            signature,
        };

        Ok(NewtypeMatch::Selected(signature, outcome))
    }

    /// Intern the nominal return applying one declaration over its own parameters.
    pub(in crate::sema) fn nominal_return_type(
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

        // apply the declaration over the collected arguments
        self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol,
            arguments,
        }))
    }

    /// Derive and commit one newtype's constructable backing alternatives.
    pub(in crate::sema) fn derive_newtype_constructors(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // read the raw declared entry without forcing constructor derivation
        let declared = self.definition(symbol)?;
        let Some(dir::Definition::Newtype(definition)) = declared.as_deref() else {
            return Ok(());
        };

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
            self.newtype_candidates(backing, return_type, template, OverloadRule::Ordered)?;
        let constructors = candidates
            .iter()
            .map(|candidate| dir::NewtypeConstructor {
                backing: candidate.backing,
                ty: candidate.signature,
            })
            .collect();

        self.module_mut(symbol.module_id)
            .members_tail
            .set_newtype_constructors(symbol, constructors);

        Ok(())
    }

    /// Build argument matching candidates from one newtype backing.
    fn newtype_candidates(
        &mut self,
        backing: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
        template: Option<dir::GlobalGenericTemplateId>,
        rule: OverloadRule,
    ) -> CompilerResult<SmallVec<[NewtypeCandidate; 2]>> {
        // try each union arm before the complete union domain
        let mut backings = SmallVec::<[(dir::GlobalTypeId, Option<u32>); 2]>::new();
        if let dir::Type::Union(union) = self.ty(backing)? {
            let elements = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(
                self.type_ids(backing.module_id, union.elements)?,
            );
            backings.reserve(elements.len() + 1);
            for (index, element) in elements.into_iter().enumerate() {
                backings.push((element, Some(index as u32)));
            }
            if rule == OverloadRule::Ordered {
                backings.push((backing, None));
            }
        } else {
            backings.push((backing, None));
        }

        // map scalar and tuple backings onto ordinary callable signatures
        let mut candidates = SmallVec::<[NewtypeCandidate; 2]>::with_capacity(backings.len());
        for (backing, arm) in backings {
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
                parks: false,
                asynchrony: dir::Asynchrony::Sync,
                template,
                arguments: dir::TypeListId::EMPTY,
                this_parameter: None,
                parameters,
                return_type: Some(return_type),
                is_generator: false,
                is_construct: false,
            };
            let signature = self.intern_signature(function)?;
            candidates.push(NewtypeCandidate {
                backing,
                arm,
                signature,
            });
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

        let return_type = function.return_type;

        // match the constructor signature against the written arguments
        self.match_signature(
            origin,
            candidate.signature.module_id,
            None,
            &[],
            type_arguments,
            &function,
            return_type,
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
        let expected_return = self.shallow_resolve(expected_return)?;
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
