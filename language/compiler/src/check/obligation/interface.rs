use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CheckState, InterfaceConformanceObligation, InterfaceMember, MemberCandidate, MemberLookup,
    ObligationCheck, ObligationFailure, Origin, Relation, TypeSubstitution,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check one declaration's interface conformance.
    pub(in crate::check) fn check_interface_conformance(
        &mut self,
        origin: Origin,
        obligation: &InterfaceConformanceObligation,
    ) -> CompilerResult<ObligationCheck> {
        let symbol = obligation.symbol;
        let (members, implementations, extension_target) = {
            let Some(definition) = self.definition(symbol)? else {
                return Err(CompilerError::Internal {
                    message: format!("interface conformance has no definition: {symbol:?}"),
                });
            };
            let extension_target = match definition {
                dir::Definition::Extension(extension) => Some(extension.target.r#type()),
                dir::Definition::Struct(_)
                | dir::Definition::Class(_)
                | dir::Definition::Enum(_) => None,
                dir::Definition::TypeAlias(_)
                | dir::Definition::Interface(_)
                | dir::Definition::Newtype(_) => return Ok(ObligationCheck::Holds),
            };

            (
                definition.members().to_vec(),
                definition.implementations().to_vec(),
                extension_target,
            )
        };
        if implementations.is_empty() {
            return Ok(ObligationCheck::Holds);
        }

        // select the implementing declaration's target
        let target = match extension_target {
            Some(target) => target,
            None => {
                let instance = self.declaration_instance(symbol)?;

                self.intern_type(dir::Type::Application(instance))?
            }
        };

        // recognize explicit unsafe assertions on extension implementations
        let source = obligation.source;
        let is_unsafe_extension = extension_target.is_some()
            && self
                .module(source.module_id)
                .decorators
                .applications_for_owner(source)
                .any(|application| {
                    application.resolution.target.language_item() == Some(dir::LanguageItem::Unsafe)
                });
        let mut failures = Vec::new();

        // prove each declared interface
        for heritage in implementations {
            let conforms = self.decide_declared_conformance(
                origin,
                heritage.ty,
                target,
                &members,
                is_unsafe_extension,
            )?;
            if !conforms {
                failures.push(ObligationFailure::InterfaceNotImplemented {
                    source: heritage.source,
                    ty: target,
                    interface: heritage.ty,
                });
            }
        }

        Ok(ObligationCheck::from_failures(failures))
    }

    /// Decide whether declared members conform to one applied interface.
    fn decide_declared_conformance(
        &mut self,
        origin: Origin,
        interface: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        is_unsafe_extension: bool,
    ) -> CompilerResult<bool> {
        // validate compiler-known markers through their compiler rule
        let (_, application) = self.nominal_application(interface)?;
        let auto_interface = self
            .language_item(application.symbol)?
            .and_then(dir::AutoInterface::from_language_item)
            .filter(|interface| interface.is_marker());
        if let Some(interface) = auto_interface {
            if is_unsafe_extension && interface.permits_unsafe_implementation() {
                return Ok(true);
            }

            return self.satisfies_auto_interface(origin, target, interface);
        }

        // resolve associated projections through the declared implementation
        let substitution = TypeSubstitution::default().with_receiver(target);
        let Some(interface) = self.instantiate_interface_implementation(
            origin,
            interface,
            target,
            members,
            &substitution,
        )?
        else {
            return Ok(false);
        };
        let requirements = self.interface_requirements(interface, target)?;

        // match each named requirement against declared or inherent members
        for requirement in &requirements.members {
            let mut candidates = SmallVec::<[_; 2]>::new();
            for member in members {
                if member.space() != requirement.space || member.key() != Some(requirement.key) {
                    continue;
                }
                let member = match self.body().declared_member(member)? {
                    Some(member) => member,
                    None => continue,
                };
                candidates.push(member);
            }
            if candidates.is_empty() {
                // use target members that do not come from this declaration
                let inherent = self.inherent_member_candidates(origin, target, requirement)?;
                if inherent.is_empty() {
                    if requirement.has_default || requirement.is_optional {
                        continue;
                    }

                    return Ok(false);
                }
                let Some(required) = requirement.ty else {
                    continue;
                };

                let mut is_satisfied = false;
                for candidate in &inherent {
                    let found = candidate.callable.unwrap_or(candidate.access_type);
                    let decision = self.decide_member_relation(
                        origin,
                        Relation::Assignable,
                        requirement.role,
                        found,
                        required,
                        substitution.receiver,
                    )?;
                    if decision {
                        is_satisfied = true;
                        break;
                    }
                }
                if !is_satisfied {
                    return Ok(false);
                }

                continue;
            }

            // accept an abstract associated requirement by presence
            let Some(required) = requirement.ty else {
                continue;
            };

            let mut is_satisfied = false;
            for candidate in candidates {
                let Some(found) = candidate.ty else {
                    continue;
                };
                let found = self.substitute_type(found, &substitution)?;
                let decision = self.decide_member_relation(
                    origin,
                    Relation::Assignable,
                    requirement.role,
                    found,
                    required,
                    substitution.receiver,
                )?;
                if decision {
                    is_satisfied = true;
                    break;
                }
            }
            if !is_satisfied {
                return Ok(false);
            }
        }

        // validate call, construct, and index requirements from the target
        let signatures =
            self.decide_interface_signatures(origin, Relation::Satisfies, target, &requirements)?;
        if !signatures {
            return Ok(false);
        }

        // validate inherited interfaces through the same declaration
        for inherited in &requirements.inherited {
            let conforms = self.decide_declared_conformance(
                origin,
                inherited.ty,
                target,
                members,
                is_unsafe_extension,
            )?;
            if !conforms {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return target members matching one interface requirement.
    fn inherent_member_candidates(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        requirement: &InterfaceMember,
    ) -> CompilerResult<Vec<MemberCandidate>> {
        let lookup = self.body().lookup_inherent_member(
            origin,
            origin.module(),
            target,
            requirement.space,
            requirement.key,
        )?;
        let candidates = match lookup {
            MemberLookup::Missing => Vec::new(),
            MemberLookup::Found(candidates) => candidates,
            lookup @ MemberLookup::Intersection(_) => {
                lookup
                    .into_candidates()
                    .ok_or_else(|| CompilerError::Internal {
                        message: "nominal implementation has a structural member".into(),
                    })?
            }
            MemberLookup::Field(_) | MemberLookup::Union(_) => {
                return Err(CompilerError::Internal {
                    message: "nominal implementation has a structural member".into(),
                });
            }
        };

        Ok(candidates)
    }

    /// Check whether one type satisfies one compiler-known auto interface.
    pub(in crate::check) fn check_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<ObligationCheck> {
        let ty = self.reduce_type_head(origin, ty)?;

        // hold without checking once an operand already reported an error
        if self.any_error_operand(&[ty])? {
            return Ok(ObligationCheck::holds());
        }
        if self.satisfies_auto_interface(origin, ty, interface)? {
            return Ok(ObligationCheck::holds());
        }

        let source = self.origin_source(origin)?;
        let failure = ObligationFailure::AutoInterfaceNotSatisfied {
            source,
            ty,
            interface,
        };

        Ok(ObligationCheck::fail(failure))
    }
}
