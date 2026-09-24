use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseKind, CheckState, InterfaceMember, MemberCandidate, MemberLookup, Origin, Relation,
    RelationCheck, SignatureRejection, TypeSubstitution, Value, VariableKind, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// Interface protocol required by a generated operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct Protocol {
    /// The protocol interface symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The protocol generic arguments.
    pub(in crate::sema) arguments: Vec<dir::GlobalTypeId>,
    /// Checked argument types classifying candidates in decisions only.
    pub(in crate::sema) classification: Vec<dir::GlobalTypeId>,
}

impl Protocol {
    /// Create one complete applied protocol.
    fn new(symbol: dir::GlobalSymbolId, arguments: Vec<dir::GlobalTypeId>) -> Self {
        Self {
            symbol,
            arguments,
            classification: Vec::new(),
        }
    }

    /// Classify candidates against checked argument types in leading slots.
    fn classify(
        mut self,
        check: &mut CheckState<'_>,
        origin: Origin,
        classification: &[dir::GlobalTypeId],
    ) -> CompilerResult<Self> {
        self.classification = Vec::with_capacity(classification.len());
        for argument in classification {
            let asked = check.ask_argument(origin, *argument)?;
            self.classification.push(asked);
        }

        Ok(self)
    }

    /// Return the deciding protocol with classified leading arguments.
    fn classified(&self) -> Self {
        let mut arguments = self.arguments.clone();
        for (slot, argument) in arguments.iter_mut().zip(&self.classification) {
            *slot = *argument;
        }

        Self {
            symbol: self.symbol,
            arguments,
            classification: Vec::new(),
        }
    }

    /// Return this protocol as a generic instance interned into one module.
    pub(in crate::sema) fn instance(
        &self,
        check: &mut CheckState<'_>,
    ) -> CompilerResult<dir::GenericApplication> {
        // apply the protocol's own template to the written arguments
        let mut arguments = self.arguments.clone();
        match check.symbol_template(self.symbol)? {
            Some(template) => {
                let substitution = check.applied_substitution(template, &self.arguments)?;
                arguments = substitution
                    .bindings
                    .iter()
                    .map(|binding| binding.argument)
                    .collect();
            }
            None if self.arguments.is_empty() => {}
            // keep foreign templates symbolic while declaring
            None if !check.reads_module(self.symbol.module_id) => {}
            None => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "nongeneric protocol {:?} received {} arguments",
                        self.symbol,
                        self.arguments.len(),
                    ),
                });
            }
        }

        // intern the applied arguments
        let arguments = check.intern_type_ids(&arguments)?;

        Ok(dir::GenericApplication {
            symbol: self.symbol,
            arguments,
        })
    }
}

/// Protocol member accepted for a generated operation.
pub(in crate::sema) struct ProtocolMember {
    /// The member resolution.
    pub(in crate::sema) resolution: dir::MemberDecision,
    /// The member type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
}

/// Protocol call accepted for a generated operation.
#[derive(Debug, Clone)]
pub(in crate::sema) struct ProtocolCall {
    /// The member read selecting the call.
    pub(in crate::sema) member: dir::MemberDecision,
    /// The call resolution.
    pub(in crate::sema) resolution: dir::CallDecision,
    /// The call return type.
    pub(in crate::sema) return_type: dir::GlobalTypeId,
}

impl dir::TypeFold for ProtocolCall {
    fn map_types<E>(
        &mut self,
        map: &mut impl FnMut(dir::GlobalTypeId) -> Result<dir::GlobalTypeId, E>,
    ) -> Result<(), E> {
        self.member.map_types(map)?;
        self.resolution.map_types(map)?;
        self.return_type.map_types(map)
    }
}

