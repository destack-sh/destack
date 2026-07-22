use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CandidateOutcome, Cause, CauseKind, CheckState, DeclaredMember, Dependency,
    MemberCandidate, MemberLookup, Origin, Relation, SignatureMatch, SignatureSelection,
    TypeSubstitution, answer,
};

/// Interface protocol required by a generated operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct Protocol {
    /// The protocol interface symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The protocol generic arguments.
    pub(in crate::check) arguments: Vec<dir::GlobalTypeId>,
}

impl Protocol {
    /// Return one protocol interface instance.
    pub(in crate::check) fn new(
        symbol: dir::GlobalSymbolId,
        arguments: Vec<dir::GlobalTypeId>,
    ) -> Self {
        Self { symbol, arguments }
    }

    /// Return this protocol as a generic instance interned into one module.
    pub(in crate::check) fn instance(
        &self,
        check: &mut CheckState<'_>,
        module: ModuleId,
    ) -> CompilerResult<dir::GenericInstance> {
        let arguments = check.intern_type_ids(module, &self.arguments)?;

        Ok(dir::GenericInstance {
            symbol: self.symbol,
            arguments,
        })
    }
}

/// Protocol member accepted for a generated operation.
pub(in crate::check) struct ProtocolMember {
    /// The member resolution.
    pub(in crate::check) resolution: dir::MemberResolution,
    /// The member type.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The declaration that exposed the member.
    pub(in crate::check) owner: dir::GlobalSymbolId,
    /// The member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The generic argument bindings.
    pub(in crate::check) generic_arguments: Vec<dir::GenericArgumentBinding>,
}

/// Protocol implementation selected for one receiver.
pub(in crate::check) struct ProtocolImplementation {
    /// The selected protocol generic arguments.
    pub(in crate::check) arguments: Vec<dir::GlobalTypeId>,
}

/// Protocol call accepted for a generated operation.
pub(in crate::check) struct ProtocolCall {
    /// The call resolution.
    pub(in crate::check) resolution: dir::CallResolution,
    /// The call return type.
    pub(in crate::check) return_type: dir::GlobalTypeId,
}

impl CheckState<'_> {
    /// Return one protocol backed by a language item.
    pub(in crate::check) fn language_protocol(
        &self,
        item: dir::LanguageItem,
        arguments: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<Protocol> {
        Ok(Protocol::new(self.language_symbol(item)?, arguments))
    }
}

impl BodyState<'_, '_> {
    /// Select one protocol implementation for a receiver.
    pub(in crate::check) fn select_protocol_implementation(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<ProtocolImplementation>>> {
        let module = origin.module();
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let lookup_receiver = answer!(self.reduce_type_head(origin, lookup_receiver)?);
        let lookup_receiver = self.intern_apparent_type(module, lookup_receiver)?;
        let extension = self.select_extension_protocol_implementation(
            origin,
            module,
            lookup_receiver,
            protocol,
        )?;
        if !matches!(extension, Answer::Ready(None)) {
            return Ok(extension);
        }

        let Some(instance) = self.apparent_instance(lookup_receiver)? else {
            return Ok(Answer::Ready(None));
        };
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(Answer::Ready(None));
        };
        let heritages = definition
            .heritages()
            .into_iter()
            .cloned()
            .collect::<SmallVec<[_; 2]>>();
        let substitution = instance.substitution(self)?.with_receiver(receiver);

        self.select_heritage_protocol_implementation(
            origin,
            module,
            &substitution,
            heritages.iter(),
            protocol,
        )
    }

    /// Select one protocol implementation from extension candidates.
    fn select_extension_protocol_implementation(
        &mut self,
        origin: Origin,
        module: ModuleId,
        lookup_receiver: dir::GlobalTypeId,
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<ProtocolImplementation>>> {
        let extensions = self.visible_receiver_extensions(module, lookup_receiver)?;
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // try each visible extension as a protocol implementation candidate
        for extension_symbol in extensions {
            if self.is_absent_symbol(extension_symbol) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)?
            else {
                continue;
            };
            if !extension.is_visible_from(module) {
                continue;
            }

            let target_type = extension.target.r#type();
            let implements = extension.implements.clone();
            let template = self.symbol_template(extension_symbol)?;
            let Some(substitution) = answer!(self.match_extension_target(
                origin,
                lookup_receiver,
                template,
                target_type,
            )?) else {
                continue;
            };
            let implementation = self.select_heritage_protocol_implementation(
                origin,
                module,
                &substitution.with_receiver(lookup_receiver),
                implements.iter(),
                protocol,
            )?;
            match implementation {
                Answer::Ready(Some(implementation)) => {
                    return Ok(Answer::Ready(Some(implementation)));
                }
                Answer::Ready(None) => {}
                Answer::Pending(pending) => blockers.extend(pending),
            }
        }

        Ok(Answer::ready_unless_blocked(None, blockers))
    }

