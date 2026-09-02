use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseKind, CheckState, InterfaceMember, LookupReceiver, MemberCandidate, MemberLookup,
    Origin, Relation, RelationCheck, TypeArgumentInference, TypeSubstitution, Value, Verdict,
    member_arms,
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
    fn with_classification(mut self, classification: &[dir::GlobalTypeId]) -> Self {
        self.classification = classification.to_vec();

        self
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
        // bind the written arguments to the language protocol's template
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

impl CheckState<'_> {
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

        // infer the protocol arguments against the receiver
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
        // infer the protocol application, then select the member under it
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
        // select the candidates implementing the requirement along the receiver's dereferences
        let value = Value {
            ty: lookup_receiver,
            node: None,
            place: None,
            is_fresh: false,
        };
        let Some(candidates) =
            self.select_protocol_step_candidates(origin, value, space, key, protocol)?
        else {
            return Ok(None);
        };

        // read the first candidate per runtime arm, joining several as one union
        let arms = member_arms(&candidates);
        let mut accesses = Vec::with_capacity(arms.len());
        let mut types = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for (arm, group) in arms {
            let receiver = arm.map_or(receiver, |arm| arm.receiver);
            let Some(candidate) = group.first() else {
                return Ok(None);
            };
            let candidate = candidate.instantiate(origin, self)?;
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
    ) -> CompilerResult<Option<ProtocolCall>> {
        // select the candidates implementing the requirement along the receiver's dereferences
        let value = Value {
            ty: lookup_receiver,
            ..receiver
        };
        let Some(candidates) =
            self.select_protocol_step_candidates(origin, value, space, key, protocol)?
        else {
            return Ok(None);
        };

        // select the first accepting candidate per runtime arm, joining several as one union
        let arms = member_arms(&candidates);
        let mut calls = Vec::with_capacity(arms.len());
        let mut returns = SmallVec::<[dir::GlobalTypeId; 4]>::new();
        for (arm, group) in arms {
            let receiver = match arm {
                Some(arm) => Value {
                    ty: arm.receiver,
                    ..receiver
                },
                None => receiver,
            };
            let mut selected = None;
            for candidate in group {
                selected = self.select_protocol_call_candidate(
                    origin,
                    receiver,
                    argument_sources,
                    candidate,
                )?;
                if selected.is_some() {
                    break;
                }
            }
            let Some((call, return_type)) = selected else {
                return Ok(None);
            };
            returns.push(return_type);
            calls.push(call);
        }

        Ok(Some(match calls.as_slice() {
            [_] => ProtocolCall {
                resolution: dir::OperationResolution::One(calls.remove(0)),
                return_type: returns[0],
            },
            _ => {
                let return_type = self.normalized_union_type(returns)?;

                ProtocolCall {
                    resolution: dir::OperationResolution::Union {
                        arms: calls,
                        ty: return_type,
                    },
                    return_type,
                }
            }
        }))
    }

    /// Select the candidates the first dereference step exposing the requirement yields.
    fn select_protocol_step_candidates(
        &mut self,
        origin: Origin,
        receiver: Value,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        // take the first step whose candidates implement the requirement
        let module = origin.module();
        for step in self.builtin_steps(origin, receiver)? {
            // read a scalar constant's protocols on its apparent primitive
            let stepped = self.ownership_payload(origin, step.ty)?;
            let stepped = if self.is_literal_shape(stepped)? {
                self.widen_type(stepped)?
            } else {
                stepped
            };
            let Some(mut candidates) =
                self.select_protocol_candidates(origin, module, stepped, space, key, protocol)?
            else {
                continue;
            };
            // dispatch erased value receivers through their dynamic payload
            let constraint = match space {
                dir::MemberSpace::Instance => self.erased_constraint(stepped)?,
                _ => None,
            };
            for candidate in &mut candidates {
                candidate.receiver = match constraint {
                    Some(constraint) => LookupReceiver::Dynamic {
                        adjustments: step.adjustments.clone(),
                        constraint,
                    },
                    None => LookupReceiver::Direct(step.adjustments.clone()),
                };
            }

            return Ok(Some(candidates));
        }

        Ok(None)
    }

    /// Select the candidates implementing one protocol requirement on a receiver.
    fn select_protocol_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        protocol: &Protocol,
    ) -> CompilerResult<Option<MemberLookup>> {
        // require a requirement under the key
        let interface = protocol.classified().instance(self)?;
        let interface_type = self.intern_type(dir::Type::Application(interface))?;
        let requirements = self.interface_members(interface_type, receiver, space, key)?;
        if requirements.is_empty() {
            return Ok(None);
        }

        // decide the implementing extension
        let implementation = self.decide_extension_implementation(
            origin,
            Relation::Storable,
            module,
            receiver,
            &interface,
            None,
        )?;

        // read the winner's members under the key, interface defaults included
        let mut candidates = Vec::new();
        let mut winner = None;
        if let (Verdict::Holds, Some(extension)) = (implementation.verdict, implementation.winner)
            && let Some(source) =
                self.decide_extension_source(origin, module, receiver, receiver, extension)?
        {
            candidates = self
                .extension_candidates(origin, receiver, receiver, &source, space, Some(key))?
                .into_iter()
                .map(|(_, candidate)| candidate)
                .collect::<Vec<_>>();
            winner = Some(source.extension);

            // prove the conformance to bind the protocol's arguments
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.constrain_type(origin, cause, Relation::Subtype, receiver, interface_type)?;
        }

        // otherwise keep the inherent members the receiver's conformances select
        if candidates.is_empty() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            let verdict =
                self.constrain_type(origin, cause, Relation::Subtype, receiver, interface_type)?;
            if verdict == Verdict::Fails {
                return Ok(None);
            }
            candidates = self.conformance_candidates(
                origin,
                module,
                receiver,
                winner,
                interface.symbol,
                space,
                key,
                &requirements,
            )?;
            if candidates.is_empty() {
                return Ok(None);
            }
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            self.constrain_type(origin, cause, Relation::Storable, receiver, interface_type)?;
        }

        // bind the site's protocol application to its classified form
        self.bind_protocol_classification(origin, protocol)?;

        Ok(Some(candidates))
    }

    /// Keep the inherent members one receiver's conformances select for an interface requirement.
    fn conformance_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        extension: Option<dir::GlobalSymbolId>,
        interface: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
        requirements: &[InterfaceMember],
    ) -> CompilerResult<MemberLookup> {
        // look the key up on the receiver
        let lookup = self.lookup_visible_member(origin, module, receiver, space, key)?;

        // collect the members the receiver's, the extension's, and each owner's conformances select
        let mut conformers = FxIndexSet::default();
        if let Some(instance) = self.apparent_instance(receiver)? {
            self.collect_conformance_members(instance.symbol, interface, &mut conformers)?;
        }
        if let Some(extension) = extension {
            self.collect_conformance_members(extension, interface, &mut conformers)?;
        }
        for candidate in &lookup {
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
        for candidate in lookup {
            let Some(declared) = candidate.declaration() else {
                continue;
            };
            let source = self.symbol_source(declared.symbol)?;
            if conformers.contains(&declared.symbol) || sources.contains(&source) {
                kept.push(candidate);
            }
        }

        Ok(kept)
    }

    /// Bind the site's protocol application to its checked argument types.
    fn bind_protocol_classification(
        &mut self,
        origin: Origin,
        protocol: &Protocol,
    ) -> CompilerResult<()> {
        // keep a protocol the site left unclassified as asked
        if protocol.classification.is_empty() {
            return Ok(());
        }

        // equate the asked application with its classified form
        let instance = protocol.instance(self)?;
        let instance = self.intern_type(dir::Type::Application(instance))?;
        let classified = protocol.classified().instance(self)?;
        let classified = self.intern_type(dir::Type::Application(classified))?;
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        self.push_relation(RelationCheck::new(
            origin,
            Relation::Equal,
            classified,
            instance,
            cause,
        ))?;

        Ok(())
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
        let conformances = match self.definition(owner)? {
            Some(definition) => definition.implementations().to_vec(),
            None => Vec::new(),
        };
        for conformance in conformances {
            let Some((_, applied)) = self.nominal_application_maybe(conformance.interface)? else {
                continue;
            };
            if applied.symbol != interface {
                continue;
            }
            members.extend(conformance.members.iter().map(|member| member.member));
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
    ) -> CompilerResult<Option<(dir::Call, dir::GlobalTypeId)>> {
        // decide the candidate before constraining it
        let attempt = |state: &mut Self| {
            let candidate = candidate.instantiate(origin, state)?;
            let arguments = state.source_callable_arguments(origin, argument_sources)?;

            state.select_member_call(origin, receiver, &candidate, &arguments, argument_sources)
        };
        let selected = match self.decide(attempt)?.0 {
            Some(_) => attempt(self)?,
            None => None,
        };

        Ok(selected
            .map(|call| (call.return_type, call))
            .map(|(ty, call)| (call, ty)))
    }
}
