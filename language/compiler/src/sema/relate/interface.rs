use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseId, CauseKind, CheckState, GenericParameterId, MemberRole, Origin, Relation,
    TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult};

use super::nominal::HeritageApplication;

/// Requirements imposed by one applied interface.
#[derive(Debug, Clone)]
pub(in crate::sema) struct InterfaceRequirements {
    /// The members declared directly by the interface.
    pub(in crate::sema) members: SmallVec<[InterfaceMember; 8]>,
    /// The directly inherited interface applications.
    pub(in crate::sema) inherited: SmallVec<[HeritageApplication; 8]>,
    /// The index signatures declared directly by the interface.
    pub(in crate::sema) index_signatures: SmallVec<[InterfaceIndexSignature; 2]>,
    /// The call signatures declared directly by the interface.
    pub(in crate::sema) call_signatures: SmallVec<[InterfaceSignature; 2]>,
    /// The construct signatures declared directly by the interface.
    pub(in crate::sema) construct_signatures: SmallVec<[InterfaceSignature; 2]>,
}

/// The signature family one requirement selects from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum SignatureFamily {
    /// The call signatures of a type.
    Call,
    /// The construct signatures of a type.
    Construct,
}

/// One symbol-free signature required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct InterfaceSignature {
    /// The signature's source declaration.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The applied signature type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
}

/// One index signature required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct InterfaceIndexSignature {
    /// The signature's source declaration.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The applied structural signature.
    pub(in crate::sema) signature: dir::TypeIndexSignature,
}

/// One member required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct InterfaceMember {
    /// The required interface member symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The interface member's source declaration.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The member space.
    pub(in crate::sema) space: dir::MemberSpace,
    /// The member key.
    pub(in crate::sema) key: dir::StaticKey,
    /// The member type, when the member has a value.
    pub(in crate::sema) ty: Option<dir::GlobalTypeId>,
    /// How the member participates in assignability.
    pub(in crate::sema) role: MemberRole,
    /// Whether the member has a default implementation.
    pub(in crate::sema) has_default: bool,
    /// Whether the member is optional on its declaration.
    pub(in crate::sema) is_optional: bool,
    /// Whether the member rejects writes after initialization.
    pub(in crate::sema) is_readonly: bool,
}

