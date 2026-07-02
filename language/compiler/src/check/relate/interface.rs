use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, MemberRole, Origin, Relation, TypeSubstitution, answer};

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
}

impl CheckState<'_> {
    /// Decide whether one source exposes every member of one interface application.
    /// `target_module` is the owner of `target_instance`'s argument list.
    pub(in crate::check) fn decide_interface_satisfied(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target_module: ModuleId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let module = origin.module();
        let members =
            answer!(self.interface_members(origin, target_module, target_instance, source)?);

        // require each member from the source
        let mut decision = Answer::Ready(true);
        for member in members {
            let lookup =
                answer!(self.lookup_member(origin, module, source, member.space, member.key)?);

            let found = lookup.value_type();
            let Some(found) = found else {
                return Ok(Answer::Ready(false));
            };

            // associated types without values only need presence
            let Some(member_type) = member.ty else {
                continue;
            };

            let assignment = if member.role.uses_method_assignability() {
                self.decide_method_assignable(origin, found, member_type)?
            } else {
                self.decide_relation(origin, Relation::Assignable, found, member_type)?
            };
            decision = decision.and(assignment);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the members required by one interface application.
    /// `instance_module` is the owner of `instance`'s argument list.
    pub(in crate::check) fn interface_members(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[InterfaceMember; 8]>>> {
        let closure = answer!(self.heritage_closure(origin, instance_module, instance)?);
        let mut members = SmallVec::<[InterfaceMember; 8]>::new();

        // collect inherited members before direct members; heritage_closure
        // interns every inherited application's argument list into origin.module()
        for application in closure.applications {
            answer!(self.collect_interface_members(
                origin,
                origin.module(),
                &application.instance,
                receiver,
                &mut members
            )?);
        }
        answer!(self.collect_interface_members(
            origin,
            instance_module,
            instance,
            receiver,
            &mut members
        )?);

        Ok(Answer::Ready(members))
    }

    /// Collect direct members required by one applied interface.
    /// `instance_module` is the owner of `instance`'s argument list.
    fn collect_interface_members(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        receiver: dir::GlobalTypeId,
        required: &mut SmallVec<[InterfaceMember; 8]>,
    ) -> CompilerResult<Answer<()>> {
        let Some(dir::Definition::Interface(definition)) = self.definition(instance.symbol) else {
            return Ok(Answer::Ready(()));
        };
        let members = definition.members.clone();
        let substitution = self
            .instance_substitution(instance_module, instance)?
            .with_receiver(receiver);

        // collect direct interface members with applied arguments
        for member in members {
            let Some(role) = MemberRole::from_definition(&member) else {
                continue;
            };
            let Some(key) = member.key() else {
                continue;
            };
            let ty = match answer!(self.definition_member_type(&member)?) {
                Some(ty) => Some(self.substitute_type(origin.module(), ty, &substitution)?),
                ty => ty,
            };
            required.push(InterfaceMember {
                space: member.space(),
                key,
                ty,
                role,
            });
        }

        Ok(Answer::Ready(()))
    }

    /// Decide whether one member owner satisfies one interface.
    pub(in crate::check) fn member_owner_implements_interface(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        owner: dir::GlobalSymbolId,
        owner_arguments: &[dir::GenericArgumentBinding],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // prove extension owners through their declared implemented interfaces
        if matches!(self.definition(owner), Some(dir::Definition::Extension(_))) {
            let owner_arguments = owner_arguments
                .iter()
                .map(|argument| argument.argument)
                .collect::<Vec<_>>();

            return self.extension_instance_implements_interface(
                origin,
                module,
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
        if matches!(self.definition(owner), Some(dir::Definition::Extension(_))) {
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
        extension_symbol: dir::GlobalSymbolId,
        extension_arguments: &[dir::GlobalTypeId],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(Answer::Ready(false));
        };
        let implements = extension.implements.clone();
        if implements.is_empty() {
            return Ok(Answer::Ready(false));
        }

        // substitute applied extension arguments into implemented interfaces
        let arguments = self.intern_type_ids(module, extension_arguments)?;
        let extension = dir::GenericInstance {
            symbol: extension_symbol,
            arguments,
        };
        let substitution = self.instance_substitution(module, &extension)?;

        self.extension_implements_interface(origin, module, &substitution, &implements, interface)
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
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(Answer::Ready(false));
        };
        let implements = extension.implements.clone();
        if implements.is_empty() {
            return Ok(Answer::Ready(false));
        }

        // substitute applied extension arguments into implemented interfaces
        let arguments = self.intern_type_ids(module, extension_arguments)?;
        let extension = dir::GenericInstance {
            symbol: extension_symbol,
            arguments,
        };
        let substitution = self.instance_substitution(module, &extension)?;

        self.extension_has_protocol(origin, module, &substitution, &implements, protocol)
    }

    /// Decide whether one extension implementation covers one requested interface.
    pub(in crate::check) fn extension_implements_interface(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &TypeSubstitution,
        implements: &[dir::NominalHeritage],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        // compare each declared implemented interface
        let interface_arguments = self.type_ids(module, interface.arguments)?.to_vec();
        for heritage in implements {
            let implemented = self.substituted_heritage(module, substitution, heritage)?;
            let matches = if implemented.symbol == interface.symbol {
                let implemented_arguments = self.type_ids(module, implemented.arguments)?.to_vec();

                self.relate_type_arguments(
                    origin,
                    interface.symbol,
                    &implemented_arguments,
                    &interface_arguments,
                )?
            } else if let Some(inherited) =
                answer!(self.heritage_instance(origin, module, &implemented, interface.symbol)?)
            {
                let inherited_arguments = self.type_ids(module, inherited.arguments)?.to_vec();

                self.relate_type_arguments(
                    origin,
                    interface.symbol,
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
            let implemented = self.substituted_heritage(module, substitution, heritage)?;
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
    /// The returned instance's argument list land in `module`.
    pub(in crate::check) fn substituted_heritage(
        &mut self,
        module: ModuleId,
        substitution: &TypeSubstitution,
        heritage: &dir::NominalHeritage,
    ) -> CompilerResult<dir::GenericInstance> {
        let arguments = heritage
            .arguments
            .iter()
            .map(|argument| self.substitute_type(module, *argument, substitution))
            .collect::<CompilerResult<Vec<_>>>()?;
        let arguments = self.intern_type_ids(module, &arguments)?;

        Ok(dir::GenericInstance {
            symbol: heritage.symbol,
            arguments,
        })
    }
}
