use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CandidateOutcome, CheckState, DeclaredMember, ExtensionMatch, InterfaceMember,
    MemberCandidate, MemberLookup, Origin, Relation, SignatureMatch, TypeArgumentInference,
    TypeSubstitution, Value,
};
use crate::{CompilerError, CompilerResult};

/// Interface protocol required by a generated operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct Protocol {
    /// The protocol interface symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The protocol generic arguments.
    pub(in crate::sema) arguments: Vec<dir::GlobalTypeId>,
    /// Checked argument types classifying candidates in probes only.
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
    pub(in crate::sema) fn instance(
        &self,
        check: &mut CheckState<'_>,
        _module: ModuleId,
    ) -> CompilerResult<dir::GenericApplication> {
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
        let arguments = check.intern_type_ids(&arguments)?;

        Ok(dir::GenericApplication {
            symbol: self.symbol,
            arguments,
        })
    }

    /// Return matching members from one selected protocol implementation.
    fn members(
        &self,
        check: &mut CheckState<'_>,
        implementation: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<SmallVec<[InterfaceMember; 2]>> {
        check.interface_members(implementation, receiver, space, key)
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
        self.resolution.map_types(map)?;
        self.return_type.map_types(map)
    }
}

impl CheckState<'_> {
    /// Return one complete protocol application backed by a language item.
    pub(in crate::sema) fn language_protocol(
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
    ) -> CompilerResult<Protocol> {
        let symbol = self.language_symbol(item)?;
        let Some(template) = self.symbol_template(symbol)? else {
            if written.is_empty() {
                return Ok(Protocol::new(symbol, Vec::new()));
            }

            // carry the written arguments of unloaded foreign templates
            if !self.is_loaded_module(symbol.module_id) {
                return Ok(Protocol::new(symbol, written.to_vec()));
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "nongeneric language protocol {symbol:?} received {} arguments",
                    written.len(),
                ),
            });
        };
        let parameters = self.generic_template_parameters(template)?;
        let Some(substitution) = self.instantiate_parameters(
            origin,
            &parameters,
            written,
            TypeSubstitution::default().with_receiver(receiver),
            TypeArgumentInference::Exact,
        )?
        else {
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
            self.check.push_relation(constraint)?;
        }
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
        let protocol = self.infer_language_protocol(origin, lookup_receiver, item, written)?;
        let protocol = protocol.with_classification(classification);
        let selected = self.select_protocol_call(
            origin,
            receiver,
            lookup_receiver,
            space,
            key,
            &protocol,
            argument_sources,
        )?;

        Ok(selected.map(|call| (protocol, call)))
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
        let protocol = self.infer_language_protocol(origin, lookup_receiver, item, written)?;
        let protocol = protocol.with_classification(classification);
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
        // read the lookup receiver as its apparent type
        let module = origin.module();
        let lookup_receiver = self.intern_apparent_type(lookup_receiver)?;

        let extension = self.select_extension_protocol_member(
            origin,
            module,
            receiver,
            lookup_receiver,
            space,
            key,
            protocol,
        )?;
        if !extension.is_none() {
            return Ok(extension);
        }

        // enumerate requirements for the fallback lookup only
        let interface = protocol.instance(self, module)?;
        let interface = self.intern_type(dir::Type::Application(interface))?;
        let requirements = protocol.members(self, interface, lookup_receiver, space, key)?;
        let lookup = self.lookup_inherent_member(origin, module, lookup_receiver, space, key)?;
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
    pub(in crate::sema) fn select_protocol_call(
        &mut self,
        origin: Origin,
        receiver: Value,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
        argument_sources: &[dir::ArgumentSource],
    ) -> CompilerResult<Option<ProtocolCall>> {
        let module = origin.module();
        let lookup_receiver = self.intern_apparent_type(lookup_receiver)?;
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
        if !extension.is_none() {
            return Ok(extension);
        }

        // enumerate requirements for the fallback lookup only
        let interface = protocol.instance(self, module)?;
        let interface = self.intern_type(dir::Type::Application(interface))?;
        let requirements = protocol.members(self, interface, lookup_receiver, space, key)?;
        let lookup = self.lookup_inherent_member(origin, module, lookup_receiver, space, key)?;
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
    ) -> CompilerResult<Option<ProtocolCall>> {
        self.select_extension_protocol_operation(
            origin,
            module,
            receiver.ty,
            lookup_receiver,
            space,
            key,
            protocol,
            |state, implementation, candidates| {
                let requirements =
                    protocol.members(state, implementation, lookup_receiver, space, key)?;

                // skip extensions whose implementation declares no matching requirement
                if requirements.is_empty() {
                    return Ok(None);
                }

                let call = state.select_protocol_call_member(
                    origin,
                    receiver,
                    argument_sources,
                    candidates,
                )?;

                Ok(call)
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
    ) -> CompilerResult<Option<ProtocolMember>> {
        self.select_extension_protocol_operation(
            origin,
            module,
            receiver,
            lookup_receiver,
            space,
            key,
            protocol,
            |state, implementation, candidates| {
                let requirements =
                    protocol.members(state, implementation, lookup_receiver, space, key)?;

                // skip extensions whose implementation declares no matching requirement
                if requirements.is_empty() {
                    return Ok(None);
                }

                let member = state.select_protocol_member_from_candidates(receiver, candidates)?;

                Ok(member)
            },
        )
    }

    /// Select one extension protocol candidate.
    fn select_extension_protocol_operation<T>(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
        mut select: impl FnMut(
            &mut BodyState<'_, '_>,
            dir::GlobalTypeId,
            Vec<MemberCandidate>,
        ) -> CompilerResult<Option<T>>,
    ) -> CompilerResult<Option<T>> {
        let extensions = self.visible_implementation_extensions(
            origin,
            module,
            lookup_receiver,
            protocol.symbol,
        )?;

        // concrete targets confirm before blankets, so declarations shadow them
        let mut ordered = Vec::with_capacity(extensions.len());
        for extension_symbol in extensions {
            let target = match self.definition(extension_symbol)? {
                Some(dir::Definition::Extension(extension)) => Some(extension.target.r#type()),
                _ => None,
            };
            let is_blanket = match target {
                Some(target) => matches!(self.ty(target)?, dir::Type::Parameter(_)),
                None => false,
            };
            ordered.push((extension_symbol, is_blanket));
        }
        ordered.sort_by_key(|(_, is_blanket)| *is_blanket);

        // confirm each candidate in one evaluation, committing the first accepted match
        for (extension_symbol, _) in ordered {
            if self.is_absent_symbol(extension_symbol) {
                continue;
            }
            let extension = match self.definition(extension_symbol)? {
                Some(dir::Definition::Extension(extension)) => extension,
                Some(_) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "indexed extension symbol {extension_symbol:?} has no extension definition"
                        ),
                    });
                }
                None => continue,
            };
            if !extension.is_visible_from(module) {
                continue;
            }

            let target_type = extension.target.r#type();
            let interfaces = extension
                .implements
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            let definition_members = extension.members.clone();
            let members = self.matching_extension_members(&definition_members, space, key)?;
            if members.is_empty() {
                continue;
            }

            let classified = protocol.classified();
            let selected = self.confirm_candidate(|state| {
                let matched = state.match_extension_protocol_members(
                    origin,
                    module,
                    receiver,
                    lookup_receiver,
                    extension_symbol,
                    target_type,
                    &interfaces,
                    &members,
                    &classified,
                )?;

                Ok(match matched {
                    Some(matched) => CandidateOutcome::Accepted(matched),
                    None => CandidateOutcome::Rejected(()),
                })
            })?;
            if let Some((implementation, candidates)) = selected {
                return select(self, implementation, candidates);
            }
        }

        Ok(None)
    }

    /// Match one extension implementation against a protocol member request.
    fn match_extension_protocol_members(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        target_type: dir::GlobalTypeId,
        interfaces: &[dir::GlobalTypeId],
        members: &[DeclaredMember],
        protocol: &Protocol,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, Vec<MemberCandidate>)>> {
        let matched = self.match_extension_protocol_implementation(
            origin,
            module,
            receiver,
            lookup_receiver,
            extension_symbol,
            target_type,
            interfaces,
            protocol,
        )?;
        let Some((substitution, implementation)) = matched else {
            return Ok(None);
        };
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "matched extension symbol {extension_symbol:?} has no extension definition"
                ),
            });
        };
        let definition_members = extension.members.clone();
        let Some(implementation) = self.instantiate_interface_implementation(
            origin,
            implementation,
            lookup_receiver,
            &definition_members,
            &substitution,
        )?
        else {
            return Ok(None);
        };

        // drop the key the protocol request already selected
        let candidates =
            self.extension_member_candidates(origin, extension_symbol, &substitution, members)?;
        let candidates = candidates
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(None);
        }

        Ok(Some((implementation, candidates)))
    }

    /// Match one extension target and implemented protocol.
    fn match_extension_protocol_implementation(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        target_type: dir::GlobalTypeId,
        interfaces: &[dir::GlobalTypeId],
        protocol: &Protocol,
    ) -> CompilerResult<Option<(TypeSubstitution, dir::GlobalTypeId)>> {
        let template = self.symbol_template(extension_symbol)?;
        let interface = protocol.instance(self, module)?;

        let matched = self.match_extension_implementation(
            origin,
            Relation::Assignable,
            module,
            receiver,
            lookup_receiver,
            &interface,
            template,
            target_type,
            interfaces,
        )?;

        // unproven bounds leave the protocol unselected at this ask
        Ok(match matched {
            ExtensionMatch::Matched(substitution, implementation) => {
                Some((*substitution, implementation))
            }
            ExtensionMatch::Unmatched | ExtensionMatch::Unproven => None,
        })
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
    ) -> CompilerResult<Option<ProtocolMember>> {
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
                    let Some(member) = self.select_protocol_member_lookup(
                        origin,
                        module,
                        arm.receiver,
                        arm.receiver,
                        space,
                        key,
                        protocol,
                        requirements,
                        arm.lookup,
                    )?
                    else {
                        return Ok(None);
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

                Ok(Some(ProtocolMember { resolution, ty }))
            }
            lookup @ MemberLookup::Intersection(_) => {
                let Some(candidates) = lookup.into_candidates() else {
                    return Ok(None);
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
            MemberLookup::Missing | MemberLookup::Field(_) | MemberLookup::Undecided => Ok(None),
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
    ) -> CompilerResult<Option<ProtocolCall>> {
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
                    let Some(call) = self.select_protocol_call_lookup(
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
                    )?
                    else {
                        return Ok(None);
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

                Ok(Some(ProtocolCall {
                    resolution,
                    return_type,
                }))
            }
            lookup @ MemberLookup::Intersection(_) => {
                let Some(candidates) = lookup.into_candidates() else {
                    return Ok(None);
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
            MemberLookup::Missing | MemberLookup::Field(_) | MemberLookup::Undecided => Ok(None),
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
    ) -> CompilerResult<Option<ProtocolMember>> {
        let candidates = self.select_nominal_protocol_candidates(
            origin,
            module,
            lookup_receiver,
            protocol,
            requirements,
            candidates,
        )?;
        let member = candidates
            .into_iter()
            .next()
            .map(|candidate| self.protocol_member_from_candidate(receiver, candidate));

        Ok(member)
    }

    /// Select the first protocol member from ordered candidates.
    fn select_protocol_member_from_candidates(
        &self,
        receiver: dir::GlobalTypeId,
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Option<ProtocolMember>> {
        let member = candidates
            .into_iter()
            .next()
            .map(|candidate| self.protocol_member_from_candidate(receiver, candidate));

        Ok(member)
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
            access_type: candidate.access_type,
            callable_type: candidate.callable,
            selection: dir::Selection::new(symbol, generic_arguments.clone()),
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
    ) -> CompilerResult<Option<ProtocolCall>> {
        let candidates = self.select_nominal_protocol_candidates(
            origin,
            module,
            lookup_receiver,
            protocol,
            requirements,
            candidates,
        )?;
        for candidate in candidates {
            let call = self.select_protocol_call_from_candidate(
                origin,
                receiver,
                argument_sources,
                candidate,
            )?;
            if !call.is_none() {
                return Ok(call);
            }
        }

        Ok(None)
    }

    /// Retain candidates declared by the interface or its proven implementers.
    fn select_nominal_protocol_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        protocol: &Protocol,
        requirements: &[InterfaceMember],
        candidates: Vec<MemberCandidate>,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        let declarations = requirements
            .iter()
            .map(|requirement| requirement.source)
            .collect::<FxIndexSet<_>>();
        let Some((_, receiver_instance)) = self.nominal_application_maybe(receiver)? else {
            let selected = self.select_protocol_declarations(&declarations, candidates)?;

            return Ok(selected);
        };
        let mut owners = FxIndexSet::default();
        owners.insert(receiver_instance.symbol);
        owners.extend(candidates.iter().map(|candidate| candidate.owner));
        let interface = protocol.instance(self.check, module)?;

        // match the requested interface against each owner's declared implementations
        let mut implementing_owners = FxIndexSet::default();
        for owner in owners {
            let Some(definition) = self.definition(owner)? else {
                return Err(CompilerError::Internal {
                    message: format!("protocol candidate owner {owner:?} has no definition"),
                });
            };
            if matches!(definition, dir::Definition::Interface(_)) {
                continue;
            }
            let interfaces = definition
                .implementations()
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            if interfaces.is_empty() {
                continue;
            }

            // apply the receiver's concrete arguments to this declaration
            let application_type = if owner == receiver_instance.symbol {
                Some(receiver)
            } else {
                self.heritage_instance(origin, receiver, receiver, owner)?
            };
            let Some(application_type) = application_type else {
                continue;
            };

            // match the declared implementations at this application
            let (application_module, application) = self.nominal_application(application_type)?;
            let mut substitution = self.instance_substitution(application_module, &application)?;
            let matched = self.match_implemented_interface(
                origin,
                Relation::Assignable,
                module,
                &[],
                &mut substitution,
                &interfaces,
                &interface,
            )?;
            if matched.is_some() {
                implementing_owners.insert(owner);
            }
        }

        // retain interface declarations and members of proven implementers
        let mut selected = Vec::new();
        for candidate in candidates {
            let source = self.symbol_source(candidate.symbol)?;
            if implementing_owners.contains(&candidate.owner) || declarations.contains(&source) {
                selected.push(candidate);
            }
        }

        Ok(selected)
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
    ) -> CompilerResult<Option<ProtocolCall>> {
        for candidate in candidates {
            let call = self.select_protocol_call_from_candidate(
                origin,
                receiver,
                argument_sources,
                candidate,
            )?;
            if !call.is_none() {
                return Ok(call);
            }
        }

        Ok(None)
    }

    /// Select one already proved protocol call candidate.
    fn select_protocol_call_from_candidate(
        &mut self,
        origin: Origin,
        receiver: Value,
        argument_sources: &[dir::ArgumentSource],
        candidate: MemberCandidate,
    ) -> CompilerResult<Option<ProtocolCall>> {
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
        let signature = match attempt {
            SignatureMatch::Selected(signature) => signature,
            SignatureMatch::Invalid { .. }
            | SignatureMatch::ReturnMismatch(_)
            | SignatureMatch::Inapplicable(_) => {
                return Ok(None);
            }
        };

        let arguments = self.bind_argument_sources(origin, &signature, argument_sources)?;
        let call = signature.member_call(resolution, candidate.owner, symbol, arguments);
        let resolution = dir::OperationResolution::One(call);

        Ok(Some(ProtocolCall {
            resolution,
            return_type: signature.return_type,
        }))
    }
}