    /// Select protocol arguments from heritage clauses.
    fn select_heritage_protocol_implementation<'a>(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &TypeSubstitution,
        heritages: impl IntoIterator<Item = &'a dir::NominalHeritage>,
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<ProtocolImplementation>>> {
        // compare each implemented interface with the requested protocol
        for heritage in heritages {
            let implemented =
                answer!(self.substituted_heritage(origin, module, substitution, heritage)?);
            let instance = if implemented.symbol == protocol {
                Some(implemented)
            } else {
                answer!(self.heritage_instance(origin, module, &implemented, protocol)?)
            };

            if let Some(instance) = instance {
                let arguments = self.type_ids(module, instance.arguments)?.to_vec();

                return Ok(Answer::Ready(Some(ProtocolImplementation { arguments })));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Select one protocol member by key.
    pub(in crate::check) fn select_protocol_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        let module = origin.module();
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let lookup_receiver = answer!(self.reduce_type_head(origin, lookup_receiver)?);
        let lookup_receiver = self.intern_apparent_type(module, lookup_receiver)?;
        let extension = self.select_extension_protocol_member(
            origin,
            module,
            receiver,
            lookup_receiver,
            key,
            protocol,
        )?;
        if !matches!(extension, Answer::Ready(None)) {
            return Ok(extension);
        }

        let lookup = answer!(self.lookup_inherent_member(
            origin,
            module,
            lookup_receiver,
            dir::MemberSpace::Instance,
            key,
        )?);
        let candidates = match lookup {
            MemberLookup::Found(candidates) => candidates,
            MemberLookup::Field(_) | MemberLookup::Missing => return Ok(Answer::Ready(None)),
        };

        self.select_protocol_member_candidate(
            origin,
            module,
            receiver,
            lookup_receiver,
            protocol,
            candidates,
        )
    }

    /// Select one protocol method call by key.
    pub(in crate::check) fn select_protocol_call(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        protocol: &Protocol,
        argument_types: &[dir::GlobalTypeId],
        argument_sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        let module = origin.module();
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let lookup_receiver = answer!(self.reduce_type_head(origin, lookup_receiver)?);
        let lookup_receiver = self.intern_apparent_type(module, lookup_receiver)?;
        let extension = self.select_extension_protocol_call(
            origin,
            module,
            receiver,
            lookup_receiver,
            key,
            protocol,
            argument_types,
            argument_sources,
        )?;
        if !matches!(extension, Answer::Ready(None)) {
            return Ok(extension);
        }

        let lookup = answer!(self.lookup_inherent_member(
            origin,
            module,
            lookup_receiver,
            dir::MemberSpace::Instance,
            key,
        )?);
        let candidates = match lookup {
            MemberLookup::Found(candidates) => candidates,
            MemberLookup::Field(_) | MemberLookup::Missing => return Ok(Answer::Ready(None)),
        };

        self.select_protocol_call_candidate(
            origin,
            module,
            receiver,
            lookup_receiver,
            protocol,
            argument_types,
            argument_sources,
            candidates,
        )
    }

    /// Select one protocol call from extension candidates.
    fn select_extension_protocol_call(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        protocol: &Protocol,
        argument_types: &[dir::GlobalTypeId],
        argument_sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        self.select_extension_protocol(
            origin,
            module,
            lookup_receiver,
            key,
            protocol,
            |state, candidates| match state.select_protocol_call_member(
                origin,
                receiver,
                argument_types,
                argument_sources,
                candidates,
            )? {
                Answer::Ready(Some(call)) => Ok(Answer::Ready(CandidateOutcome::Accepted(call))),
                Answer::Ready(None) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
            },
        )
    }

    /// Select one protocol member from extension candidates.
    fn select_extension_protocol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        self.select_extension_protocol(
            origin,
            module,
            lookup_receiver,
            key,
            protocol,
            |state, candidates| match state
                .select_protocol_member_from_candidates(receiver, candidates)
            {
                Some(member) => Ok(Answer::Ready(CandidateOutcome::Accepted(member))),
                None => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
            },
        )
    }

    /// Select one extension protocol candidate.
    fn select_extension_protocol<T>(
        &mut self,
        origin: Origin,
        module: ModuleId,
        lookup_receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        protocol: &Protocol,
        mut select: impl FnMut(
            &mut BodyState<'_, '_>,
            Vec<MemberCandidate>,
        ) -> CompilerResult<Answer<CandidateOutcome<T, ()>>>,
    ) -> CompilerResult<Answer<Option<T>>> {
        let extensions = self.visible_receiver_extensions(module, lookup_receiver)?;
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // try each extension as one candidate transaction
        for extension_symbol in extensions {
            if self.is_absent_symbol(extension_symbol) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)?
            else {
                continue;
            };
            if !extension.is_visible_from(module) {
                continue;
            }

            let target_type = extension.target.r#type();
            let implements = extension.implements.clone();
            let definition_members = extension.members.clone();
            let members = answer!(self.protocol_extension_members(&definition_members, key)?);
            if members.is_empty() {
                continue;
            }

            let selected = self.confirm_candidate(|state| {
                let matched = state.match_extension_protocol_members(
                    origin,
                    module,
                    lookup_receiver,
                    extension_symbol,
                    target_type,
                    &implements,
                    &members,
                    protocol,
                )?;
                let candidates = match matched {
                    Answer::Ready(Some(candidates)) => candidates,
                    Answer::Ready(None) => {
                        return Ok(Answer::Ready(CandidateOutcome::Rejected(())));
                    }
                    Answer::Pending(pending) => return Ok(Answer::Pending(pending)),
                };

                select(state, candidates)
            })?;
            match selected {
                Answer::Ready(Some(selected)) => {
                    return Ok(Answer::Ready(Some(selected)));
                }
                Answer::Ready(None) => {}
                Answer::Pending(pending) => {
                    blockers.extend(pending);
                }
            }
        }

        Ok(Answer::ready_unless_blocked(None, blockers))
    }