impl CheckState<'_> {
    /// Relate one source to an applied interface.
    pub(in crate::sema) fn relate_interface(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read the target interface declaration
        let (_, target_instance) = self.nominal_application(target)?;
        let is_nominal = match self.definition(target_instance.symbol)? {
            Some(dir::Definition::Interface(interface)) => interface.is_nominal,
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("interface relation target {target:?} is not an interface"),
                });
            }
        };

        // classify the interfaces the compiler decides itself
        let auto_interface = self
            .language_item(target_instance.symbol)?
            .and_then(dir::AutoInterface::from_language_item)
            .filter(|interface| interface.is_intrinsic());

        // find the target interface in the source heritage closure
        let application = if let dir::Type::Application(source_instance) = self.ty(source)? {
            if source_instance.symbol == target_instance.symbol {
                Some((source.module_id, source_instance))
            } else {
                let inherited =
                    self.heritage_instance(origin, source, source, target_instance.symbol)?;

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
            let source_arguments: SmallVec<[_; 8]> = self
                .type_ids(application_module, application.arguments)?
                .into();
            let target_arguments: SmallVec<[_; 8]> = self
                .type_ids(target.module_id, target_instance.arguments)?
                .into();
            let form = self.default_variance_form(target_instance.symbol)?;
            let argument_relation = match relation {
                Relation::Subtype => relation,
                _ => relation.interior(),
            };
            return self.relate_type_arguments(
                origin,
                cause,
                target_instance.symbol,
                form,
                argument_relation,
                &source_arguments,
                &target_arguments,
            );
        }

        // select a visible extension implementation
        let implemented = self.body().decide_extension_implementation(
            origin,
            relation,
            target.module_id,
            source,
            &target_instance,
            None,
        )?;
        if implemented == Verdict::Holds {
            return Ok(Verdict::Holds);
        }

        // use intrinsic conformance when no declaration provides it
        if let Some(auto_interface) = auto_interface {
            let decided =
                self.satisfies_intrinsic_interface(origin, source, target, auto_interface)?;

            return Ok(decided.join_undecided(implemented));
        }

        // dynamic values carry their erased interface constraint
        if let dir::Type::Dynamic(dynamic) = self.ty(source)? {
            return self.constrain_type(origin, cause, relation, dynamic.constraint, target);
        }

        // structural interfaces conform by shape
        if !is_nominal {
            return self.relate_interface_requirements(origin, cause, relation, source, target);
        }

        Ok(implemented)
    }

    /// Relate two declaration members structurally.
    pub(in crate::sema) fn relate_member(
        &mut self,
        origin: Origin,
        relation: Relation,
        role: MemberRole,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Verdict> {
        if role.is_callable() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

            self.relate_method(origin, cause, relation, source, target, receiver)
        } else {
            self.evaluate_relation(origin, relation, source, target)
        }
    }

    /// Select one declared implementation of a requested interface.
    pub(in crate::sema) fn match_implemented_interface(
        &mut self,
        origin: Origin,
        relation: Relation,
        interface_module: ModuleId,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        interfaces: &[dir::GlobalTypeId],
        interface: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // compare each declared implemented interface
        let interface_arguments: SmallVec<[_; 8]> =
            self.type_ids(interface_module, interface.arguments)?.into();
        let arguments = self.intern_type_ids(&interface_arguments)?;
        let interface_type = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol: interface.symbol,
            arguments,
        }))?;
        for implemented in interfaces {
            // fill elided arguments before matching
            let declared = self.shallow_resolve(*implemented)?;
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
            if !self.extend_generic_substitution(
                origin,
                parameters,
                &mut scratch,
                &[(matched, interface_type)],
            )? {
                continue;
            }

            // apply the fresh bindings to the declared interface
            let implemented = self.substitute_type(declared, &scratch)?;
            let implemented = self.shallow_resolve(implemented)?;

            // select the implemented application naming the requested interface
            let (implemented_module, implemented_instance) =
                self.nominal_application(implemented)?;
            let (instance, matched) = if implemented_instance.symbol == interface.symbol {
                (
                    Some((implemented_module, implemented_instance)),
                    implemented,
                )
            } else if let Some(inherited) = self.heritage_instance(
                origin,
                implemented,
                scratch.receiver.unwrap_or(implemented),
                interface.symbol,
            )? {
                (Some(self.nominal_application(inherited)?), inherited)
            } else {
                (None, implemented)
            };

            // relate the instance arguments under the declared variance
            let is_matched = match instance {
                Some((instance_module, instance)) => {
                    let arguments: SmallVec<[_; 8]> =
                        self.type_ids(instance_module, instance.arguments)?.into();
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
                    .holds()
                }
                None => false,
            };

            // commit the bindings of the first implementation that matched
            if is_matched {
                *substitution = scratch;

                return Ok(Some(matched));
            }
        }

        Ok(None)
    }

    /// Instantiate one interface implementation with its associated type bindings.
    pub(in crate::sema) fn instantiate_interface_implementation(
        &mut self,
        origin: Origin,
        implementation: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        substitution: &TypeSubstitution,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
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

        // read the interface declaration behind the filled application
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
                let written = self.deeply_resolve(origin, written)?;
                let declared = self.deeply_resolve(origin, declared)?;
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
                if !self.relate_equal(origin, cause, written, declared)?.holds() {
                    return Ok(None);
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
        let implementation = self.intern_refinements(base, &bindings)?;
        for (_, value) in &mut bindings {
            *value = self.instantiate_interface_type(*value, implementation, receiver)?;
        }
        let implementation = self.intern_refinements(base, &bindings)?;

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
            if self.evaluate_relation(origin, Relation::Satisfies, *value, constraint)?
                == Verdict::Fails
            {
                return Ok(None);
            }
        }

        Ok(Some(implementation))
    }

    /// Relate one source against an interface's declared requirements.
    pub(in crate::sema) fn relate_interface_requirements(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // read what the applied interface demands of the source
        let requirements = self.interface_requirements(target, source)?;
        let module = origin.module();

        // require each member from the source
        let mut verdict = Verdict::Holds;
        for member in &requirements.members {
            let subject = dir::MemberSubject::new(source, source, member.space);
            let lookup = self
                .body()
                .lookup_member(origin, module, subject, member.key)?;

            // require presence when an associated type stays abstract
            let Some(member_type) = member.ty else {
                if lookup.is_found() || member.has_default || member.is_optional {
                    continue;
                }

                return Ok(Verdict::Fails);
            };

            // relate the found member against the requirement by its role
            let member_type = self.shallow_resolve(member_type)?;
            let access = self.property_access(member.role, member_type, member.is_readonly)?;
            let member_decision = match access {
                // properties relate their complete read and write operations
                Some(required) if member.role != MemberRole::Method => {
                    let Some(found) = self.body().member_binding(member.key, &lookup)? else {
                        if member.is_optional || member.has_default {
                            continue;
                        }

                        return Ok(Verdict::Fails);
                    };
                    let source = dir::TypeProperty {
                        key: member.key,
                        access: found.access,
                        is_optional: found.is_optional,
                    };
                    let target = dir::TypeProperty {
                        key: member.key,
                        access: required,
                        is_optional: member.is_optional,
                    };
                    let Some(relations) = self.shape_property_relations(
                        Relation::Assignable,
                        false,
                        &source,
                        &target,
                    ) else {
                        return Ok(Verdict::Fails);
                    };

                    self.relate_shape_fields(origin, cause, &relations)?
                }
                // methods compare callable signatures without their receivers
                Some(_) => {
                    let Some(found) = self.body().member_read_type(&lookup)? else {
                        if member.is_optional || member.has_default {
                            continue;
                        }

                        return Ok(Verdict::Fails);
                    };

                    self.relate_method(
                        origin,
                        cause,
                        Relation::Assignable,
                        found,
                        member_type,
                        None,
                    )?
                }
                // associated members use their selected value type
                None => {
                    let Some(found) = self.body().member_read_type(&lookup)? else {
                        if member.is_optional || member.has_default {
                            continue;
                        }

                        return Ok(Verdict::Fails);
                    };

                    self.constrain_type(origin, cause, Relation::Assignable, found, member_type)?
                }
            };

            verdict = verdict.and(member_decision);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // prove each direct signature from the source
        let signatures =
            self.relate_interface_signatures(origin, cause, relation, source, &requirements)?;
        verdict = verdict.and(signatures);
        if verdict == Verdict::Fails {
            return Ok(Verdict::Fails);
        }

        // preserve each inherited interface's structural or nominal identity
        for inherited in requirements.inherited {
            verdict =
                verdict.and(self.constrain_type(origin, cause, relation, source, inherited.ty)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Relate one source against an interface's direct signatures.
    pub(in crate::sema) fn relate_interface_signatures(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        requirements: &InterfaceRequirements,
    ) -> CompilerResult<Verdict> {
        // prove each required call signature from the source
        let mut verdict = Verdict::Holds;
        for signature in &requirements.call_signatures {
            let satisfied = self.relate_signature_requirement(
                origin,
                cause,
                source,
                signature.ty,
                SignatureFamily::Call,
            )?;
            verdict = verdict.and(satisfied);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // prove each required construct signature from the source
        for signature in &requirements.construct_signatures {
            let satisfied = self.relate_signature_requirement(
                origin,
                cause,
                source,
                signature.ty,
                SignatureFamily::Construct,
            )?;
            verdict = verdict.and(satisfied);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // prove each required index signature from the source
        for signature in &requirements.index_signatures {
            let satisfied =
                self.relate_index_signature(origin, relation, source, &signature.signature)?;
            verdict = verdict.and(satisfied);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Relate one source against a required signature.
    fn relate_signature_requirement(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        required: dir::GlobalTypeId,
        family: SignatureFamily,
    ) -> CompilerResult<Verdict> {
        // relate function-typed sources through their own signature
        let is_function = match self.ty(source)? {
            dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_) => true,
            dir::Type::Application(callable) => self.is_function_language_item(callable.symbol)?,
            _ => false,
        };
        if is_function {
            return match family {
                SignatureFamily::Call => {
                    self.relate_method(origin, cause, Relation::Assignable, source, required, None)
                }
                SignatureFamily::Construct => Ok(Verdict::Fails),
            };
        }

        // relate other sources through their apparent signatures
        let constraint = match self.ty(source)? {
            dir::Type::Dynamic(dynamic) => dynamic.constraint,
            dir::Type::Application(_) => source,
            _ => return Ok(Verdict::Fails),
        };

        // accept one apparent signature satisfying the requirement
        let signatures = self.apparent_signatures(constraint, family)?;
        let mut verdict = Verdict::Fails;
        for signature in signatures {
            verdict = verdict.or(self.relate_method(
                origin,
                cause,
                Relation::Assignable,
                signature.ty,
                required,
                None,
            )?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }

    /// Return requirements imposed by one interface application.
    pub(in crate::sema) fn interface_requirements(
        &mut self,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<InterfaceRequirements> {
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
        self.collect_interface_members(symbol, interface, receiver, &mut members)?;

        // apply the interface arguments to each declared signature
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

        // apply the interface arguments to each inherited clause
        let inherited = self.apply_interface_heritage(interface, receiver, inherited)?;

        Ok(InterfaceRequirements {
            members,
            inherited,
            index_signatures,
            call_signatures,
            construct_signatures,
        })
    }

    /// Return the apparent signatures of one interface constraint.
    pub(in crate::sema) fn apparent_signatures(
        &mut self,
        constraint: dir::GlobalTypeId,
        family: SignatureFamily,
    ) -> CompilerResult<SmallVec<[InterfaceSignature; 2]>> {
        // read signatures from an applied interface only
        if self.nominal_application_maybe(constraint)?.is_none() {
            return Ok(SmallVec::new());
        }

        // read the constraint's own signatures of this family
        let requirements = self.interface_requirements(constraint, constraint)?;
        let mut signatures = match family {
            SignatureFamily::Call => requirements.call_signatures.clone(),
            SignatureFamily::Construct => requirements.construct_signatures.clone(),
        };

        // collect apparent signatures from inherited interfaces
        for heritage in &requirements.inherited {
            let nested = self.apparent_signatures(heritage.ty, family)?;
            signatures.extend(nested);
        }

        Ok(signatures)
    }

    /// Return whether one symbol declares the callable value language item.
    pub(in crate::sema) fn is_function_language_item(
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
    ) -> CompilerResult<()> {
        // read the interface declaration owning these members
        let Some(dir::Definition::Interface(definition)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("interface member source {symbol:?} is not an interface"),
            });
        };

        let members = definition.members.clone();

        // collect direct interface members with applied arguments
        for member in members {
            let Ok(role) = MemberRole::try_from(&member) else {
                continue;
            };
            let Some(key) = member.key() else {
                continue;
            };
            let symbol = member.symbol().ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "named interface member has no symbol: {:?}",
                    member.source()
                ),
            })?;

            // bind associated types by constraint and treat a written value as a default
            let (declared, has_default) = match &member {
                dir::DefinitionMember::AssociatedType(associated) => {
                    (associated.constraint, associated.value.is_some())
                }
                _ => (self.definition_member_type(&member)?, member.is_default()),
            };

            // apply the interface arguments to the declared type
            let ty = match declared {
                Some(ty) => Some(self.instantiate_interface_type(ty, interface, receiver)?),
                None => None,
            };
            let (is_optional, is_readonly) = match &member {
                dir::DefinitionMember::Field(field) => (field.is_optional, field.is_readonly),
                _ => (false, false),
            };

            required.push(InterfaceMember {
                symbol,
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

        Ok(())
    }

    /// Collect one interface application's instance properties, inherited first.
    pub(in crate::sema) fn interface_instance_fields(
        &mut self,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::TypeProperty>>> {
        // read instance properties from an interface application only
        let (_, instance) = self.nominal_application(interface)?;
        if !matches!(
            self.symbol_kind(instance.symbol)?,
            dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface
        ) {
            return Ok(None);
        }

        // collect inherited interface fields first
        let requirements = self.interface_requirements(interface, receiver)?;
        let mut fields = Vec::<dir::TypeProperty>::new();
        for inherited in requirements.inherited {
            let Some(nested) = self.interface_instance_fields(inherited.ty, receiver)? else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "inherited interface application {:?} is not an interface",
                        inherited.ty,
                    ),
                });
            };

            // compose accessor operations inherited through separate requirements
            for property in nested {
                if !fields.iter_mut().any(|field| field.compose(property)) {
                    fields.push(property);
                }
            }
        }

        // add the interface's own instance members over the inherited ones
        for member in requirements.members {
            if member.space != dir::MemberSpace::Instance {
                continue;
            }

            let Some(ty) = member.ty else {
                continue;
            };
            let ty = self.shallow_resolve(ty)?;
            let Some(access) = self.property_access(member.role, ty, member.is_readonly)? else {
                continue;
            };
            let property = dir::TypeProperty {
                key: member.key,
                access,
                is_optional: member.is_optional || member.has_default,
            };

            // merge complementary getter and setter operations
            if !fields.iter_mut().any(|field| field.compose(property)) {
                fields.push(property);
            }
        }

        Ok(Some(fields))
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
    pub(in crate::sema) fn interface_members(
        &mut self,
        interface: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<SmallVec<[InterfaceMember; 2]>> {
        // select the interface's own members by space and key
        let requirements = self.interface_requirements(interface, receiver)?;
        let mut members = SmallVec::new();
        for member in requirements.members {
            if member.space == space && member.key == key {
                members.push(member);
            }
        }

        // search inherited interfaces when the named interface misses
        if members.is_empty() {
            for inherited in requirements.inherited {
                let nested = self.interface_members(inherited.ty, receiver, space, key)?;
                members.extend(nested);
                if !members.is_empty() {
                    break;
                }
            }
        }

        Ok(members)
    }
}
