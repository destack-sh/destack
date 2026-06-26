use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, MemberCandidate, MemberLookup, Origin, Protocol, ProtocolCall,
    ProtocolMember, Relation, SignatureMatch, answer,
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
        let lookup = self.lookup_protocol_member(
            origin,
            module,
            lookup_receiver,
            dir::MemberSpace::Instance,
            key,
            protocol,
        )?;
        let candidates = match lookup {
            MemberLookup::Found(candidates) => candidates,
            MemberLookup::Field(_) | MemberLookup::Missing => return Ok(Answer::Ready(None)),
            MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // try candidates in declaration order
        for candidate in candidates {
            if !answer!(self.candidate_satisfies_protocol(
                origin,
                module,
                lookup_receiver,
                &candidate,
                protocol
            )?) {
                continue;
            }
            let Some(symbol) = candidate.symbol else {
                continue;
            };
            let target = dir::MemberTarget::Symbol(dir::MemberCandidate {
                receiver,
                owner: candidate.owner,
                symbol,
                ty: candidate.ty,
                generic_arguments: candidate.generic_arguments.clone(),
            });
            let resolution = dir::MemberResolution::new(receiver, target);

            return Ok(Answer::Ready(Some(ProtocolMember { resolution })));
        }

        Ok(Answer::Ready(None))
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
        let extensions = self.visible_receiver_extensions(module, lookup_receiver)?;

        // try visible extension candidates in declaration order
        for extension in extensions {
            let lookup = self.lookup_extension_symbol_member(
                origin,
                module,
                lookup_receiver,
                extension,
                dir::MemberSpace::Instance,
                key,
            )?;
            let candidates = match lookup {
                MemberLookup::Found(candidates) => candidates,
                MemberLookup::Field(_) | MemberLookup::Missing => continue,
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

            if let Some(call) = answer!(self.select_protocol_call_candidate(
                origin,
                module,
                receiver,
                lookup_receiver,
                protocol,
                argument_types,
                argument_sources,
                candidates,
            )?) {
                return Ok(Answer::Ready(Some(call)));
            }
        }

        let lookup = self.lookup_member(
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
            let Some(symbol) = candidate.symbol else {
                continue;
            };
            let accepted = self.attempt_callable(origin, candidate.ty, &[], argument_types)?;
            let Some(accepted) = answer!(accepted) else {
                continue;
            };

            let mut generic_arguments = candidate.generic_arguments.clone();
            generic_arguments.extend_from_slice(&accepted.generic_arguments);
            if !answer!(self.extension_clauses_hold(
                origin,
                Some(candidate.owner),
                &generic_arguments
            )?) {
                continue;
            }

            let resolution = self.protocol_call_resolution(
                receiver,
                symbol,
                generic_arguments,
                &accepted,
                argument_sources,
            );

            return Ok(Answer::Ready(Some(ProtocolCall {
                resolution,
                return_type: accepted.return_type,
            })));
        }

        Ok(Answer::Ready(None))
    }

    /// Look up members that can satisfy one protocol witness.
    fn lookup_protocol_member(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<MemberLookup> {
        let extension =
            self.lookup_receiver_extension_member(origin, module, lookup_receiver, space, key)?;
        let extension = match extension {
            MemberLookup::Found(candidates) => {
                let mut accepted = Vec::new();
                for candidate in candidates {
                    match self.candidate_satisfies_protocol(
                        origin,
                        module,
                        lookup_receiver,
                        &candidate,
                        protocol,
                    )? {
                        Answer::Ready(true) => accepted.push(candidate),
                        Answer::Ready(false) => {}
                        Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                    }
                }

                MemberLookup::from_candidates(accepted)
            }
            lookup => lookup,
        };

        if !matches!(extension, MemberLookup::Missing) {
            return Ok(extension);
        }

        self.lookup_member(origin, module, lookup_receiver, space, key)
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

    /// Return whether one selected member candidate belongs to the protocol.
    fn candidate_satisfies_protocol(
        &mut self,
        origin: Origin,
        module: destack_source::ModuleId,
        receiver: dir::GlobalTypeId,
        candidate: &MemberCandidate,
        protocol: &Protocol,
    ) -> CompilerResult<Answer<bool>> {
        if protocol.arguments.is_empty() {
            return self.selected_member_owner_has_protocol(
                origin,
                module,
                receiver,
                candidate.owner,
                &candidate.generic_arguments,
                protocol.symbol,
            );
        }

        let interface = protocol.instance();

        self.selected_member_owner_implements_interface(
            origin,
            module,
            receiver,
            candidate.owner,
            &candidate.generic_arguments,
            &interface,
        )
    }

    /// Return the durable call resolution for one accepted protocol method.
    fn protocol_call_resolution(
        &self,
        receiver: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        generic_arguments: Vec<dir::GenericArgumentBinding>,
        accepted: &SignatureMatch,
        argument_sources: &[dir::ArgumentSource],
    ) -> dir::CallResolution {
        let target = dir::CallTarget::Symbol(dir::CallCandidate {
            receiver: Some(receiver),
            symbol,
            generic_arguments,
        });

        dir::CallResolution::new(
            target,
            Some(accepted.callable),
            Self::parameter_types(&accepted.parameters),
            Self::generated_argument_bindings(argument_sources, &accepted.parameters),
            accepted.return_type,
        )
    }
}
