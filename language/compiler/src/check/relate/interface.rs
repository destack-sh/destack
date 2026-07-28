use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, Cause, CauseKind, CheckState, MemberRole, Origin, Relation, TypeSubstitution,
    answer,
};
use crate::{CompilerError, CompilerResult};

use super::nominal::HeritageApplication;

/// Requirements imposed by one applied interface.
#[derive(Debug, Clone)]
pub(in crate::check) struct InterfaceRequirements {
    /// The members declared directly by the interface.
    pub(in crate::check) members: SmallVec<[InterfaceMember; 8]>,
    /// The directly inherited interface applications.
    pub(in crate::check) inherited: SmallVec<[HeritageApplication; 8]>,
    /// The index signatures declared directly by the interface.
    pub(in crate::check) index_signatures: SmallVec<[InterfaceIndexSignature; 2]>,
}

/// One index signature required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct InterfaceIndexSignature {
    /// The signature's source declaration.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The declared key domain.
    pub(in crate::check) key_type: dir::GlobalTypeId,
    /// The declared value type.
    pub(in crate::check) value_type: dir::GlobalTypeId,
    /// Whether lookups may miss.
    pub(in crate::check) is_optional: bool,
    /// Whether writes are rejected.
    pub(in crate::check) is_readonly: bool,
}