impl CheckState<'_> {
    /// Return one complete protocol application backed by a language item.
    pub(in crate::sema) fn language_protocol(
        &mut self,
        item: dir::LanguageItem,
        written: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<Protocol> {
        // bind the written arguments to the language protocol's template
        let symbol = self.language_symbol(item)?;
        let arguments = match self.symbol_template(symbol)? {
            Some(template) => {
                let parameters = self.generic_template_parameters(template)?;
                let Some(substitution) = self.bind_explicit_arguments(
                    &parameters,
                    &written,
                    TypeSubstitution::default(),
                )?
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "language protocol {symbol:?} cannot bind {} arguments",
                            written.len(),
                        ),
                    });
                };

                substitution.arguments().collect()
            }
            None if written.is_empty() => Vec::new(),
            // carry the written arguments of foreign templates while declaring
            None if !self.reads_module(symbol.module_id) => written,
            None => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "nongeneric language protocol {symbol:?} received {} arguments",
                        written.len(),
                    ),
                });
            }
        };

        Ok(Protocol::new(symbol, arguments))
    }

    /// Infer one complete protocol application backed by a language item.
    fn infer_language_protocol(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        item: dir::LanguageItem,
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Protocol> {
        // require the language protocol's template
        let symbol = self.language_symbol(item)?;
        let Some(template) = self.symbol_template(symbol)? else {
            if written.is_empty() {
                return Ok(Protocol::new(symbol, Vec::new()));
            }

            // carry the written arguments of foreign templates while declaring
            if !self.reads_module(symbol.module_id) {
                return Ok(Protocol::new(symbol, written.to_vec()));
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "nongeneric language protocol {symbol:?} received {} arguments",
                    written.len(),
                ),
            });
        };

        // infer the protocol arguments against the receiver
        let parameters = self.generic_template_parameters(template)?;
        let Some(substitution) = self.instantiate_parameters(
            origin,
            &parameters,
            written,
            TypeSubstitution::default().with_receiver(receiver),
        )?
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "language protocol {symbol:?} cannot infer from {} arguments",
                    written.len(),
                ),
            });
        };

        // keep the protocol declaration's parameter bounds
        for constraint in
            self.substitute_application_constraints(origin, template, &substitution)?
        {
            self.push_relation(constraint)?;
        }

        // read the inferred arguments
        let arguments = substitution.arguments().collect();

        Ok(Protocol::new(symbol, arguments))
    }

    /// Select one call while inferring its complete language protocol application.
    pub(in crate::sema) fn select_language_protocol_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        item: dir::LanguageItem,
        written: &[dir::GlobalTypeId],
        classification: &[dir::GlobalTypeId],
        argument_sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Option<(Protocol, ProtocolCall)>> {
        // infer the protocol application, then select the call under it
        let protocol = self.infer_language_protocol(origin, lookup_receiver, item, written)?;
        let classified = protocol.clone().classify(self, origin, classification)?;
        let selected = self.select_protocol_call(
            origin,
            receiver,
            lookup_receiver,
            space,
            key,
            &classified,
            argument_sources,
        )?;
        if let Ok(call) = selected {
            return Ok(Some((classified, call)));
        }

        // probe the immutable borrows of the classified arguments
        let mut borrowed = Vec::with_capacity(classification.len());
        for argument in classification {
            borrowed.push(self.autoref_argument(origin, *argument)?);
        }
        if borrowed == classification {
            return Ok(None);
        }
        let classified = protocol.classify(self, origin, &borrowed)?;
        let selected = self.select_protocol_call(
            origin,
            receiver,
            lookup_receiver,
            space,
            key,
            &classified,
            argument_sources,
        )?;

        Ok(selected.ok().map(|call| (classified, call)))
    }

    /// Return the immutable borrow one argument lends to a protocol, at a region the call binds.
    fn autoref_argument(
        &mut self,
        origin: Origin,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let resolved = self.shallow_resolve(argument)?;
        if matches!(self.ty(resolved)?, dir::Type::Form(form) if matches!(form.form, dir::Form::Borrowed(_)))
        {
            return Ok(argument);
        }
        let region = self.open_memory_type(origin, dir::MemoryParameter::Region)?;
        let access = self.access_literal(dir::Access::Immutable)?;
        let borrowed = self.borrow_value(region, access, argument)?;

        Ok(borrowed)
    }

    /// Select one member while inferring its complete language protocol application.
    pub(in crate::sema) fn select_language_protocol_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        item: dir::LanguageItem,
        written: &[dir::GlobalTypeId],
        classification: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<(Protocol, ProtocolMember)>> {
        // infer the protocol application, then select the member under it
        let protocol = self.infer_language_protocol(origin, lookup_receiver, item, written)?;
        let protocol = protocol.classify(self, origin, classification)?;
        let selected =
            self.select_protocol_member(origin, receiver, lookup_receiver, space, key, &protocol)?;

        Ok(selected.map(|member| (protocol, member)))
    }

    /// Select one protocol member by key.
    pub(in crate::sema) fn select_protocol_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<Option<ProtocolMember>> {
        // use receiver adjustments for instance members and exact lookup for associated members
        let value = Value {
            ty: lookup_receiver,
            node: None,
            place: None,
            is_fresh: false,
        };
        let subject = self.member_subject(origin, receiver, lookup_receiver, space)?;
        let candidates = self.match_member(
            origin,
            origin.module(),
            value,
            subject,
            key,
            dir::Access::Readonly,
            Some(protocol),
        )?;
        if candidates.is_empty() {
            return Ok(None);
        }

        // read the first candidate per runtime arm, joining several as one union
        let arms = candidates.arms();
        let mut accesses = Vec::with_capacity(arms.len());
        let mut types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for group in arms {
            let receiver = group.arm.map_or(receiver, |arm| arm.receiver);
            let Some(candidate) = group.candidates.first() else {
                return Ok(None);
            };
            let candidate = candidate.instantiate(origin, self)?;
            candidate.constrain(self)?;
            let access = self.protocol_member_access(receiver, &candidate)?;
            types.push(access.ty);
            accesses.push(access);
        }

        Ok(Some(match accesses.as_slice() {
            [_] => ProtocolMember {
                ty: types[0],
                resolution: dir::OperationResolution::One(accesses.remove(0)),
            },
            _ => {
                let ty = self.normalized_union_type(types)?;

                ProtocolMember {
                    resolution: dir::OperationResolution::Union { arms: accesses, ty },
                    ty,
                }
            }
        }))
    }

    /// Select one protocol method call by key.
    pub(in crate::sema) fn select_protocol_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
        argument_sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Result<ProtocolCall, SignatureRejection>> {
        // select the candidates implementing the requirement along the receiver's dereferences
        let value = Value {
            ty: lookup_receiver,
            ..receiver
        };
        let subject = self.member_subject(origin, receiver.ty, lookup_receiver, space)?;
        let candidates = self.match_member(
            origin,
            origin.module(),
            value,
            subject,
            key,
            dir::Access::Readonly,
            Some(protocol),
        )?;
        if candidates.is_empty() {
            return Ok(Err(SignatureRejection::Inapplicable));
        }

        // select the first accepting candidate per runtime arm, joining several as one union
        let arms = candidates.arms();
        let mut members = Vec::with_capacity(arms.len());
        let mut calls = Vec::with_capacity(arms.len());
        let mut returns = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for group in arms {
            let receiver = match group.arm {
                Some(arm) => Value {
                    ty: arm.receiver,
                    ..receiver
                },
                None => receiver,
            };
            let mut selected = Err(SignatureRejection::Inapplicable);
            for candidate in group.candidates {
                selected = self.select_protocol_call_candidate(
                    origin,
                    receiver,
                    argument_sources,
                    candidate,
                )?;
                if selected.is_ok() {
                    break;
                }
            }
            let (member, call) = match selected {
                Ok(selected) => selected,
                Err(rejection) => return Ok(Err(rejection)),
            };
            returns.push(call.return_type);
            members.push(member);
            calls.push(call);
        }

        // join the arms, arms agreeing on one call as that call
        Ok(Ok(match calls.as_slice() {
            [first, rest @ ..] if rest.iter().all(|call| call == first) => ProtocolCall {
                member: dir::OperationResolution::One(members.remove(0)),
                resolution: dir::OperationResolution::One(calls.remove(0)),
                return_type: returns[0],
            },
            _ => {
                let member_types = members
                    .iter()
                    .map(|member| member.ty)
                    .collect::<SmallVec<[dir::GlobalTypeId; 4]>>();
                let member_type = self.normalized_union_type(member_types)?;
                let return_type = self.normalized_union_type(returns)?;

                ProtocolCall {
                    member: dir::OperationResolution::Union {
                        arms: members,
                        ty: member_type,
                    },
                    resolution: dir::OperationResolution::Union {
                        arms: calls,
                        ty: return_type,
                    },
                    return_type,
                }
            }
        }))
    }

    /// Look up members implementing one protocol on the exact receiver type.
    pub(in crate::sema) fn lookup_protocol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<MemberLookup> {
        // reduce the receiver and classify the requested protocol
        let receiver = self.normalize(origin, receiver)?;
        let receiver = if self.is_literal_shape(receiver)? {
            self.widen_type(receiver)?
        } else {
            receiver
        };
        let interface = protocol.classified().instance(self)?;
        let interface_type = self.intern_type(dir::Type::Application(interface))?;
        let requirements = self.interface_members(interface_type, receiver, space, key)?;
        if requirements.is_empty() {
            return Ok(MemberLookup::default());
        }

        // probe exact conformance and retain the selected declaration
        let selected = self.decide_deduction(|state| {
            let implementation = state.decide_extension_implementation(
                origin,
                interface_type.module_id,
                receiver,
                &interface,
                None,
            )?;
            let cause = state.intern_cause(Cause::root(origin, CauseKind::Expression));
            let verdict =
                state.constrain_type(origin, cause, Relation::Subtype, receiver, interface_type)?;
            let winner = implementation.winner.map(|winner| winner.extension);

            Ok((verdict != Verdict::Fails).then_some(winner))
        })?;
        let Some(winner) = selected else {
            return Ok(MemberLookup::default());
        };

        // confirm only the selected implementation and collect its members once
        let mut candidates = if let Some(winner) = winner {
            let implementation = self.confirm_extension_implementation(
                origin,
                interface_type.module_id,
                receiver,
                &interface,
                winner,
            )?;
            let Some(source) = implementation.winner else {
                return Err(CompilerError::Internal {
                    message: "a selected protocol implementation no longer applies".to_string(),
                });
            };
            self.extension_candidates(origin, receiver, receiver, &source, space, Some(key))?
                .into_iter()
                .map(|(_, candidate)| candidate)
                .collect()
        } else {
            Vec::new()
        };

        // read requirements implemented by inherited declaration members
        let is_inherited = candidates.is_empty();
        if is_inherited {
            candidates = self
                .lookup_visible_member(origin, module, receiver, space, key)?
                .candidates;
        }
        let mut candidates = self.conformance_candidates(
            receiver,
            winner,
            interface.symbol,
            &requirements,
            candidates.into(),
        )?;

        // retain the selected protocol's constraints on its candidates
        let instance = protocol.instance(self)?;
        let instance = self.intern_type(dir::Type::Application(instance))?;
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        for candidate in &mut candidates.candidates {
            let Some(declared) = candidate.declaration_mut() else {
                return Err(CompilerError::Internal {
                    message: "a protocol member without a declaration".to_string(),
                });
            };
            declared.bounds.push(RelationCheck::new(
                origin,
                Relation::Subtype,
                receiver,
                interface_type,
                cause,
            ));
            if is_inherited {
                declared.bounds.push(RelationCheck::new(
                    origin,
                    Relation::Storable,
                    receiver,
                    interface_type,
                    cause,
                ));
            }
            if !protocol.classification.is_empty() {
                declared.bounds.push(RelationCheck::new(
                    origin,
                    Relation::Equal,
                    interface_type,
                    instance,
                    cause,
                ));
            }
        }

        Ok(candidates)
    }

    /// Keep the members one receiver's conformances select for an interface requirement.
    fn conformance_candidates(
        &mut self,
        receiver: dir::GlobalTypeId,
        extension: Option<dir::GlobalSymbolId>,
        interface: dir::GlobalSymbolId,
        requirements: &[InterfaceMember],
        lookup: MemberLookup,
    ) -> CompilerResult<MemberLookup> {
        // collect the members selected by the receiver and each declaring owner
        let mut conformers = FxIndexSet::default();
        if let Some(instance) = self.apparent_instance(receiver)? {
            self.collect_conformance_members(instance.symbol, interface, &mut conformers)?;
        }
        if let Some(extension) = extension {
            self.collect_conformance_members(extension, interface, &mut conformers)?;
        }
        for candidate in &lookup.candidates {
            if let Some(declared) = candidate.declaration() {
                self.collect_conformance_members(declared.owner, interface, &mut conformers)?;
            }
        }

        // keep the selected members and the interface's own requirement declarations
        let sources = requirements
            .iter()
            .map(|requirement| requirement.source)
            .collect::<FxIndexSet<_>>();
        let mut kept = Vec::new();
        for candidate in lookup.candidates {
            let Some(declared) = candidate.declaration() else {
                continue;
            };
            let source = self.symbol_source(declared.symbol)?;
            if conformers.contains(&declared.symbol) || sources.contains(&source) {
                kept.push(candidate);
            }
        }

        Ok(kept.into())
    }

    /// Return the durable access one protocol member candidate resolves to.
    fn protocol_member_access(
        &self,
        receiver: dir::GlobalTypeId,
        candidate: &MemberCandidate,
    ) -> CompilerResult<dir::MemberAccess> {
        // resolve the candidate as a durable member access
        let Some(declared) = candidate.declaration() else {
            return Err(CompilerError::Internal {
                message: "a protocol member reads no declaration".to_string(),
            });
        };
        let ty = candidate.access.store();
        let target =
            dir::MemberTarget::Symbol(candidate.resolution_candidate(declared, receiver, ty));

        Ok(dir::MemberAccess::new(receiver, target, ty))
    }

    /// Collect the members one owner's conformances select for an interface's requirements.
    fn collect_conformance_members(
        &mut self,
        owner: dir::GlobalSymbolId,
        interface: dir::GlobalSymbolId,
        members: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<()> {
        // collect the members every conformance to the interface selects
        let declared = self.definition(owner)?;
        let conformances = match declared.as_deref() {
            Some(definition) => definition.implementations().cloned().collect::<Vec<_>>(),
            None => Vec::new(),
        };
        for conformance in conformances {
            let Some((_, applied)) = self.nominal_application_maybe(conformance.interface)? else {
                continue;
            };
            if applied.symbol != interface {
                continue;
            }
            let selected = self.conformance_members(conformance.source)?;
            members.extend(selected.iter().map(|member| member.member));
        }

        Ok(())
    }

    /// Select one protocol call from one candidate, deciding it before constraining.
    fn select_protocol_call_candidate(
        &mut self,
        origin: Origin,
        receiver: Value,
        argument_sources: &[dir::ArgumentSource],
        candidate: &MemberCandidate,
    ) -> CompilerResult<Result<(dir::MemberAccess, dir::Call), SignatureRejection>> {
        // decide the candidate before constraining it
        let attempt = |state: &mut Self| {
            let candidate = candidate.instantiate(origin, state)?;
            candidate.constrain(state)?;
            let member = state.protocol_member_access(receiver.ty, &candidate)?;
            let arguments = state.source_callable_arguments(origin, argument_sources)?;
            let call = state.select_member_call(origin, receiver, &candidate, &arguments)?;

            Ok(call.map(|call| (member, call)))
        };

        // run the attempt once the decision admits it
        let (attempted, verdict) = self.decide(attempt)?;
        match attempted {
            Ok(_) if verdict != Verdict::Fails => attempt(self),
            Ok(_) => Ok(Err(SignatureRejection::Inapplicable)),
            Err(rejection) => Ok(Err(rejection)),
        }
    }

    /// Return the type one argument asks an implementation with.
    pub(in crate::sema) fn ask_argument(
        &mut self,
        origin: Origin,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // bind a protocol operand to an owned value's type beneath its form
        let resolved = self.shallow_resolve(argument)?;
        if let dir::Type::Form(form) = self.ty(resolved)?
            && form.form == dir::Form::Owned
        {
            return self.ask_argument(origin, form.value);
        }

        // collect the literal leaves of the argument
        let leaves: SmallVec<[dir::GlobalTypeId; 4]> = match self.ty(resolved)? {
            dir::Type::Literal(_) => SmallVec::from_slice(&[resolved]),
            dir::Type::Union(union) => self.type_ids(resolved.module_id, union.elements)?.into(),
            _ => return Ok(argument),
        };
        let mut literals = SmallVec::<[dir::Literal; 4]>::new();
        for leaf in &leaves {
            let leaf = self.shallow_resolve(*leaf)?;
            let dir::Type::Literal(literal) = self.ty(leaf)? else {
                return Ok(argument);
            };
            literals.push(literal);
        }

        // ask numeric literals of one kind through one open variable of that kind
        let mut kinds = SmallVec::<[VariableKind; 4]>::new();
        for leaf in &leaves {
            kinds.extend(self.numeric_literal_kind(*leaf)?);
        }
        if kinds.len() == leaves.len() && kinds.iter().all(|kind| *kind == kinds[0]) {
            let variable = self.open_variable_of(origin, kinds[0]);

            return self.variable_type(variable);
        }

        // ask every other literal shape as its widened base
        let mut widened = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for literal in literals {
            widened.push(self.intern_type(literal.widen())?);
        }

        self.normalized_union_type(widened)
    }
}
