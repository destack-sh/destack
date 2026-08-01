use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CandidateOutcome, CandidateVerdict, CheckState, DeclaredMember, Dependency,
    InterfaceMember, MemberCandidate, MemberLookup, Origin, Relation, SignatureMatch,
    TypeArgumentInference, TypeSubstitution, Value, answer,
};
use crate::{CompilerError, CompilerResult};

/// Interface protocol required by a generated operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct Protocol {
    /// The protocol interface symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The protocol generic arguments.
    pub(in crate::check) arguments: Vec<dir::GlobalTypeId>,
    /// Checked argument types classifying candidates in probes only.
    pub(in crate::check) classification: Vec<dir::GlobalTypeId>,
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
    fn with_classification(mut self, classification: &[dir::GlobalTypeId]) -> Self {
        self.classification = classification.to_vec();

        self
    }

    /// Return the probe-facing protocol with classified leading arguments.
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
    pub(in crate::check) fn instance(
        &self,
        check: &mut CheckState<'_>,
        _module: ModuleId,
    ) -> CompilerResult<dir::GenericApplication> {
        match check.symbol_template(self.symbol)? {
            Some(template) => {
                check.template_substitution(template, &self.arguments)?;
            }
            None if self.arguments.is_empty() => {}
            // keep unloaded foreign templates symbolic
            None if !check.is_loaded_module(self.symbol.module_id) => {}
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
        let arguments = check.intern_type_ids(&self.arguments)?;

        Ok(dir::GenericApplication {
            symbol: self.symbol,
            arguments,
        })
    }

    /// Return matching members from one selected protocol implementation.
    fn members(
        &self,
        check: &mut CheckState<'_>,
        origin: Origin,
        implementation: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<SmallVec<[InterfaceMember; 2]>>> {
        check.interface_members(origin, implementation, receiver, space, key)
    }
}

/// Protocol member accepted for a generated operation.
pub(in crate::check) struct ProtocolMember {
    /// The member resolution.
    pub(in crate::check) resolution: dir::MemberResolution,
    /// The member type.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// Protocol call accepted for a generated operation.
pub(in crate::check) struct ProtocolCall {
    /// The call resolution.
    pub(in crate::check) resolution: dir::CallResolution,
    /// The call return type.
    pub(in crate::check) return_type: dir::GlobalTypeId,
}

impl CheckState<'_> {
    /// Return one complete protocol application backed by a language item.
    pub(in crate::check) fn language_protocol(
        &mut self,
        module: ModuleId,
        item: dir::LanguageItem,
        written: Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<Protocol> {
        let symbol = self.language_symbol(item)?;
        let arguments = match self.symbol_template(symbol)? {
            Some(template) => {
                let Some(substitution) =
                    self.bind_explicit_arguments(module, template, &written)?
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
            // carry the written arguments of unloaded foreign templates
            None if !self.is_loaded_module(symbol.module_id) => written,
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
}

// union arms recurse per receiver while the key stays fixed
#[allow(clippy::only_used_in_recursion)]
impl BodyState<'_, '_> {
    /// Infer one complete protocol application backed by a language item.
    fn infer_language_protocol(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        item: dir::LanguageItem,
        written: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Protocol>> {
        let symbol = self.language_symbol(item)?;
        let Some(template) = self.symbol_template(symbol)? else {
            if written.is_empty() {
                return Ok(Answer::Ready(Protocol::new(symbol, Vec::new())));
            }

            // carry the written arguments of unloaded foreign templates
            if !self.is_loaded_module(symbol.module_id) {
                return Ok(Answer::Ready(Protocol::new(symbol, written.to_vec())));
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "nongeneric language protocol {symbol:?} received {} arguments",
                    written.len(),
                ),
            });
        };
        let parameters = self.generic_template_parameters(template)?;
        let Some(substitution) = answer!(self.instantiate_parameters(
            origin,
            &parameters,
            written,
            TypeSubstitution::default().with_receiver(receiver),
            TypeArgumentInference::Exact,
        )?) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "language protocol {symbol:?} cannot infer from {} arguments",
                    written.len(),
                ),
            });
        };

        // retain the protocol declaration's parameter bounds
        for constraint in
            self.substitute_application_constraints(origin, template, &substitution)?
        {
            self.check.push_constraint(constraint);
        }
        let arguments = substitution.arguments().collect();

        Ok(Answer::Ready(Protocol::new(symbol, arguments)))
    }

    /// Select one call while inferring its complete language protocol application.
    pub(in crate::check) fn select_language_protocol_call(
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
    ) -> CompilerResult<Answer<Option<(Protocol, ProtocolCall)>>> {
        let protocol =
            answer!(self.infer_language_protocol(origin, lookup_receiver, item, written,)?);
        let protocol = protocol.with_classification(classification);
        let selected = answer!(self.select_protocol_call(
            origin,
            receiver,
            lookup_receiver,
            space,
            key,
            &protocol,
            argument_sources,
        )?);

        Ok(Answer::Ready(selected.map(|call| (protocol, call))))
    }

    /// Select one member while inferring its complete language protocol application.
    pub(in crate::check) fn select_language_protocol_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        item: dir::LanguageItem,
        written: &[dir::GlobalTypeId],
        classification: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<Option<(Protocol, ProtocolMember)>>> {
        let protocol =
            answer!(self.infer_language_protocol(origin, lookup_receiver, item, written,)?);
        let protocol = protocol.with_classification(classification);
        let selected = answer!(self.select_protocol_member(
            origin,
            receiver,
            lookup_receiver,
            space,
            key,
            &protocol,
        )?);

        Ok(Answer::Ready(selected.map(|member| (protocol, member))))
    }

    /// Select one protocol member by key.
    pub(in crate::check) fn select_protocol_member(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        let module = origin.module();
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let lookup_receiver = answer!(self.reduce_type_head(origin, lookup_receiver)?);

        let lookup_receiver = self.intern_apparent_type(module, lookup_receiver)?;
        let interface = protocol.instance(self, module)?;
        let interface = self.intern_type(dir::Type::Application(interface))?;
        let requirements =
            answer!(protocol.members(self, origin, interface, lookup_receiver, space, key)?);
        let extension = self.select_extension_protocol_member(
            origin,
            module,
            receiver,
            lookup_receiver,
            space,
            key,
            protocol,
        )?;
        if !matches!(extension, Answer::Ready(None)) {
            return Ok(extension);
        }

        let lookup =
            answer!(self.lookup_inherent_member(origin, module, lookup_receiver, space, key,)?);
        self.select_protocol_member_lookup(
            origin,
            module,
            receiver,
            lookup_receiver,
            space,
            key,
            protocol,
            &requirements,
            lookup,
        )
    }

    /// Select one protocol method call by key.
    pub(in crate::check) fn select_protocol_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
        argument_sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        let module = origin.module();
        let receiver_type = answer!(self.reduce_type_head(origin, receiver.ty)?);
        let receiver = Value {
            ty: receiver_type,
            ..receiver
        };
        let lookup_receiver = answer!(self.reduce_type_head(origin, lookup_receiver)?);
        let lookup_receiver = self.intern_apparent_type(module, lookup_receiver)?;
        let interface = protocol.instance(self, module)?;
        let interface = self.intern_type(dir::Type::Application(interface))?;
        let requirements =
            answer!(protocol.members(self, origin, interface, lookup_receiver, space, key)?);
        let extension = self.select_extension_protocol_call(
            origin,
            module,
            receiver,
            lookup_receiver,
            space,
            key,
            protocol,
            argument_sources,
        )?;
        if !matches!(extension, Answer::Ready(None)) {
            return Ok(extension);
        }

        let lookup =
            answer!(self.lookup_inherent_member(origin, module, lookup_receiver, space, key,)?);
        self.select_protocol_call_lookup(
            origin,
            module,
            receiver,
            lookup_receiver,
            space,
            key,
            protocol,
            &requirements,
            argument_sources,
            lookup,
        )
    }

    /// Select one protocol call from extension candidates.
    fn select_extension_protocol_call(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
        argument_sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        self.select_extension_protocol_operation(
            origin,
            module,
            lookup_receiver,
            space,
            key,
            protocol,
            |state, implementation, selected, candidates| {
                let requirements = answer!(protocol.members(
                    state,
                    origin,
                    implementation,
                    lookup_receiver,
                    space,
                    key,
                )?);
                let candidates =
                    state.select_implementation_candidates(&selected, &requirements, candidates)?;
                match state.select_protocol_call_member(
                    origin,
                    receiver,
                    argument_sources,
                    candidates,
                )? {
                    Answer::Ready(call) => Ok(Answer::Ready(call)),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
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
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        self.select_extension_protocol_operation(
            origin,
            module,
            lookup_receiver,
            space,
            key,
            protocol,
            |state, implementation, selected, candidates| {
                let requirements = answer!(protocol.members(
                    state,
                    origin,
                    implementation,
                    lookup_receiver,
                    space,
                    key,
                )?);
                let candidates =
                    state.select_implementation_candidates(&selected, &requirements, candidates)?;
                match state.select_protocol_member_from_candidates(receiver, candidates)? {
                    Answer::Ready(member) => Ok(Answer::Ready(member)),
                    Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
                }
            },
        )
    }

    /// Select one extension protocol candidate.
    fn select_extension_protocol_operation<T>(
        &mut self,
        origin: Origin,
        module: ModuleId,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
        mut select: impl FnMut(
            &mut BodyState<'_, '_>,
            dir::GlobalTypeId,
            dir::InterfaceImplementation,
            Vec<MemberCandidate>,
        ) -> CompilerResult<Answer<Option<T>>>,
    ) -> CompilerResult<Answer<Option<T>>> {
        let extensions =
            answer!(self.visible_implementation_extensions(origin, module, lookup_receiver,)?);
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let mut viable = None;
        let mut indeterminate = None;

        // classify each extension without retaining speculative state
        for extension_symbol in extensions {
            if self.is_absent_symbol(extension_symbol) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)?
            else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "indexed protocol implementation {extension_symbol:?} has no extension definition"
                    ),
                });
            };
            if !extension.is_visible_from(module) {
                continue;
            }

            let target_type = extension.target.r#type();
            let implements = extension.implements.clone();
            let definition_members = extension.members.clone();
            let members =
                answer!(self.protocol_extension_members(&definition_members, space, key)?);
            if members.is_empty() {
                continue;
            }

            let classified = protocol.classified();
            let verdict = self.probe_candidate(|state| {
                state.match_extension_protocol_candidate(
                    origin,
                    module,
                    lookup_receiver,
                    extension_symbol,
                    target_type,
                    &implements,
                    &classified,
                )
            })?;
            let candidate = (
                extension_symbol,
                target_type,
                implements,
                definition_members,
                members,
            );
            match verdict {
                Answer::Ready(CandidateVerdict::Viable) => {
                    viable = Some(candidate);

                    break;
                }
                Answer::Ready(CandidateVerdict::Indeterminate) => {
                    indeterminate.get_or_insert(candidate);
                }
                Answer::Ready(CandidateVerdict::Rejected) => {}
                // keep a candidate whose outer variables are still open
                Answer::Pending(pending) => {
                    indeterminate.get_or_insert(candidate);
                    blockers.extend(pending);
                }
            }
        }

        // apply one selected extension in the enclosing transaction
        let Some((extension_symbol, target_type, implements, definition_members, members)) =
            viable.or(indeterminate)
        else {
            return Ok(Answer::ready_unless_blocked(None, blockers));
        };
        let matched = self.match_extension_protocol_members(
            origin,
            module,
            lookup_receiver,
            extension_symbol,
            target_type,
            &implements,
            &definition_members,
            &members,
            protocol,
        )?;
        let (implementation, selected, candidates) = match matched {
            Answer::Ready(Some(matched)) => matched,
            Answer::Ready(None) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "selected extension protocol candidate {extension_symbol:?} was rejected"
                    ),
                });
            }
            Answer::Pending(pending) => return Ok(Answer::Pending(pending)),
        };

        select(self, implementation, selected, candidates)
    }

    /// Classify one extension protocol candidate.
    fn match_extension_protocol_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        lookup_receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        target_type: dir::GlobalTypeId,
        implementations: &[dir::InterfaceImplementation],
        protocol: &Protocol,
    ) -> CompilerResult<Answer<CandidateOutcome<(), ()>>> {
        let matched = self.match_extension_protocol_implementation(
            origin,
            module,
            lookup_receiver,
            extension_symbol,
            target_type,
            implementations,
            protocol,
        )?;

        Ok(matched.map(|matched| match matched {
            Some(_) => CandidateOutcome::Accepted(()),
            None => CandidateOutcome::Rejected(()),
        }))
    }

    /// Match one extension implementation against a protocol member request.
    fn match_extension_protocol_members(
        &mut self,
        origin: Origin,
        module: ModuleId,
        lookup_receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        target_type: dir::GlobalTypeId,
        implementations: &[dir::InterfaceImplementation],
        definition_members: &[dir::DefinitionMember],
        members: &[DeclaredMember],
        protocol: &Protocol,
    ) -> CompilerResult<
        Answer<
            Option<(
                dir::GlobalTypeId,
                dir::InterfaceImplementation,
                Vec<MemberCandidate>,
            )>,
        >,
    > {
        let matched = self.match_extension_protocol_implementation(
            origin,
            module,
            lookup_receiver,
            extension_symbol,
            target_type,
            implementations,
            protocol,
        )?;
        let Some((substitution, implementation, index)) = answer!(matched) else {
            return Ok(Answer::Ready(None));
        };
        let candidates =
            self.extension_member_candidates(origin, extension_symbol, &substitution, members)?;
        let candidates = answer!(candidates);
        if candidates.is_empty() {
            return Ok(Answer::Ready(None));
        }
        let declared = implementations
            .get(index)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "selected interface implementation {index} is absent from {extension_symbol:?}"
                ),
            })?;
        let (interface_module, interface) = self.require_nominal_application(implementation)?;
        let selected = answer!(self.check.select_interface_implementation(
            origin,
            extension_symbol,
            declared.interface.source,
            implementation,
            lookup_receiver,
            definition_members,
            &substitution,
            interface_module,
            &interface,
        )?);
        let Ok(selected) = selected else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some((implementation, selected, candidates))))
    }

    /// Match one extension target and implemented protocol.
    fn match_extension_protocol_implementation(
        &mut self,
        origin: Origin,
        module: ModuleId,
        lookup_receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        target_type: dir::GlobalTypeId,
        implementations: &[dir::InterfaceImplementation],
        protocol: &Protocol,
    ) -> CompilerResult<Answer<Option<(TypeSubstitution, dir::GlobalTypeId, usize)>>> {
        let template = self.symbol_template(extension_symbol)?;
        let Some(mut substitution) =
            answer!(self.instantiate_extension(origin, lookup_receiver, template, target_type)?)
        else {
            return Ok(Answer::Ready(None));
        };

        // match the declared implementation to the requested protocol
        let interface = protocol.instance(self, module)?;
        let implementation = answer!(self.match_implemented_interface(
            origin,
            Relation::Assignable,
            module,
            module,
            &[],
            &mut substitution,
            implementations,
            &interface,
        )?);

        Ok(Answer::Ready(implementation.map(
            |(implementation, index)| (substitution, implementation, index),
        )))
    }

    /// Return declared extension members matching one protocol key.
    fn protocol_extension_members(
        &mut self,
        members: &[dir::DefinitionMember],
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Vec<DeclaredMember>>> {
        let mut matched = Vec::new();
        for member in members {
            let member = match self.declared_member(member)? {
                Answer::Ready(Some(member)) => member,
                Answer::Ready(None) => continue,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            if member.matches(space, key) {
                matched.push(member);
            }
        }

        Ok(Answer::Ready(matched))
    }

    /// Retain candidates selected by one checked interface implementation.
    fn select_implementation_candidates(
        &self,
        implementation: &dir::InterfaceImplementation,
        requirements: &[InterfaceMember],
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        let mut declarations = FxIndexSet::default();

        // accept the interface declaration itself for generic and default dispatch
        for requirement in requirements {
            declarations.insert(requirement.source);
            if let Some(member) = implementation.member(requirement.source) {
                declarations.extend(member.declarations.iter().copied());
            }
        }

        // retain only declarations fixed by the implementation record
        let mut selected = Vec::new();
        for candidate in candidates {
            let source = self.symbol_source(candidate.symbol)?;
            if declarations.contains(&source) {
                selected.push(candidate);
            }
        }

        Ok(selected)
    }

    /// Select one protocol member from every receiver alternative.
    fn select_protocol_member_lookup(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
        requirements: &[InterfaceMember],
        lookup: MemberLookup,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        match lookup {
            MemberLookup::Found(candidates) => self.select_protocol_member_candidate(
                origin,
                module,
                receiver,
                lookup_receiver,
                protocol,
                requirements,
                candidates,
            ),
            MemberLookup::Union(lookups) => {
                let mut arms = Vec::with_capacity(lookups.len());
                let mut types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                for arm in lookups {
                    let Some(member) = answer!(self.select_protocol_member_lookup(
                        origin,
                        module,
                        arm.receiver,
                        arm.receiver,
                        space,
                        key,
                        protocol,
                        requirements,
                        arm.lookup,
                    )?) else {
                        return Ok(Answer::Ready(None));
                    };
                    let dir::OperationResolution::One(access) = member.resolution else {
                        return Err(CompilerError::Internal {
                            message: "union protocol lookup contains a nested union".to_string(),
                        });
                    };
                    arms.push(access);
                    types.push(member.ty);
                }
                let ty = match types.as_slice() {
                    [single] => *single,
                    _ => self.normalized_union_type(types)?,
                };
                let resolution = dir::OperationResolution::Union { arms, ty };

                Ok(Answer::Ready(Some(ProtocolMember { resolution, ty })))
            }
            lookup @ MemberLookup::Intersection(_) => {
                let Some(candidates) = lookup.into_candidates() else {
                    return Ok(Answer::Ready(None));
                };

                self.select_protocol_member_candidate(
                    origin,
                    module,
                    receiver,
                    lookup_receiver,
                    protocol,
                    requirements,
                    candidates,
                )
            }
            MemberLookup::Missing | MemberLookup::Field(_) => Ok(Answer::Ready(None)),
        }
    }

    /// Select one protocol call from every receiver alternative.
    fn select_protocol_call_lookup(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
        requirements: &[InterfaceMember],
        argument_sources: &[dir::ArgumentSource],
        lookup: MemberLookup,
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        match lookup {
            MemberLookup::Found(candidates) => self.select_protocol_call_candidate(
                origin,
                module,
                receiver,
                lookup_receiver,
                protocol,
                requirements,
                argument_sources,
                candidates,
            ),
            MemberLookup::Union(lookups) => {
                let mut calls = Vec::with_capacity(lookups.len());
                let mut returns = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                for arm in lookups {
                    let Some(call) = answer!(self.select_protocol_call_lookup(
                        origin,
                        module,
                        Value {
                            ty: arm.receiver,
                            ..receiver
                        },
                        arm.receiver,
                        space,
                        key,
                        protocol,
                        requirements,
                        argument_sources,
                        arm.lookup,
                    )?) else {
                        return Ok(Answer::Ready(None));
                    };
                    let dir::OperationResolution::One(call) = call.resolution else {
                        return Err(CompilerError::Internal {
                            message: "union protocol call contains a nested union".to_string(),
                        });
                    };
                    returns.push(call.return_type);
                    calls.push(call);
                }
                let return_type = match returns.as_slice() {
                    [single] => *single,
                    _ => self.normalized_union_type(returns)?,
                };
                let resolution = dir::OperationResolution::Union {
                    arms: calls,
                    ty: return_type,
                };

                Ok(Answer::Ready(Some(ProtocolCall {
                    resolution,
                    return_type,
                })))
            }
            lookup @ MemberLookup::Intersection(_) => {
                let Some(candidates) = lookup.into_candidates() else {
                    return Ok(Answer::Ready(None));
                };

                self.select_protocol_call_candidate(
                    origin,
                    module,
                    receiver,
                    lookup_receiver,
                    protocol,
                    requirements,
                    argument_sources,
                    candidates,
                )
            }
            MemberLookup::Missing | MemberLookup::Field(_) => Ok(Answer::Ready(None)),
        }
    }

    /// Select one protocol member from already ordered candidates.
    fn select_protocol_member_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        protocol: &Protocol,
        requirements: &[InterfaceMember],
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        let candidates = answer!(self.select_nominal_protocol_candidates(
            origin,
            module,
            lookup_receiver,
            protocol,
            requirements,
            candidates,
        )?);
        let member = candidates
            .into_iter()
            .next()
            .map(|candidate| self.protocol_member_from_candidate(receiver, candidate));

        Ok(Answer::Ready(member))
    }

    /// Select the first protocol member from ordered candidates.
    fn select_protocol_member_from_candidates(
        &self,
        receiver: dir::GlobalTypeId,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<Option<ProtocolMember>>> {
        let member = candidates
            .into_iter()
            .next()
            .map(|candidate| self.protocol_member_from_candidate(receiver, candidate));

        Ok(Answer::Ready(member))
    }

    /// Return one durable protocol member resolution.
    fn protocol_member_from_candidate(
        &self,
        receiver: dir::GlobalTypeId,
        candidate: MemberCandidate,
    ) -> ProtocolMember {
        let symbol = candidate.symbol;
        let generic_arguments = candidate.generic_arguments;
        let target = dir::MemberTarget::Symbol(dir::MemberCandidate {
            receiver: candidate.receiver.resolve(receiver),
            space: candidate.space,
            owner: candidate.owner,
            symbol,
            access_type: candidate.access_type,
            callable_type: candidate.callable,
            generic_arguments: generic_arguments.clone(),
        });
        let access = dir::MemberAccess::new(receiver, target, candidate.access_type);
        let resolution = dir::OperationResolution::One(access);

        ProtocolMember {
            resolution,
            ty: candidate.access_type,
        }
    }

    /// Select one protocol call from already ordered candidates.
    fn select_protocol_call_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        protocol: &Protocol,
        requirements: &[InterfaceMember],
        argument_sources: &[dir::ArgumentSource],
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        let candidates = answer!(self.select_nominal_protocol_candidates(
            origin,
            module,
            lookup_receiver,
            protocol,
            requirements,
            candidates,
        )?);
        for candidate in candidates {
            let call = self.select_protocol_call_from_candidate(
                origin,
                receiver,
                argument_sources,
                candidate,
            )?;
            if !matches!(call, Answer::Ready(None)) {
                return Ok(call);
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Retain candidates selected by the receiver's checked implementations.
    fn select_nominal_protocol_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        protocol: &Protocol,
        requirements: &[InterfaceMember],
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<Vec<MemberCandidate>>> {
        let mut declarations = requirements
            .iter()
            .map(|requirement| requirement.source)
            .collect::<FxIndexSet<_>>();
        let Some((receiver_module, receiver_instance)) =
            self.nominal_application_maybe(receiver)?
        else {
            let selected = self.select_protocol_declarations(&declarations, candidates)?;

            return Ok(Answer::Ready(selected));
        };
        let mut owners = FxIndexSet::default();
        owners.insert(receiver_instance.symbol);
        owners.extend(candidates.iter().map(|candidate| candidate.owner));
        let interface = protocol.instance(self.check, module)?;

        // collect exact declarations from each relevant nominal implementation
        for owner in owners {
            let Some(definition) = self.definition(owner)? else {
                return Err(CompilerError::Internal {
                    message: format!("protocol candidate owner {owner:?} has no definition"),
                });
            };
            if matches!(definition, dir::Definition::Interface(_)) {
                continue;
            }
            let definition_members = definition.members().to_vec();
            let implementations = definition.implementations().to_vec();
            if implementations.is_empty() {
                continue;
            }

            // apply the receiver's concrete arguments to this declaration
            let application_type = if owner == receiver_instance.symbol {
                Some(receiver)
            } else {
                answer!(self.heritage_instance(
                    origin,
                    receiver_module,
                    &receiver_instance,
                    owner,
                )?)
            };
            let Some(application_type) = application_type else {
                continue;
            };
            let (application_module, application) =
                self.require_nominal_application(application_type)?;
            let mut substitution = self.instance_substitution(application_module, &application)?;
            let matched = answer!(self.match_implemented_interface(
                origin,
                Relation::Assignable,
                module,
                module,
                &[],
                &mut substitution,
                &implementations,
                &interface,
            )?);
            let Some((implemented, index)) = matched else {
                continue;
            };
            let declared = implementations
                .get(index)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!(
                        "selected interface implementation {index} is absent from {owner:?}"
                    ),
                })?;
            let (interface_module, interface) = self.require_nominal_application(implemented)?;
            let selected = answer!(self.check.select_interface_implementation(
                origin,
                owner,
                declared.interface.source,
                implemented,
                application_type,
                &definition_members,
                &substitution,
                interface_module,
                &interface,
            )?);
            let Ok(implementation) = selected else {
                continue;
            };
            for requirement in requirements {
                if let Some(member) = implementation.member(requirement.source) {
                    declarations.extend(member.declarations.iter().copied());
                }
            }
        }
        let selected = self.select_protocol_declarations(&declarations, candidates)?;

        Ok(Answer::Ready(selected))
    }

    /// Retain candidates whose declarations are selected by a protocol implementation.
    fn select_protocol_declarations(
        &self,
        declarations: &FxIndexSet<dir::GlobalNodeIdAny>,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        let mut selected = Vec::new();
        for candidate in candidates {
            let source = self.symbol_source(candidate.symbol)?;
            if declarations.contains(&source) {
                selected.push(candidate);
            }
        }

        Ok(selected)
    }

    /// Select one protocol call after the owner has proved the protocol.
    fn select_protocol_call_member(
        &mut self,
        origin: Origin,
        receiver: Value,
        argument_sources: &[dir::ArgumentSource],
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        for candidate in candidates {
            let call = self.select_protocol_call_from_candidate(
                origin,
                receiver,
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
        receiver: Value,
        argument_sources: &[dir::ArgumentSource],
        candidate: MemberCandidate,
    ) -> CompilerResult<Answer<Option<ProtocolCall>>> {
        let symbol = candidate.symbol;
        let resolution = candidate.receiver.resolve(receiver.ty);
        let selection_type = match &resolution {
            dir::MemberReceiver::Direct(receiver) => receiver.ty(),
            dir::MemberReceiver::Dynamic(dispatch) => dispatch.constraint,
        };
        let selection_receiver = Value {
            ty: selection_type,
            ..receiver
        };
        let arguments = self.source_callable_arguments(origin, argument_sources)?;
        let attempt = self.attempt_callable(
            origin,
            candidate.callable.ok_or_else(|| CompilerError::Internal {
                message: format!("protocol member {symbol:?} has no callable type"),
            })?,
            Some(candidate.owner),
            Some(selection_receiver),
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

        let arguments = Self::source_argument_bindings(argument_sources, &signature.parameters);
        let call = signature.member_call(resolution, candidate.owner, symbol, arguments);
        let resolution = dir::OperationResolution::One(call);

        Ok(Answer::Ready(Some(ProtocolCall {
            resolution,
            return_type: signature.return_type,
        })))
    }
}
