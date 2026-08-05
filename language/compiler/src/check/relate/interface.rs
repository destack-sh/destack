use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, Cause, CauseKind, CheckState, GenericParameterId, MemberRole, Origin, Relation,
    TypeSubstitution, answer,
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
    /// The call signatures declared directly by the interface.
    pub(in crate::check) call_signatures: SmallVec<[InterfaceSignature; 2]>,
    /// The construct signatures declared directly by the interface.
    pub(in crate::check) construct_signatures: SmallVec<[InterfaceSignature; 2]>,
}

/// The signature family one requirement selects from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum SignatureFamily {
    /// The call signatures of a type.
    Call,
    /// The construct signatures of a type.
    Construct,
}

/// One symbol-free signature required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct InterfaceSignature {
    /// The signature's source declaration.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The applied signature type.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// One index signature required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct InterfaceIndexSignature {
    /// The signature's source declaration.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The applied structural signature.
    pub(in crate::check) signature: dir::TypeIndexSignature,
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
    /// The member type, when the member has a value.
    pub(in crate::check) ty: Option<dir::GlobalTypeId>,
    /// How the member participates in assignability.
    pub(in crate::check) role: MemberRole,
    /// Whether the member has a default implementation.
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
        let (_, target_instance) = self.nominal_application(target)?;
        let is_nominal = match self.definition(target_instance.symbol)? {
            Some(dir::Definition::Interface(interface)) => interface.is_nominal,
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("interface relation target {target:?} is not an interface"),
                });
            }
        };
        let auto_interface = self
            .language_item(target_instance.symbol)?
            .and_then(dir::AutoInterface::from_language_item)
            .filter(|interface| interface.is_marker());

        // find the target interface in the source heritage closure
        let application = if let dir::Type::Application(source_instance) = self.ty(source)? {
            if source_instance.symbol == target_instance.symbol {
                Some((source.module_id, source_instance))
            } else {
                let inherited =
                    answer!(self.heritage_instance(origin, source, target_instance.symbol)?);

                match inherited {
                    Some(inherited) => Some(self.nominal_application(inherited)?),
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
            let form = self.default_variance_form(target_instance.symbol)?;
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
                    relation.interior(),
                    &source_arguments,
                    &target_arguments,
                )?
            };

            return Ok(arguments);
        }

        // select a visible extension implementation
        let module = origin.module();
        let implemented = self.body().decide_extension_implementation(
            origin,
            relation,
            module,
            target.module_id,
            source,
            &target_instance,
            None,
        )?;
        if !matches!(implemented, Answer::Ready(false)) {
            return Ok(implemented);
        }

        // derive marker conformance when no declaration provides it
        if let Some(auto_interface) = auto_interface {
            return self.satisfies_auto_interface(origin, source, auto_interface);
        }

        // dynamic values carry their erased interface constraint
        if let dir::Type::Dynamic(dynamic) = self.ty(source)? {
            return self.decide_relation(origin, relation, dynamic.constraint, target);
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
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        if role.is_callable() {
            self.decide_method_relation(origin, relation, source, target, receiver)
        } else {
            self.decide_relation(origin, relation, source, target)
        }
    }

    /// Select one declared implementation of a requested interface.
    pub(in crate::check) fn match_implemented_interface(
        &mut self,
        origin: Origin,
        relation: Relation,
        interface_module: ModuleId,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        implementations: &[dir::NominalHeritage],
        interface: &dir::GenericApplication,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // compare each declared implemented interface
        let interface_arguments = self
            .type_ids(interface_module, interface.arguments)?
            .to_vec();
        let arguments = self.intern_type_ids(&interface_arguments)?;
        let interface_type = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol: interface.symbol,
            arguments,
        }))?;
        for heritage in implementations {
            // fill elided arguments before matching
            let declared = self.settled_root(heritage.ty)?;
            let declared = match self.ty(declared)? {
                dir::Type::Application(instance) => {
                    match self.fill_elided_application(declared.module_id, &instance)? {
                        Some(filled) => filled,
                        None => declared,
                    }
                }
                _ => declared,
            };

            // bind open parameters here, relate them below
            let mut scratch = substitution.clone();
            let matched = self.substitute_type(declared, &scratch)?;
            if !answer!(self.extend_generic_substitution(
                origin,
                parameters,
                &mut scratch,
                &[(matched, interface_type)],
            )?) {
                continue;
            }
            let implemented = self.substitute_type(declared, &scratch)?;
            let implemented = self.settled_root(implemented)?;

            // select the implemented application naming the requested interface
            let (implemented_module, implemented_instance) =
                self.nominal_application(implemented)?;
            let implemented_symbol = self.resolve_symbol_alias(implemented_instance.symbol)?;
            let interface_symbol = self.resolve_symbol_alias(interface.symbol)?;
            let (instance, matched) = if implemented_symbol == interface_symbol {
                (
                    Some((implemented_module, implemented_instance)),
                    implemented,
                )
            } else if let Some(inherited) =
                answer!(self.heritage_instance(origin, implemented, interface_symbol)?)
            {
                (Some(self.nominal_application(inherited)?), inherited)
            } else {
                (None, implemented)
            };

            // relate the instance arguments under the declared variance
            let matches = match instance {
                Some((instance_module, instance)) => {
                    let arguments = self.type_ids(instance_module, instance.arguments)?.to_vec();
                    let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                    let form = self.default_variance_form(interface.symbol)?;

                    self.relate_type_arguments(
                        origin,
                        cause,
                        interface.symbol,
                        form,
                        relation.interior(),
                        &arguments,
                        &interface_arguments,
                    )?
                }
                None => Answer::Ready(false),
            };

            match matches {
                Answer::Ready(true) => {
                    *substitution = scratch;

                    return Ok(Answer::Ready(Some(matched)));
                }
                Answer::Ready(false) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Instantiate one interface implementation with its associated type bindings.
    pub(in crate::check) fn instantiate_interface_implementation(
        &mut self,
        origin: Origin,
        implementation: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // read the implemented interface and its written refinements
        let (mut base, mut bindings) = self.refinement_bindings(implementation)?;
        let dir::Type::Application(application) = self.ty(base)? else {
            return Err(CompilerError::Internal {
                message: format!("interface implementation {implementation:?} has no application"),
            });
        };

        // fill elided interface arguments before applying associated bindings
        if let Some(filled) = self.fill_elided_application(base.module_id, &application)? {
            base = filled;
        }
        let dir::Type::Application(application) = self.ty(base)? else {
            return Err(CompilerError::Internal {
                message: format!("filled interface {base:?} has no application"),
            });
        };
        let Some(dir::Definition::Interface(definition)) = self.definition(application.symbol)?
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "implementation target has no interface definition: {:?}",
                    application.symbol,
                ),
            });
        };
        let interface_members = definition.members.clone();

        // select written bindings, declared values, then interface defaults
        for member in &interface_members {
            let dir::DefinitionMember::AssociatedType(associated) = member else {
                continue;
            };
            let written = bindings
                .iter()
                .find(|(key, _)| *key == associated.key)
                .map(|(_, value)| *value);
            let declared = members.iter().find_map(|member| match member {
                dir::DefinitionMember::AssociatedType(candidate)
                    if candidate.key == associated.key =>
                {
                    candidate.value
                }
                _ => None,
            });
            let declared = declared
                .map(|value| self.substitute_type(value, substitution))
                .transpose()?;

            // reject conflicting source bindings
            if let (Some(written), Some(declared)) = (written, declared) {
                let written = answer!(self.reduce_type(origin, written)?);
                let declared = answer!(self.reduce_type(origin, declared)?);
                if !answer!(self.decide_equal(origin, written, declared)?) {
                    return Ok(Answer::Ready(None));
                }
            }

            // record the selected associated type
            let Some(value) = written.or(declared).or(associated.value) else {
                continue;
            };
            match bindings.iter_mut().find(|(key, _)| *key == associated.key) {
                Some((_, current)) => *current = value,
                None => bindings.push((associated.key, value)),
            }
        }

        // instantiate every binding through the complete implementation
        let implementation = self.intern_refinements(origin.module(), base, &bindings)?;
        for (_, value) in &mut bindings {
            *value = self.instantiate_interface_type(*value, implementation, receiver)?;
        }
        let implementation = self.intern_refinements(origin.module(), base, &bindings)?;

        // require each concrete binding to satisfy its declared bound
        for member in &interface_members {
            let dir::DefinitionMember::AssociatedType(associated) = member else {
                continue;
            };
            let Some(constraint) = associated.constraint else {
                continue;
            };
            let Some((_, value)) = bindings.iter().find(|(key, _)| *key == associated.key) else {
                continue;
            };
            let constraint =
                self.instantiate_interface_type(constraint, implementation, receiver)?;
            if !answer!(self.decide_relation(origin, Relation::Satisfies, *value, constraint)?) {
                return Ok(Answer::Ready(None));
            }
        }

        Ok(Answer::Ready(Some(implementation)))
    }

    /// Decide whether one source satisfies one interface's declared requirements.
    pub(in crate::check) fn decide_interface_requirements(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let requirements = answer!(self.interface_requirements(target, source)?);
        let module = origin.module();

        // require each member from the source
        let mut decision = Answer::Ready(true);
        for member in &requirements.members {
            let subject = dir::MemberSubject::new(source, source, member.space);
            let lookup = answer!(
                self.body()
                    .lookup_member(origin, module, subject, member.key)?
            );

            // require presence when an associated type stays abstract
            let Some(member_type) = member.ty else {
                if lookup.is_found() || member.has_default || member.is_optional {
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
                role if role.is_callable() => self.decide_method_relation(
                    origin,
                    Relation::Assignable,
                    found,
                    member_type,
                    None,
                )?,
                _ => self.decide_relation(origin, Relation::Assignable, found, member_type)?,
            };
            decision = decision.and(member_decision);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        // prove each direct signature from the source
        let signatures =
            answer!(self.decide_interface_signatures(origin, relation, source, &requirements,)?);
        if !signatures {
            return Ok(Answer::Ready(false));
        }

        // preserve each inherited interface's structural or nominal identity
        for inherited in requirements.inherited {
            decision =
                decision.and(self.decide_relation(origin, relation, source, inherited.ty)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source satisfies an interface's direct signatures.
    pub(in crate::check) fn decide_interface_signatures(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        requirements: &InterfaceRequirements,
    ) -> CompilerResult<Answer<bool>> {
        // prove each required call signature from the source
        for signature in &requirements.call_signatures {
            let satisfied = answer!(self.decide_signature_requirement(
                origin,
                source,
                signature.ty,
                SignatureFamily::Call,
            )?);
            if !satisfied {
                return Ok(Answer::Ready(false));
            }
        }

        // prove each required construct signature from the source
        for signature in &requirements.construct_signatures {
            let satisfied = answer!(self.decide_signature_requirement(
                origin,
                source,
                signature.ty,
                SignatureFamily::Construct,
            )?);
            if !satisfied {
                return Ok(Answer::Ready(false));
            }
        }

        // prove each required index signature from the source
        for signature in &requirements.index_signatures {
            let satisfied = answer!(self.decide_index_signature_satisfied(
                origin,
                relation,
                source,
                &signature.signature,
            )?);
            if !satisfied {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Decide whether one source's signature surface satisfies a required signature.
    fn decide_signature_requirement(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        required: dir::GlobalTypeId,
        family: SignatureFamily,
    ) -> CompilerResult<Answer<bool>> {
        let head = answer!(self.reduce_type_head(origin, source)?);

        // relate function-typed sources through their own signature
        let is_function = match self.ty(head)? {
            dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_) => true,
            dir::Type::Application(callable) => self.is_function_language_item(callable.symbol)?,
            _ => false,
        };
        if is_function {
            return match family {
                SignatureFamily::Call => {
                    self.decide_method_relation(origin, Relation::Assignable, head, required, None)
                }
                SignatureFamily::Construct => Ok(Answer::Ready(false)),
            };
        }

        // relate other sources through their apparent signatures
        let constraint = match self.ty(head)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            dir::Type::Application(_) => head,
            _ => return Ok(Answer::Ready(false)),
        };
        let signatures = answer!(self.apparent_signatures(origin, constraint, family)?);
        for signature in signatures {
            let satisfied = answer!(self.decide_method_relation(
                origin,
                Relation::Assignable,
                signature.ty,
                required,
                None,
            )?);
            if satisfied {
                return Ok(Answer::Ready(true));
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Return requirements imposed by one interface application.
    pub(in crate::check) fn interface_requirements(
        &mut self,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<InterfaceRequirements>> {
        // read the named interface definition
        let (_, instance) = self.nominal_application(interface)?;
        let symbol = instance.symbol;
        let Some(dir::Definition::Interface(definition)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("interface requirements target {interface:?} is not an interface"),
            });
        };

        // collect the interface's declared members and heritage
        let inherited = definition.extends.clone();
        let definition_members = definition.members.clone();
        let mut members = SmallVec::new();
        let mut index_signatures = SmallVec::new();
        let mut call_signatures = SmallVec::new();
        let mut construct_signatures = SmallVec::new();

        // collect only the named interface's own members
        answer!(self.collect_interface_members(symbol, interface, receiver, &mut members,)?);
        for member in &definition_members {
            // apply the substitution to each index signature domain
            if let dir::DefinitionMember::IndexSignature(signature) = member {
                index_signatures.push(InterfaceIndexSignature {
                    source: signature.source,
                    signature: dir::TypeIndexSignature {
                        name: signature.name,
                        key_type: self.instantiate_interface_type(
                            signature.key_type,
                            interface,
                            receiver,
                        )?,
                        value_type: self.instantiate_interface_type(
                            signature.value_type,
                            interface,
                            receiver,
                        )?,
                        is_optional: signature.is_optional,
                        is_readonly: signature.is_readonly,
                    },
                });
            }
            // apply the substitution to each call signature
            else if let dir::DefinitionMember::CallSignature(signature) = member {
                call_signatures.push(InterfaceSignature {
                    source: signature.source,
                    ty: self.instantiate_interface_type(signature.ty, interface, receiver)?,
                });
            }
            // apply the substitution to each construct signature
            else if let dir::DefinitionMember::ConstructSignature(signature) = member {
                construct_signatures.push(InterfaceSignature {
                    source: signature.source,
                    ty: self.instantiate_interface_type(signature.ty, interface, receiver)?,
                });
            }
        }
        let inherited = self.apply_interface_heritage(interface, receiver, inherited)?;

        Ok(Answer::Ready(InterfaceRequirements {
            members,
            inherited,
            index_signatures,
            call_signatures,
            construct_signatures,
        }))
    }

    /// Return the apparent signatures of one interface constraint.
    pub(in crate::check) fn apparent_signatures(
        &mut self,
        origin: Origin,
        constraint: dir::GlobalTypeId,
        family: SignatureFamily,
    ) -> CompilerResult<Answer<SmallVec<[InterfaceSignature; 2]>>> {
        let constraint = answer!(self.reduce_type_head(origin, constraint)?);
        if self.nominal_application_maybe(constraint)?.is_none() {
            return Ok(Answer::Ready(SmallVec::new()));
        }
        let requirements = answer!(self.interface_requirements(constraint, constraint)?);
        let mut signatures = match family {
            SignatureFamily::Call => requirements.call_signatures.clone(),
            SignatureFamily::Construct => requirements.construct_signatures.clone(),
        };

        // collect apparent signatures from inherited interfaces
        for heritage in &requirements.inherited {
            let nested = answer!(self.apparent_signatures(origin, heritage.ty, family)?);
            signatures.extend(nested);
        }

        Ok(Answer::Ready(signatures))
    }

    /// Return whether one symbol declares the callable value language item.
    pub(in crate::check) fn is_function_language_item(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let item = self.language_item(symbol)?;

        Ok(matches!(
            item,
            Some(dir::LanguageItem::Function | dir::LanguageItem::FunctionPointer)
        ))
    }

    /// Collect direct members required by one applied interface.
    fn collect_interface_members(
        &mut self,
        symbol: dir::GlobalSymbolId,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        required: &mut SmallVec<[InterfaceMember; 8]>,
    ) -> CompilerResult<Answer<()>> {
        let Some(dir::Definition::Interface(definition)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("interface member source {symbol:?} is not an interface"),
            });
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
            // associated types bound implementers by constraint; a written
            //  value is a default the implementer may override
            let (declared, has_default) = match &member {
                dir::DefinitionMember::AssociatedType(associated) => {
                    (associated.constraint, associated.value.is_some())
                }
                _ => (
                    answer!(self.definition_member_type(&member)?),
                    member.is_default(),
                ),
            };
            let ty = match declared {
                Some(ty) => Some(self.instantiate_interface_type(ty, interface, receiver)?),
                None => None,
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
                has_default,
                is_optional,
                is_readonly,
            });
        }

        Ok(Answer::Ready(()))
    }

    /// Collect one interface application's instance properties, inherited first.
    pub(in crate::check) fn interface_instance_fields(
        &mut self,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<Vec<dir::TypeProperty>>>> {
        let (_, instance) = self.nominal_application(interface)?;
        if !matches!(
            self.symbol_kind(instance.symbol)?,
            dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
        ) {
            return Ok(Answer::Ready(None));
        }

        // collect inherited interface fields first
        let requirements = answer!(self.interface_requirements(interface, receiver)?);
        let mut fields = Vec::new();
        for inherited in requirements.inherited {
            let Some(nested) = answer!(self.interface_instance_fields(inherited.ty, receiver)?)
            else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "inherited interface application {:?} is not an interface",
                        inherited.ty,
                    ),
                });
            };
            fields.extend(nested);
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

    /// Instantiate direct heritage clauses under one interface implementation.
    fn apply_interface_heritage(
        &mut self,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        heritages: Vec<dir::NominalHeritage>,
    ) -> CompilerResult<SmallVec<[HeritageApplication; 8]>> {
        let mut applied = SmallVec::new();

        // instantiate each direct inherited interface application
        for heritage in heritages {
            let ty = self.instantiate_interface_type(heritage.ty, interface, receiver)?;

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
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<SmallVec<[InterfaceMember; 2]>>> {
        // select the interface's own members by space and key
        let requirements = answer!(self.interface_requirements(interface, receiver)?);
        let mut members = SmallVec::new();
        for member in requirements.members {
            if member.space == space && member.key == key {
                members.push(member);
            }
        }

        // search inherited interfaces when the named interface misses
        if members.is_empty() {
            for inherited in requirements.inherited {
                let nested = answer!(self.interface_members(inherited.ty, receiver, space, key)?);
                members.extend(nested);
                if !members.is_empty() {
                    break;
                }
            }
        }

        Ok(Answer::Ready(members))
    }
}
