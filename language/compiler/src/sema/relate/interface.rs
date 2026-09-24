use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    CandidateOutcome, Cause, CauseId, CauseKind, CheckState, GenericParameterId, MemberRole,
    Origin, PropertySource, Relation, TypeSubstitution, Verdict,
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

/// The outcome of matching one declaration's implemented interfaces against an ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum ImplementedInterface {
    /// One declared application matched, applied to the ask.
    Matched(dir::GlobalTypeId),
    /// No declared application matched.
    Unmatched,
    /// Several declared applications could match the open ask.
    Undecided,
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
        // relate the subject as given, a form over an open value through the value it holds
        let reduced_source = self.normalize(origin, source)?;
        let interface = self.nominal_application(target)?.1.symbol;
        let held = self.strip_form(origin, reduced_source)?;
        let source = match self.ty(held)? {
            dir::Type::Parameter(_) | dir::Type::Erased(_) | dir::Type::This
                if held != reduced_source
                    && self.requirements_accept(origin, interface, reduced_source, held)? =>
            {
                held
            }
            _ => reduced_source,
        };
        let target = match self.type_flags(target)?.has_this() {
            true => {
                let substitution = TypeSubstitution::default().with_receiver(source);

                self.substitute_type(target, &substitution)?
            }
            false => target,
        };

        // decide an open numeric literal by its default type
        let (_, target_instance) = self.nominal_application(target)?;
        if let Some(variable) = self.root_variable(source)?
            && let Some(fallback) = self.root_kind(variable)?.fallback()
        {
            let fallback = self.intern_type(fallback)?;
            if self.decide_relation(origin, Relation::Subtype, fallback, target)? == Verdict::Fails
            {
                return Ok(Verdict::Fails);
            }

            return Ok(Verdict::Ambiguous);
        }

        // decide the interface by a where clause on this exact subject, like `where ^T: Unpin`
        for predicate in self.assumed_predicates(origin)? {
            if predicate.relation != dir::WhereRelation::Satisfies {
                continue;
            }
            let left = self.shallow_resolve(predicate.left)?;
            if !self.type_flags(left)?.has_parameter()
                || self.ty(predicate.right)?.symbol() != Some(target_instance.symbol)
            {
                continue;
            }
            let matches_subject = left == reduced_source
                || left == source
                || self.decide_relation(origin, Relation::Equal, left, reduced_source)?
                    == Verdict::Holds;
            if matches_subject
                && self.decide_relation(origin, Relation::Subtype, predicate.right, target)?
                    == Verdict::Holds
            {
                return Ok(Verdict::Holds);
            }
        }

        // read the target interface declaration
        let is_nominal = match self.definition(target_instance.symbol)?.as_deref() {
            Some(dir::Definition::Interface(interface)) => interface.is_nominal,
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("interface relation target {target:?} is not an interface"),
                });
            }
        };

        // flow a nullish try residual into every return type that includes it
        if matches!(
            self.language_item(target_instance.symbol)?,
            Some(dir::LanguageItem::FromResidual)
        ) && let [residual] = *self.type_ids(target.module_id, target_instance.arguments)?
            && self.is_nullish_type(origin, residual)?
        {
            return self.decide_relation(origin, Relation::Subtype, residual, source);
        }

        // answer a rigid parameter through its bounds ahead of the implementations
        if let dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) = self.ty(source)?
            && self
                .relate_parameter_bounds(origin, cause, relation, parameter, target)?
                .holds()
        {
            return Ok(Verdict::Holds);
        }

        // classify the interfaces the compiler decides itself
        let auto_interface = self
            .language_item(target_instance.symbol)?
            .and_then(dir::AutoInterface::from_language_item)
            .filter(|interface| interface.has_builtin_implementation());

        // find the target interface in the subject's heritage closure
        let applications = self.heritage_applications(origin, source, target_instance.symbol)?;

        // compare the selected interface arguments by their declared variance
        if !applications.is_empty() {
            let target_arguments: SmallVec<[_; 8]> = self
                .type_ids(target.module_id, target_instance.arguments)?
                .into();
            let form = self.default_variance_form(target_instance.symbol)?;
            if applications.len() > 1 && !self.collect_open_variables([target])?.is_empty() {
                return Ok(Verdict::Ambiguous);
            }
            for (application_module, application) in &applications {
                let source_arguments: SmallVec<[_; 8]> = self
                    .type_ids(*application_module, application.arguments)?
                    .into();
                let relate = |state: &mut Self| {
                    state.relate_type_arguments(
                        origin,
                        cause,
                        target_instance.symbol,
                        form,
                        relation,
                        &source_arguments,
                        &target_arguments,
                    )
                };
                if applications.len() == 1 {
                    return relate(self);
                }
                let verdict = self.decide_candidate(|state| {
                    Ok(match relate(state)? {
                        Verdict::Holds => CandidateOutcome::Accepted(()),
                        _ => CandidateOutcome::Rejected(()),
                    })
                })?;
                if verdict == Verdict::Holds {
                    return relate(self);
                }
            }

            return Ok(Verdict::Fails);
        }

        // select a visible extension implementation
        let implementation = self.decide_extension_implementation(
            origin,
            target.module_id,
            reduced_source,
            &target_instance,
            None,
        )?;
        let implemented = implementation.verdict;
        if implemented == Verdict::Holds {
            return Ok(Verdict::Holds);
        }

        // decide intrinsic conformance on stored values
        let decides_intrinsically = auto_interface == Some(dir::AutoInterface::DynamicSafe)
            || !self.is_nominal_interface(source)?;
        if decides_intrinsically {
            // use the intrinsic conformance the compiler decides itself
            if let Some(auto_interface) = auto_interface {
                let decided = self.decide_intrinsic_interface(
                    origin,
                    reduced_source,
                    target,
                    auto_interface,
                )?;

                return Ok(decided.join_undecided(implemented));
            }

            // conform through the heritage of an intrinsically implemented interface
            if self.inherits_intrinsic_interface(origin, reduced_source, target_instance.symbol)? {
                return Ok(Verdict::Holds);
            }
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

    /// Return whether one type applies a nominal interface.
    fn is_nominal_interface(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let Some((_, instance)) = self.nominal_application_maybe(ty)? else {
            return Ok(false);
        };

        Ok(matches!(
            self.definition(instance.symbol)?.as_deref(),
            Some(dir::Definition::Interface(interface)) if interface.is_nominal
        ))
    }

    /// Return whether one type implements an interface through an intrinsically implemented one.
    pub(in crate::sema) fn inherits_intrinsic_interface(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        for interface in self.intrinsic_inheritors(origin, target)? {
            let ty = self.language_type(dir::LanguageItem::from(interface), &[])?;
            if self.decide_intrinsic_interface(origin, source, ty, interface)? == Verdict::Holds {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the intrinsically implemented interfaces extending one interface.
    fn intrinsic_inheritors(
        &mut self,
        origin: Origin,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::AutoInterface; 2]>> {
        // scan the builtin-implemented interfaces
        let mut inheritors = SmallVec::new();
        for interface in dir::AutoInterface::ALL {
            if !interface.has_builtin_implementation() {
                continue;
            }
            let item = dir::LanguageItem::from(interface);
            if self.language_symbol(item)? == target {
                continue;
            }
            let ty = self.language_type(item, &[])?;
            if self.declared_conformance(origin, ty, ty, target)?.is_some() {
                inheritors.push(interface);
            }
        }
        Ok(inheritors)
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
        assumed: Option<&TypeSubstitution>,
    ) -> CompilerResult<Verdict> {
        if role.is_callable() {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

            self.relate_method(origin, cause, relation, source, target, receiver, assumed)
        } else {
            self.decide_relation(origin, relation, source, target)
        }
    }

    /// Select one declared implementation of a requested interface.
    pub(in crate::sema) fn match_implemented_interface(
        &mut self,
        origin: Origin,
        interface_module: ModuleId,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        interfaces: &[dir::GlobalTypeId],
        interface: &dir::GenericApplication,
        decides_open: bool,
    ) -> CompilerResult<ImplementedInterface> {
        // compare each declared implemented interface against the ask
        let interface_arguments: SmallVec<[_; 8]> =
            self.type_ids(interface_module, interface.arguments)?.into();
        let arguments = self.intern_type_ids(&interface_arguments)?;
        let interface_type = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol: interface.symbol,
            arguments,
        }))?;

        // leave an open ask undecided while several declared applications could match it
        if decides_open && !self.collect_open_variables([interface_type])?.is_empty() {
            let mut viable = 0;
            for implemented in interfaces {
                let declared = self.shallow_resolve(*implemented)?;
                if self.ty(declared)?.symbol() != Some(interface.symbol) {
                    continue;
                }
                let matched = self.decide_candidate(|state| {
                    let mut scratch = substitution.clone();
                    let matched = state.substitute_type(declared, &scratch)?;
                    let bound = state.extend_generic_substitution(
                        origin,
                        parameters,
                        &mut scratch,
                        &[(matched, interface_type)],
                    )?;
                    Ok(match bound {
                        true => CandidateOutcome::Accepted(()),
                        false => CandidateOutcome::Rejected(()),
                    })
                })?;
                if matched != Verdict::Fails {
                    viable += 1;
                }
            }
            if viable > 1 {
                return Ok(ImplementedInterface::Undecided);
            }
        }

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
            let bound = self.extend_generic_substitution(
                origin,
                parameters,
                &mut scratch,
                &[(matched, interface_type)],
            )?;
            if !bound {
                continue;
            }

            // apply the fresh bindings to the declared interface
            let implemented = self.substitute_type(declared, &scratch)?;
            let implemented = self.shallow_resolve(implemented)?;

            // unify the header naming the requested interface with the requested arguments
            let (implemented_module, implemented_instance) =
                self.nominal_application(implemented)?;
            if implemented_instance.symbol != interface.symbol {
                continue;
            }
            let arguments: SmallVec<[_; 8]> = self
                .type_ids(implemented_module, implemented_instance.arguments)?
                .into();
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            let matched = self.match_header_arguments(
                origin,
                cause,
                interface.symbol,
                &arguments,
                &interface_arguments,
            )?;

            // leave an open ask undecided while its variables keep this header possible
            if matched == Verdict::Ambiguous {
                return Ok(ImplementedInterface::Undecided);
            }

            // commit the bindings of the first implementation that matched
            if matched == Verdict::Holds {
                *substitution = scratch;
                let arguments = self.intern_type_ids(&arguments)?;
                let matched =
                    self.intern_type(dir::Type::Application(dir::GenericApplication {
                        symbol: interface.symbol,
                        arguments,
                    }))?;

                return Ok(ImplementedInterface::Matched(matched));
            }
        }

        Ok(ImplementedInterface::Unmatched)
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
        let (mut base, mut bindings) = self.refinements(implementation)?;
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
        let declared = self.definition(application.symbol)?;
        let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "implementation target has no interface definition: {:?}",
                    application.symbol,
                ),
            });
        };

        // read the members the interface declares
        let interface_members = definition.members.clone();

        // select written bindings, declared values, then interface defaults
        for member in &interface_members {
            let Some(key) = member.key().filter(|_| member.is_associated()) else {
                continue;
            };
            let written = bindings
                .iter()
                .find(|(candidate, _)| *candidate == key)
                .map(|(_, value)| *value);
            let mut declared = None;
            for candidate in members {
                if candidate.kind() == member.kind() && candidate.key() == Some(key) {
                    declared = self.associated_member_value(candidate)?;
                    break;
                }
            }
            let declared = declared
                .map(|(_, value)| self.substitute_type(value, substitution))
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

            // record the selected associated member
            let default = self
                .associated_member_value(member)?
                .map(|(_, value)| value);
            let Some(value) = written.or(declared).or(default) else {
                continue;
            };
            match bindings.iter_mut().find(|(candidate, _)| *candidate == key) {
                Some((_, current)) => *current = value,
                None => bindings.push((key, value)),
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
            let Some(key) = member.key().filter(|_| member.is_associated()) else {
                continue;
            };
            let Some(constraint) = self.associated_member_constraint(base, key)? else {
                continue;
            };
            let Some((_, value)) = bindings.iter().find(|(candidate, _)| *candidate == key) else {
                continue;
            };

            let constraint =
                self.instantiate_interface_type(constraint, implementation, receiver)?;
            if self.decide_relation(origin, Relation::Subtype, *value, constraint)?
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
        // read what the applied interface requires of the source
        let requirements = self.interface_requirements(target, source)?;
        let module = origin.module();

        // require each member from the source
        let mut verdict = Verdict::Holds;
        for member in &requirements.members {
            let subject = self.member_subject(origin, source, source, member.space)?;
            let lookup = self.lookup_member(origin, module, subject, member.key)?;

            // require presence when an associated type stays abstract
            let Some(member_type) = member.ty else {
                if !lookup.is_empty() || member.has_default || member.is_optional {
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
                    let Some(found) = self.member_binding(member.key, &lookup)? else {
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
                    let Some(relations) = Self::shape_property_relations(
                        Relation::Storable,
                        PropertySource::Stored,
                        &source,
                        &target,
                    ) else {
                        return Ok(Verdict::Fails);
                    };

                    self.relate_shape_fields(origin, cause, &relations)?
                }
                // methods compare callable signatures without their receivers
                Some(_) => {
                    let Some(found) = self.member_read_type(&lookup)? else {
                        if member.is_optional || member.has_default {
                            continue;
                        }

                        return Ok(Verdict::Fails);
                    };

                    self.relate_method(
                        origin,
                        cause,
                        Relation::Storable,
                        found,
                        member_type,
                        Some(source),
                        Some(&TypeSubstitution::default().with_receiver(source)),
                    )?
                }
                // associated members use their selected value type
                None => {
                    let Some(found) = self.member_read_type(&lookup)? else {
                        if member.is_optional || member.has_default {
                            continue;
                        }

                        return Ok(Verdict::Fails);
                    };

                    self.constrain_type(origin, cause, Relation::Storable, found, member_type)?
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
            let required = self.relate_signature_requirement(
                origin,
                cause,
                source,
                signature.ty,
                SignatureFamily::Call,
            )?;
            verdict = verdict.and(required);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // prove each required construct signature from the source
        for signature in &requirements.construct_signatures {
            let required = self.relate_signature_requirement(
                origin,
                cause,
                source,
                signature.ty,
                SignatureFamily::Construct,
            )?;
            verdict = verdict.and(required);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // prove each required index signature from the source
        for signature in &requirements.index_signatures {
            let required =
                self.relate_index_signature(origin, cause, relation, source, &signature.signature)?;
            verdict = verdict.and(required);
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
                SignatureFamily::Call => self.relate_method(
                    origin,
                    cause,
                    Relation::Storable,
                    source,
                    required,
                    None,
                    None,
                ),
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
                Relation::Storable,
                signature.ty,
                required,
                None,
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
        let declared = self.definition(symbol)?;
        let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
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

        // read whether the item names a callable representation
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
        let declared = self.definition(symbol)?;
        let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
            return Err(CompilerError::Internal {
                message: format!("interface member source {symbol:?} is not an interface"),
            });
        };

        // read the members the declaration holds
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

            // bind associated types by constraint
            let declared = match &member {
                dir::DefinitionMember::AssociatedType(associated) => associated.constraint,
                _ => self.definition_member_type(&member)?,
            };
            let has_default = self.definition_member_has_default(&member)?;

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
                implementer: receiver,
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