    /// Match one extension implementation against a protocol member request.
    fn match_extension_protocol_members(
        &mut self,
        origin: Origin,
        module: ModuleId,
        lookup_receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        target_type: dir::GlobalTypeId,
        implements: &[dir::NominalHeritage],
        members: &[DeclaredMember],
        protocol: &Protocol,
    ) -> CompilerResult<Answer<Option<Vec<MemberCandidate>>>> {
        let template = self.symbol_template(extension_symbol)?;
        let Some(mut substitution) =
            answer!(self.match_extension_target(origin, lookup_receiver, template, target_type,)?)
        else {
            return Ok(Answer::Ready(None));
        };

        // bind protocol arguments that appear directly in implementation clauses
        if !answer!(self.bind_extension_protocol_arguments(
            origin,
            module,
            &mut substitution,
            implements,
            protocol,
        )?) {
            return Ok(Answer::Ready(None));
        }

        // prove the extension implements the requested protocol
        if !answer!(self.extension_protocol_holds(
            origin,
            module,
            &substitution,
            implements,
            protocol,
        )?) {
            return Ok(Answer::Ready(None));
        }

        let candidates = answer!(self.extension_member_candidates(
            origin,
            extension_symbol,
            &substitution,
            members,
        )?);
        if candidates.is_empty() {
            return Ok(Answer::Ready(None));
        }

        Ok(Answer::Ready(Some(candidates)))
    }

