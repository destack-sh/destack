use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, DeclaredMember, Dependency, MemberCandidate, MemberLookup, Origin,
    Protocol, ProtocolCall, ProtocolMember, Relation, SelectedSignature, TypeSubstitution, answer,
};

impl CheckState<'_> {
    /// Return one protocol backed by a language item.
    pub(in crate::check) fn language_protocol(
        &self,
        item: dir::LanguageItem,
        arguments: Vec<dir::GlobalTypeId>,
    ) -> Protocol {
        Protocol::new(self.language_symbol(item), arguments)
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

        let lookup = self.lookup_inherent_member(
            origin,
            module,
            lookup_receiver,
            dir::MemberSpace::Instance,
            key,
        )?;
        let candidates = match lookup {
            MemberLookup::Found(candidates) => candidates,
            MemberLookup::Field(_) | MemberLookup::Missing => return Ok(Answer::Ready(None)),
            MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
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

        let lookup = self.lookup_inherent_member(
            origin,
            module,
            lookup_receiver,
            dir::MemberSpace::Instance,
            key,
        )?;
        let candidates = match lookup {
            MemberLookup::Found(candidates) => candidates,
            MemberLookup::Field(_) | MemberLookup::Missing => return Ok(Answer::Ready(None)),
            MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
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
        module: destack_source::ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        protocol: &Protocol,
        argument_types: &[dir::GlobalTypeId],
        argument_sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        let extensions = self.visible_receiver_extensions(module, lookup_receiver)?;
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // try each extension as one candidate transaction
        for extension_symbol in extensions {
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)
            else {
                continue;
            };
            if !extension.is_visible_from(module) || self.is_absent_symbol(extension_symbol) {
                continue;
            }

            let target_type = extension.target.r#type();
            let where_clauses = extension.where_clauses.clone();
            let implements = extension.implements.clone();
            let definition_members = extension.members.clone();
            let members = answer!(self.protocol_extension_members(&definition_members, key)?);
            if members.is_empty() {
                continue;
            }

            let template = self.symbol_template(extension_symbol);
            let probe = self.begin_probe();
            let matched = self.match_extension(
                origin,
                module,
                lookup_receiver,
                extension_symbol,
                template,
                target_type,
                &where_clauses,
                &members,
            )?;
            let matched = match matched {
                Answer::Ready(Some(matched)) => matched,
                Answer::Ready(None) => {
                    self.reject_probe(probe);

                    continue;
                }
                Answer::Pending(pending) => {
                    self.reject_probe(probe);
                    blockers.extend(self.live_blockers(pending));

                    continue;
                }
            };
            let protocol_holds = self.extension_protocol_holds(
                origin,
                module,
                &matched.substitution,
                &implements,
                protocol,
            )?;
            match protocol_holds {
                Answer::Ready(true) => {}
                Answer::Ready(false) => {
                    self.reject_probe(probe);

                    continue;
                }
                Answer::Pending(pending) => {
                    self.reject_probe(probe);
                    blockers.extend(self.live_blockers(pending));

                    continue;
                }
            }

            let call = self.select_protocol_call_member(
                origin,
                receiver,
                argument_types,
                argument_sources,
                matched.candidates,
            )?;
            match call {
                Answer::Ready(Some(call)) => {
                    self.commit_probe(probe);

                    return Ok(Answer::Ready(Some(call)));
                }
                Answer::Ready(None) => {
                    self.reject_probe(probe);
                }
                Answer::Pending(pending) => {
                    self.reject_probe(probe);
                    blockers.extend(self.live_blockers(pending));
                }
            }
        }

        Ok(Answer::ready_unless_blocked(None, blockers))
    }

    /// Select one protocol member from extension candidates.
    fn select_extension_protocol_member(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        let extensions = self.visible_receiver_extensions(module, lookup_receiver)?;
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // try each extension as one candidate transaction
        for extension_symbol in extensions {
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)
            else {
                continue;
            };
            if !extension.is_visible_from(module) || self.is_absent_symbol(extension_symbol) {
                continue;
            }

            let target_type = extension.target.r#type();
            let where_clauses = extension.where_clauses.clone();
            let implements = extension.implements.clone();
            let definition_members = extension.members.clone();
            let members = answer!(self.protocol_extension_members(&definition_members, key)?);
            if members.is_empty() {
                continue;
            }

            let template = self.symbol_template(extension_symbol);
            let probe = self.begin_probe();
            let matched = self.match_extension(
                origin,
                module,
                lookup_receiver,
                extension_symbol,
                template,
                target_type,
                &where_clauses,
                &members,
            )?;
            let matched = match matched {
                Answer::Ready(Some(matched)) => matched,
                Answer::Ready(None) => {
                    self.reject_probe(probe);

                    continue;
                }
                Answer::Pending(pending) => {
                    self.reject_probe(probe);
                    blockers.extend(self.live_blockers(pending));

                    continue;
                }
            };
            let protocol_holds = self.extension_protocol_holds(
                origin,
                module,
                &matched.substitution,
                &implements,
                protocol,
            )?;
            match protocol_holds {
                Answer::Ready(true) => {}
                Answer::Ready(false) => {
                    self.reject_probe(probe);

                    continue;
                }
                Answer::Pending(pending) => {
                    self.reject_probe(probe);
                    blockers.extend(self.live_blockers(pending));

                    continue;
                }
            }

            let member = self.select_protocol_member_from_candidates(receiver, matched.candidates);
            match member {
                Some(member) => {
                    self.commit_probe(probe);

                    return Ok(Answer::Ready(Some(member)));
                }
                None => {
                    self.reject_probe(probe);
                }
            }
        }

        Ok(Answer::ready_unless_blocked(None, blockers))
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
        module: destack_source::ModuleId,
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
            owner: candidate.owner,
            symbol,
            ty: candidate.ty,
            generic_arguments: generic_arguments.clone(),
        });
        let resolution = dir::MemberResolution::new(receiver, target);

        Some(ProtocolMember {
            resolution,
            ty: candidate.ty,
            symbol,
            generic_arguments,
        })
    }

    /// Select one protocol call from already ordered candidates.
    fn select_protocol_call_candidate(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
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
        let attempt = self.attempt_callable(origin, candidate.ty, &[], argument_types)?;
        let attempt = answer!(attempt);
        let Ok(signature) = attempt else {
            return Ok(Answer::Ready(None));
        };

        let mut generic_arguments = candidate.generic_arguments.clone();
        generic_arguments.extend_from_slice(&signature.generic_arguments);

        let resolution = self.protocol_call_resolution(
            receiver,
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
        module: destack_source::ModuleId,
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

        let interface = protocol.instance();

        self.member_owner_implements_interface(
            origin,
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
        module: destack_source::ModuleId,
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

        let interface = protocol.instance();

        self.extension_implements_interface(origin, module, substitution, implements, &interface)
    }

    /// Return the durable call resolution for one matched protocol method.
    fn protocol_call_resolution(
        &self,
        receiver: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        generic_arguments: Vec<dir::GenericArgumentBinding>,
        signature: &SelectedSignature,
        argument_sources: &[dir::ArgumentSource],
    ) -> dir::CallResolution {
        let target = dir::CallTarget::Symbol(dir::CallCandidate {
            receiver: Some(receiver),
            symbol,
            generic_arguments,
        });

        dir::CallResolution::new(
            target,
            Some(signature.callable),
            Self::parameter_types(&signature.parameters),
            Self::generated_argument_bindings(argument_sources, &signature.parameters),
            signature.return_type,
        )
    }
}