/// One member required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct InterfaceMember {
    /// The interface member's source declaration.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
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
    /// Decide one relation from a source to an applied interface.
    pub(in crate::check) fn decide_interface_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let (_, target_instance) = self.require_nominal_application(target)?;
        let is_nominal = match self.definition(target_instance.symbol)? {
            Some(dir::Definition::Interface(interface)) => interface.is_nominal,
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("interface relation target {target:?} is not an interface"),
                });
            }
        };

        // apply automatic conformance declared by the target interface
        let auto_interface = self
            .language_item(target_instance.symbol)?
            .and_then(dir::AutoInterface::from_language_item)
            .filter(|interface| interface.has_auto_conformance());
        if let Some(auto_interface) = auto_interface {
            return self.satisfies_auto_interface(origin, source, auto_interface);
        }

        // dynamic values carry their erased interface constraint
        if let dir::Type::Dynamic(dynamic) = self.ty(source)? {
            return self.decide_relation(origin, relation, dynamic.constraint, target);
        }

        // find the target interface in the source heritage closure
        let application = if let dir::Type::Application(source_instance) = self.ty(source)? {
            if source_instance.symbol == target_instance.symbol {
                Some((source.module_id, source_instance))
            } else {
                let inherited = answer!(self.heritage_instance(
                    origin,
                    source.module_id,
                    &source_instance,
                    target_instance.symbol,
                )?);

                match inherited {
                    Some(inherited) => Some(self.require_nominal_application(inherited)?),
                    None => None,
                }
            }
        } else {
            None
        };

        // compare the selected interface arguments by their declared variance
        if let Some((application_module, application)) = application {
            let source_arguments = self
                .type_ids(application_module, application.arguments)?
                .to_vec();
            let target_arguments = self
                .type_ids(target.module_id, target_instance.arguments)?
                .to_vec();
            let form = self.default_variance_form(target_instance.symbol);
            let arguments = if relation == Relation::Subtype {
                self.decide_type_arguments(
                    origin,
                    target_instance.symbol,
                    form,
                    relation,
                    &source_arguments,
                    &target_arguments,
                )?
            } else {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                self.relate_type_arguments(
                    origin,
                    cause,
                    target_instance.symbol,
                    form,
                    Relation::Assignable,
                    &source_arguments,
                    &target_arguments,
                )?
            };

            // explicit implementation also requires the declared members
            if relation == Relation::Implements {
                let requirements =
                    self.decide_interface_requirements(origin, relation, source, target)?;

                return Ok(arguments.and(requirements));
            }

            return Ok(arguments);
        }

        // select a visible extension implementation
        let module = origin.module();
        let implemented = self.body().decide_extension_implementation(
            origin,
            if relation == Relation::Subtype {
                Relation::Subtype
            } else {
                Relation::Assignable
            },
            module,
            target.module_id,
            source,
            &target_instance,
            None,
        )?;
        if !matches!(implemented, Answer::Ready(false)) {
            return Ok(implemented);
        }

        // non-nominal interfaces permit structural conformance
        if !is_nominal {
            return self.decide_interface_requirements(origin, relation, source, target);
        }

        Ok(Answer::Ready(false))
    }

    /// Decide one structural relation between declaration members.
    pub(in crate::check) fn decide_member_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        role: MemberRole,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        if role.is_callable() {
            self.decide_method_relation(origin, relation, source, target)
        } else {
            self.decide_relation(origin, relation, source, target)
        }
    }

    /// Select one declared implementation of a requested interface.
    pub(in crate::check) fn match_implemented_interface(
        &mut self,
        origin: Origin,
        relation: Relation,
        module: ModuleId,
        interface_module: ModuleId,
        substitution: &TypeSubstitution,
        implementations: &[dir::InterfaceImplementation],
        interface: &dir::GenericApplication,
    ) -> CompilerResult<Answer<Option<(dir::GlobalTypeId, usize)>>> {
        // compare each declared implemented interface
        let interface_arguments = self
            .type_ids(interface_module, interface.arguments)?
            .to_vec();
        for (index, implementation) in implementations.iter().enumerate() {
            let heritage = &implementation.interface;
            let implemented = self.substitute_type(module, heritage.ty, substitution)?;
            let (implemented_module, implemented_instance) =
                self.require_nominal_application(implemented)?;
            let (matches, matched) = if implemented_instance.symbol == interface.symbol {
                let implemented_arguments = self
                    .type_ids(implemented_module, implemented_instance.arguments)?
                    .to_vec();

                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                let form = self.default_variance_form(interface.symbol);
                let matches = self.relate_type_arguments(
                    origin,
                    cause,
                    interface.symbol,
                    form,
                    relation,
                    &implemented_arguments,
                    &interface_arguments,
                )?;

                (matches, implemented)
            } else if let Some(inherited) = answer!(self.heritage_instance(
                origin,
                implemented_module,
                &implemented_instance,
                interface.symbol,
            )?) {
                let (inherited_module, inherited_instance) =
                    self.require_nominal_application(inherited)?;
                let inherited_arguments = self
                    .type_ids(inherited_module, inherited_instance.arguments)?
                    .to_vec();

                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                let form = self.default_variance_form(interface.symbol);
                let matches = self.relate_type_arguments(
                    origin,
                    cause,
                    interface.symbol,
                    form,
                    relation,
                    &inherited_arguments,
                    &interface_arguments,
                )?;

                (matches, inherited)
            } else {
                (Answer::Ready(false), implemented)
            };

            match matches {
                Answer::Ready(true) => return Ok(Answer::Ready(Some((matched, index)))),
                Answer::Ready(false) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        Ok(Answer::Ready(None))
    }
    /// Decide whether one source satisfies one interface's declared members.
    pub(in crate::check) fn decide_interface_requirements(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let (target_module, target_instance) = self.require_nominal_application(target)?;
        let requirements = answer!(self.interface_requirements(
            origin,
            target_module,
            &target_instance,
            source
        )?);
        let module = origin.module();

        // require each member from the source
        let mut decision = Answer::Ready(true);
        for member in requirements.members {
            let lookup = answer!(self.body().lookup_member(
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
                }

                return Ok(Answer::Ready(false));
            };

            let Some(found) = self.body().member_read_type(origin, &lookup)? else {
                // absent optional and defaulted members satisfy by omission
                if member.is_optional || member.has_default {
                    continue;
                }

                return Ok(Answer::Ready(false));
            };

            let member_decision = match member.role {
                // setters accept writes flowing back into the source
                MemberRole::Setter => {
                    self.decide_relation(origin, Relation::Assignable, member_type, found)?
                }
                role if role.is_callable() => {
                    self.decide_method_relation(origin, Relation::Assignable, found, member_type)?
                }
                _ => self.decide_relation(origin, Relation::Assignable, found, member_type)?,
            };
            decision = decision.and(member_decision);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        // require each inherited interface through the ordinary relation
        for inherited in requirements.inherited {
            decision = decision.and(self.decide_relation(origin, relation, source, inherited.ty)?);
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
        instance: &dir::GenericApplication,
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
                index_signatures: SmallVec::new(),
            }));
        };
        let inherited = definition.extends.clone();
        let definition_members = definition.members.clone();
        let mut members = SmallVec::new();
        let mut index_signatures = SmallVec::new();

        // collect only the named interface's own members
        answer!(self.collect_interface_members(origin, symbol, substitution, &mut members)?);
        for member in &definition_members {
            let dir::DefinitionMember::IndexSignature(signature) = member else {
                continue;
            };
            index_signatures.push(InterfaceIndexSignature {
                source: signature.source,
                key_type: self.substitute_type(origin.module(), signature.key_type, substitution)?,
                value_type: self.substitute_type(
                    origin.module(),
                    signature.value_type,
                    substitution,
                )?,
                is_optional: signature.is_optional,
                is_readonly: signature.is_readonly,
            });
        }
        let inherited = self.apply_interface_heritage(origin, substitution, inherited)?;

        Ok(Answer::Ready(InterfaceRequirements {
            members,
            inherited,
            index_signatures,
        }))
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
                source: member.source(),
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

    /// Collect one interface application's instance properties, inherited first.
    pub(in crate::check) fn interface_instance_fields(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::TypeProperty>>>> {
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
            let (inherited_module, inherited_instance) =
                self.require_nominal_application(inherited.ty)?;
            let nested = answer!(self.interface_instance_fields(
                origin,
                inherited_module,
                &inherited_instance,
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

            let access = match member.is_readonly {
                true => dir::PropertyAccess::Read(ty),
                false => dir::PropertyAccess::ReadWrite {
                    read: ty,
                    write: ty,
                },
            };
            fields.push(dir::TypeProperty {
                key: member.key,
                access,
                is_optional: member.is_optional || member.has_default,
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
    ) -> CompilerResult<SmallVec<[HeritageApplication; 8]>> {
        let module = origin.module();
        let mut applied = SmallVec::new();

        // substitute direct inherited interface applications
        for heritage in heritages {
            let ty = self.substitute_type(module, heritage.ty, substitution)?;

            applied.push(HeritageApplication {
                source: heritage.source,
                ty,
            });
        }

        Ok(applied)
    }

    /// Return one applied interface's members selected by space and key.
    pub(in crate::check) fn interface_members(
        &mut self,
        origin: Origin,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<SmallVec<[InterfaceMember; 2]>>> {
        let (interface_module, instance) = self.require_nominal_application(interface)?;
        let requirements =
            answer!(self.interface_requirements(origin, interface_module, &instance, receiver)?);

        let mut members = SmallVec::new();
        for member in requirements.members {
            if member.space == space && member.key == key {
                members.push(member);
            }
        }

        // search inherited interfaces when the named interface misses
        if members.is_empty() {
            for inherited in requirements.inherited {
                let nested = answer!(self.interface_members(
                    origin,
                    inherited.ty,
                    receiver,
                    space,
                    key
                )?);
                members.extend(nested);
                if !members.is_empty() {
                    break;
                }
            }
        }

        Ok(Answer::Ready(members))
    }

}
