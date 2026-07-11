use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, Cause, CauseKind, CheckState, MemberRole, Origin, Relation, TypeSubstitution, answer,
};

use super::nominal::HeritageApplication;

/// Requirements imposed by one applied interface.
#[derive(Debug, Clone)]
pub(in crate::check) struct InterfaceRequirements {
    /// The members declared directly by the interface.
    pub(in crate::check) members: SmallVec<[InterfaceMember; 8]>,
    /// The directly inherited interface applications.
    pub(in crate::check) inherited: SmallVec<[HeritageApplication; 8]>,
}

/// One member required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct InterfaceMember {
    /// The member space.
    pub(in crate::check) space: dir::MemberSpace,
    /// The member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The member type, when the member carries a value.
    pub(in crate::check) ty: Option<dir::GlobalTypeId>,
    /// How the member participates in assignability.
    pub(in crate::check) role: MemberRole,
    /// Whether the member carries a default implementation.
    pub(in crate::check) has_default: bool,
    /// Whether the member is optional on its declaration.
    pub(in crate::check) is_optional: bool,
    /// Whether the member rejects writes after initialization.
    pub(in crate::check) is_readonly: bool,
}

impl CheckState<'_> {
    /// Decide whether one source exposes every member of one interface application.
    pub(in crate::check) fn decide_interface_satisfied(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target_module: ModuleId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let module = origin.module();
        let requirements =
            answer!(self.interface_requirements(origin, target_module, target_instance, source)?);

        // require each member from the source
        let mut decision = Answer::Ready(true);
        for member in requirements.members {
            let lookup = answer!(self.body(module).lookup_member(
                origin,
                module,
                source,
                member.space,
                member.key
            )?);

            // require presence when an associated type stays abstract
            let Some(member_type) = member.ty else {
                if lookup.is_found() {
                    continue;
                } else {
                    return Ok(Answer::Ready(false));
                }
            };

            let Some(found) = lookup.value_type() else {
                // absent optional and defaulted members satisfy by omission
                if member.is_optional || member.has_default {
                    continue;
                }

                return Ok(Answer::Ready(false));
            };

            let relation = member.role.conformance_relation();
            let assignment = self.decide_relation(origin, relation, found, member_type)?;
            decision = decision.and(assignment);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        // require each inherited interface through ordinary interface relation
        for inherited in requirements.inherited {
            let interface = self.intern_type(
                module,
                dir::Type::Instance(dir::GenericInstance {
                    symbol: inherited.instance.symbol,
                    arguments: inherited.instance.arguments,
                }),
            )?;
            decision = decision.and(self.decide_relation(
                origin,
                Relation::Implements,
                source,
                interface,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return requirements imposed by one interface application.
    pub(in crate::check) fn interface_requirements(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<InterfaceRequirements>> {
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(receiver);

        self.interface_requirements_with_substitution(origin, instance.symbol, &substitution)
    }

    /// Return requirements imposed by one interface under an existing substitution.
    pub(in crate::check) fn interface_requirements_with_substitution(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Answer<InterfaceRequirements>> {
        let Some(dir::Definition::Interface(definition)) = self.definition(symbol)? else {
            return Ok(Answer::Ready(InterfaceRequirements {
                members: SmallVec::new(),
                inherited: SmallVec::new(),
            }));
        };
        let inherited = definition.extends.clone();
        let mut members = SmallVec::new();

        // collect only the named interface's own members
        answer!(self.collect_interface_members(origin, symbol, substitution, &mut members)?);
        let inherited = answer!(self.apply_interface_heritage(origin, substitution, inherited)?);

        Ok(Answer::Ready(InterfaceRequirements { members, inherited }))
    }

    /// Collect direct members required by one applied interface.
    fn collect_interface_members(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        substitution: &TypeSubstitution,
        required: &mut SmallVec<[InterfaceMember; 8]>,
    ) -> CompilerResult<Answer<()>> {
        let Some(dir::Definition::Interface(definition)) = self.definition(symbol)? else {
            return Ok(Answer::Ready(()));
        };
        let members = definition.members.clone();

        // collect direct interface members with applied arguments
        for member in members {
            let Some(role) = MemberRole::from_definition(&member) else {
                continue;
            };
            let Some(key) = member.key() else {
                continue;
            };
            let ty = match answer!(self.definition_member_type(&member)?) {
                Some(ty) => Some(self.substitute_type(origin.module(), ty, substitution)?),
                ty => ty,
            };
            let (is_optional, is_readonly) = match &member {
                dir::DefinitionMember::Field(field) => (field.is_optional, field.is_readonly),
                _ => (false, false),
            };
            required.push(InterfaceMember {
                space: member.space(),
                key,
                ty,
                role,
                has_default: member.is_default(),
                is_optional,
                is_readonly,
            });
        }

        Ok(Answer::Ready(()))
    }

    /// Collect one interface application's instance fields, inherited first.
    pub(in crate::check) fn interface_instance_fields(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::TypeField>>>> {
        if !matches!(
            self.symbol_kind(instance.symbol),
            dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
        ) {
            return Ok(Answer::Ready(None));
        }

        let requirements =
            answer!(self.interface_requirements(origin, instance_module, instance, receiver)?);
        let mut fields = Vec::new();
        for inherited in requirements.inherited {
            let nested = answer!(self.interface_instance_fields(
                origin,
                origin.module(),
                &inherited.instance,
                receiver,
            )?);
            fields.extend(nested.unwrap_or_default());
        }
        for member in requirements.members {
            if member.space != dir::MemberSpace::Instance {
                continue;
            }
            let Some(ty) = member.ty else {
                continue;
            };

            fields.push(dir::TypeField {
                key: member.key,
                ty,
                is_optional: member.is_optional || member.has_default,
                is_readonly: member.is_readonly,
            });
        }

        Ok(Answer::Ready(Some(fields)))
    }

    /// Apply interface and receiver substitutions to direct heritage clauses.
    fn apply_interface_heritage(
        &mut self,
        origin: Origin,
        substitution: &TypeSubstitution,
        heritages: Vec<dir::NominalHeritage>,
    ) -> CompilerResult<Answer<SmallVec<[HeritageApplication; 8]>>> {
        let module = origin.module();
        let mut applied = SmallVec::new();

        // substitute direct inherited interface applications
        for heritage in heritages {
            let instance =
                answer!(self.substituted_heritage(origin, module, substitution, &heritage)?);

            applied.push(HeritageApplication {
                source: heritage.source,
                instance,
            });
        }

        Ok(Answer::Ready(applied))
    }

    /// Decide whether one member owner satisfies one interface.
    pub(in crate::check) fn member_owner_implements_interface(
        &mut self,
        origin: Origin,
        module: ModuleId,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        owner: dir::GlobalSymbolId,
        owner_arguments: &[dir::GenericArgumentBinding],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // prove extension owners through their declared implemented interfaces
        if matches!(self.definition(owner)?, Some(dir::Definition::Extension(_))) {
            let owner_arguments = owner_arguments
                .iter()
                .map(|argument| argument.argument)
                .collect::<Vec<_>>();

            return self.extension_instance_implements_interface(
                origin,
                module,
                interface_module,
                owner,
                &owner_arguments,
                interface,
            );
        }

        // prove nominal owners through the regular implements relation
        let interface = self.intern_type(module, dir::Type::Instance(*interface))?;

        self.decide_relation(origin, Relation::Implements, receiver, interface)
    }

    /// Decide whether one member owner names or inherits one protocol.
    pub(in crate::check) fn member_owner_has_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        owner: dir::GlobalSymbolId,
        owner_arguments: &[dir::GenericArgumentBinding],
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        // prove extension owners through their declared implemented interfaces
        if matches!(self.definition(owner)?, Some(dir::Definition::Extension(_))) {
            let owner_arguments = owner_arguments
                .iter()
                .map(|argument| argument.argument)
                .collect::<Vec<_>>();

            return self.extension_instance_has_protocol(
                origin,
                module,
                owner,
                &owner_arguments,
                protocol,
            );
        }

        // reduce nominal owners to their applied instance
        let receiver = answer!(self.reduce_type_head(origin, receiver)?);
        let (instance_module, instance) = match self.ty(receiver)? {
            dir::Type::Form(form) => match self.ty(form.value)? {
                dir::Type::Instance(instance) => (form.value.module_id, Some(instance)),
                _ => (receiver.module_id, None),
            },
            dir::Type::Instance(instance) => (receiver.module_id, Some(instance)),
            _ => (receiver.module_id, None),
        };
        let Some(instance) = instance else {
            return Ok(Answer::Ready(false));
        };

        // accept direct protocol instances before searching heritage
        if instance.symbol == protocol {
            return Ok(Answer::Ready(true));
        }

        let inherited =
            answer!(self.heritage_instance(origin, instance_module, &instance, protocol)?);

        Ok(Answer::Ready(inherited.is_some()))
    }

    /// Decide whether one applied extension implements one interface.
    fn extension_instance_implements_interface(
        &mut self,
        origin: Origin,
        module: ModuleId,
        interface_module: ModuleId,
        extension_symbol: dir::GlobalSymbolId,
        extension_arguments: &[dir::GlobalTypeId],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let Some((implements, substitution)) =
            self.applied_extension_implements(module, extension_symbol, extension_arguments)?
        else {
            return Ok(Answer::Ready(false));
        };

        self.extension_implements_interface(
            origin,
            module,
            interface_module,
            &substitution,
            &implements,
            interface,
        )
    }

    /// Decide whether one applied extension names or inherits one protocol.
    fn extension_instance_has_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        extension_symbol: dir::GlobalSymbolId,
        extension_arguments: &[dir::GlobalTypeId],
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        let Some((implements, substitution)) =
            self.applied_extension_implements(module, extension_symbol, extension_arguments)?
        else {
            return Ok(Answer::Ready(false));
        };

        self.extension_has_protocol(origin, module, &substitution, &implements, protocol)
    }

    /// Return one applied extension's implemented heritage and substitution.
    fn applied_extension_implements(
        &mut self,
        module: ModuleId,
        extension_symbol: dir::GlobalSymbolId,
        extension_arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<(Vec<dir::NominalHeritage>, TypeSubstitution)>> {
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)? else {
            return Ok(None);
        };
        let implements = extension.implements.clone();
        if implements.is_empty() {
            return Ok(None);
        }

        // substitute applied extension arguments into implemented interfaces
        let arguments = self.intern_type_ids(module, extension_arguments)?;
        let extension = dir::GenericInstance {
            symbol: extension_symbol,
            arguments,
        };
        let substitution = self.instance_substitution(module, &extension)?;

        Ok(Some((implements, substitution)))
    }

    /// Decide whether one extension implementation covers one requested interface.
    pub(in crate::check) fn extension_implements_interface(
        &mut self,
        origin: Origin,
        module: ModuleId,
        interface_module: ModuleId,
        substitution: &TypeSubstitution,
        implements: &[dir::NominalHeritage],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // compare each declared implemented interface
        let interface_arguments = self
            .type_ids(interface_module, interface.arguments)?
            .to_vec();
        for heritage in implements {
            let implemented =
                answer!(self.substituted_heritage(origin, module, substitution, heritage)?);
            let matches = if implemented.symbol == interface.symbol {
                let implemented_arguments = self.type_ids(module, implemented.arguments)?.to_vec();

                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                self.relate_type_arguments(
                    cause,
                    interface.symbol,
                    self.default_symbol_context(interface.symbol),
                    Relation::Assignable,
                    &implemented_arguments,
                    &interface_arguments,
                )?
            } else if let Some(inherited) =
                answer!(self.heritage_instance(origin, module, &implemented, interface.symbol)?)
            {
                let inherited_arguments = self
                    .type_ids(origin.module(), inherited.arguments)?
                    .to_vec();

                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                self.relate_type_arguments(
                    cause,
                    interface.symbol,
                    self.default_symbol_context(interface.symbol),
                    Relation::Assignable,
                    &inherited_arguments,
                    &interface_arguments,
                )?
            } else {
                Answer::Ready(false)
            };

            if !matches!(matches, Answer::Ready(false)) {
                return Ok(matches);
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Decide whether implemented heritage names or inherits one protocol.
    pub(in crate::check) fn extension_has_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &TypeSubstitution,
        implements: &[dir::NominalHeritage],
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        // compare each declared interface by protocol symbol
        for heritage in implements {
            let implemented =
                answer!(self.substituted_heritage(origin, module, substitution, heritage)?);
            if implemented.symbol == protocol {
                return Ok(Answer::Ready(true));
            }
            if answer!(self.heritage_instance(origin, module, &implemented, protocol)?).is_some() {
                return Ok(Answer::Ready(true));
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Return implemented heritage after extension generic substitution.
    pub(in crate::check) fn substituted_heritage(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &TypeSubstitution,
        heritage: &dir::NominalHeritage,
    ) -> CompilerResult<Answer<dir::GenericInstance>> {
        let mut arguments = Vec::new();

        // substitute and normalize each inherited argument; open roots pass
        //  through raw so bounds can flow into them
        for argument in heritage.arguments.iter().copied() {
            let argument = self.substitute_type(module, argument, substitution)?;
            let argument = self.settled_root(argument)?;
            let argument = match self.root_variable(argument)? {
                Some(_) => argument,
                None => answer!(self.reduce_type_head(origin, argument)?),
            };

            arguments.push(argument);
        }
        let arguments = self.intern_type_ids(module, &arguments)?;

        Ok(Answer::Ready(dir::GenericInstance {
            symbol: heritage.symbol,
            arguments,
        }))
    }
}