    /// Bind extension parameters that occur directly in requested protocol arguments.
    fn bind_extension_protocol_arguments(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &mut TypeSubstitution,
        implements: &[dir::NominalHeritage],
        protocol: &Protocol,
    ) -> CompilerResult<Answer<bool>> {
        if protocol.arguments.is_empty() {
            return Ok(Answer::Ready(true));
        }

        let source = self.origin_source_node(origin)?;
        let previous = substitution.parameters.len();

        // bind missing parameters from direct implemented protocol arguments
        for heritage in implements {
            // trailing defaulted interface parameters may stay unrequested
            if heritage.symbol != protocol.symbol
                || heritage.arguments.len() < protocol.arguments.len()
            {
                continue;
            }

            for (implemented, requested) in heritage.arguments.iter().zip(&protocol.arguments) {
                let implemented =
                    self.substitute_type(origin.module(), *implemented, substitution)?;
                let dir::Type::Parameter(parameter) = self.ty(implemented)? else {
                    continue;
                };
                if substitution.parameters.contains(&parameter) {
                    continue;
                }

                substitution.parameters.push(parameter);
                substitution.arguments.push(*requested);
            }
        }

        // check constraints introduced by protocol argument binding
        let mut decision = Answer::Ready(true);
        for index in previous..substitution.parameters.len() {
            let parameter = substitution.parameters[index];
            let argument = substitution.arguments[index];
            let Some(constraint) = self
                .generic_parameter(parameter)
                .and_then(|binding| binding.constraint)
            else {
                continue;
            };
            let constraint = self.substitute_type(origin.module(), constraint, substitution)?;
            let source_node = source.into_global(module);
            let source_origin = self.origin_at(origin, source_node)?;

            let cause = self.intern_cause(Cause::root(source_origin, CauseKind::Expression));

            decision = decision.and(self.constrain_type(
                cause,
                Relation::Satisfies,
                argument,
                constraint,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return declared extension members matching one protocol key.
    fn protocol_extension_members(
        &mut self,
        members: &[dir::DefinitionMember],
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Vec<DeclaredMember>>> {
        let mut matched = Vec::new();
        for member in members {
            let member = match self.declared_member(member)? {
                Answer::Ready(Some(member)) => member,
                Answer::Ready(None) => continue,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            if member.matches(dir::MemberSpace::Instance, key) {
                matched.push(member);
            }
        }

        Ok(Answer::Ready(matched))
    }

    /// Select one protocol member from already ordered candidates.
    fn select_protocol_member_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        protocol: &Protocol,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        for candidate in candidates {
            let satisfies = self.candidate_satisfies_protocol(
                origin,
                module,
                lookup_receiver,
                &candidate,
                protocol,
            )?;
            if !answer!(satisfies) {
                continue;
            }
            if let Some(member) = self.protocol_member_from_candidate(receiver, candidate) {
                return Ok(Answer::Ready(Some(member)));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Select the first symbol-backed protocol member from ordered candidates.
    fn select_protocol_member_from_candidates(
        &self,
        receiver: dir::GlobalTypeId,
        candidates: Vec<MemberCandidate>,
    ) -> Option<ProtocolMember> {
        for candidate in candidates {
            if let Some(member) = self.protocol_member_from_candidate(receiver, candidate) {
                return Some(member);
            }
        }

        None
    }

    /// Return one durable protocol member resolution.
    fn protocol_member_from_candidate(
        &self,
        receiver: dir::GlobalTypeId,
        candidate: MemberCandidate,
    ) -> Option<ProtocolMember> {
        let symbol = candidate.symbol?;
        let generic_arguments = candidate.generic_arguments;
        let target = dir::MemberTarget::Symbol(dir::MemberCandidate {
            receiver,
            adjustments: Vec::new(),
            space: candidate.space,
            owner: candidate.owner,
            symbol,
            ty: candidate.ty,
            generic_arguments: generic_arguments.clone(),
        });
        let resolution = dir::MemberResolution::new(receiver, target);

        Some(ProtocolMember {
            resolution,
            ty: candidate.ty,
            owner: candidate.owner,
            symbol,
            generic_arguments,
        })
    }

    /// Select one protocol call from already ordered candidates.
    fn select_protocol_call_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        protocol: &Protocol,
        argument_types: &[dir::GlobalTypeId],
        argument_sources: &[dir::ArgumentSource],
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        for candidate in candidates {
            let satisfies = self.candidate_satisfies_protocol(
                origin,
                module,
                lookup_receiver,
                &candidate,
                protocol,
            )?;
            if !answer!(satisfies) {
                continue;
            }
            let call = self.select_protocol_call_from_candidate(
                origin,
                receiver,
                argument_types,
                argument_sources,
                candidate,
            )?;
            if !matches!(call, Answer::Ready(None)) {
                return Ok(call);
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Select one protocol call after the owner has proved the protocol.
    fn select_protocol_call_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        argument_types: &[dir::GlobalTypeId],
        argument_sources: &[dir::ArgumentSource],
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        for candidate in candidates {
            let call = self.select_protocol_call_from_candidate(
                origin,
                receiver,
                argument_types,
                argument_sources,
                candidate,
            )?;
            if !matches!(call, Answer::Ready(None)) {
                return Ok(call);
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Select one already proved protocol call candidate.
    fn select_protocol_call_from_candidate(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        argument_types: &[dir::GlobalTypeId],
        argument_sources: &[dir::ArgumentSource],
        candidate: MemberCandidate,
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        let Some(symbol) = candidate.symbol else {
            return Ok(Answer::Ready(None));
        };
        let arguments = self.source_callable_arguments(origin, argument_types, argument_sources)?;
        let attempt = self.attempt_callable(
            origin,
            candidate.ty,
            Some(candidate.owner),
            Some(receiver),
            &candidate.generic_arguments,
            &[],
            &arguments,
            None,
        )?;
        let signature = match answer!(attempt) {
            SignatureMatch::Selected(signature) => signature,
            SignatureMatch::Invalid { .. }
            | SignatureMatch::ReturnMismatch(_)
            | SignatureMatch::Inapplicable(_) => return Ok(Answer::Ready(None)),
        };

        let mut generic_arguments = candidate.generic_arguments.clone();
        generic_arguments.extend_from_slice(&signature.generic_arguments);

        let resolution = self.protocol_call_resolution(
            receiver,
            candidate.owner,
            symbol,
            generic_arguments,
            &signature,
            argument_sources,
        );

        Ok(Answer::Ready(Some(ProtocolCall {
            resolution,
            return_type: signature.return_type,
        })))
    }

    /// Decide whether one protocol call returns a value assignable to a target.
    pub(in crate::check) fn protocol_call_returns(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        protocol: &Protocol,
        argument_types: &[dir::GlobalTypeId],
        argument_sources: &[dir::ArgumentSource],
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let call = answer!(self.select_protocol_call(
            origin,
            receiver,
            receiver,
            key,
            protocol,
            argument_types,
            argument_sources,
        )?);
        let Some(call) = call else {
            return Ok(Answer::Ready(false));
        };

        self.decide_relation(origin, Relation::Assignable, call.return_type, target)
    }

    /// Return whether one member candidate belongs to the protocol.
    fn candidate_satisfies_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        candidate: &MemberCandidate,
        protocol: &Protocol,
    ) -> CompilerResult<Answer<bool>> {
        if protocol.arguments.is_empty() {
            return self.member_owner_has_protocol(
                origin,
                module,
                receiver,
                candidate.owner,
                &candidate.generic_arguments,
                protocol.symbol,
            );
        }

        let interface = protocol.instance(self, module)?;

        self.member_owner_implements_interface(
            origin,
            module,
            module,
            receiver,
            candidate.owner,
            &candidate.generic_arguments,
            &interface,
        )
    }

    /// Decide whether an extension implementation covers one protocol request.
    fn extension_protocol_holds(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &TypeSubstitution,
        implements: &[dir::NominalHeritage],
        protocol: &Protocol,
    ) -> CompilerResult<Answer<bool>> {
        if protocol.arguments.is_empty() {
            return self.extension_has_protocol(
                origin,
                module,
                substitution,
                implements,
                protocol.symbol,
            );
        }

        let interface = protocol.instance(self, module)?;

        self.extension_implements_interface(
            origin,
            module,
            module,
            substitution,
            implements,
            &interface,
        )
    }

    /// Return the durable call resolution for one matched protocol method.
    fn protocol_call_resolution(
        &self,
        receiver: dir::GlobalTypeId,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
        generic_arguments: Vec<dir::GenericArgumentBinding>,
        signature: &SignatureSelection,
        argument_sources: &[dir::ArgumentSource],
    ) -> dir::CallResolution {
        let target = dir::CallTarget::Symbol(dir::CallCandidate {
            receiver: Some(receiver),
            adjustments: signature
                .receiver_steps
                .as_ref()
                .map(|steps| steps.to_vec())
                .unwrap_or_default(),
            generic_scope: Some(owner),
            symbol,
            generic_arguments,
        });

        dir::CallResolution::new(
            target,
            Some(signature.callable),
            Self::source_argument_bindings(argument_sources, &signature.parameters),
            signature.return_type,
        )
    }
}
