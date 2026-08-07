use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, InterfaceConformanceObligation, InterfaceMember, MemberCandidate,
    MemberLookup, ObligationCheck, ObligationFailure, Origin, Relation, TypeSubstitution, answer,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check one declaration's interface conformance.
    pub(in crate::check) fn check_interface_conformance(
        &mut self,
        origin: Origin,
        obligation: &InterfaceConformanceObligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let symbol = obligation.symbol;
        let Some(definition) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("interface conformance has no definition: {symbol:?}"),
            });
        };
        let members = definition.members().to_vec();
        let implementations = definition
            .implementations()
            .iter()
            .map(|conformance| (conformance.source, conformance.interface))
            .collect::<SmallVec<[_; 2]>>();
        let extension_target = match definition {
            dir::Definition::Extension(definition) => Some(definition.target.r#type()),
            _ => None,
        };
        if implementations.is_empty() {
            return Err(CompilerError::Internal {
                message: format!("interface conformance has no declared interfaces: {symbol:?}"),
            });
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
        let mut member_selections = Vec::with_capacity(implementations.len());

        // prove each declared interface
        for (source, interface) in implementations {
            let selected_members = answer!(self.select_declared_conformance(
                origin,
                interface,
                target,
                &members,
                is_unsafe_extension,
            )?);
            match selected_members {
                Some(members) => member_selections.push(members),
                None => {
                    failures.push(ObligationFailure::InterfaceNotImplemented {
                        source,
                        ty: target,
                        interface,
                    });
                    member_selections.push(Vec::new());
                }
            }
        }

        // publish selected members after every interface settles
        let definition = self
            .definition_mut(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("interface conformance has no mutable definition: {symbol:?}"),
            })?;
        let conformances =
            definition
                .implementations_mut()
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("definition {symbol:?} cannot implement interfaces"),
                })?;
        // require the declaration to retain the queued implementation count
        if conformances.len() != member_selections.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "definition {symbol:?} has {} conformances, expected {}",
                    conformances.len(),
                    member_selections.len()
                ),
            });
        }
        for (conformance, members) in conformances.iter_mut().zip(member_selections) {
            conformance.members = members;
        }

        Ok(Answer::Ready(ObligationCheck::from_failures(failures)))
    }

    /// Select the members satisfying one applied interface.
    fn select_declared_conformance(
        &mut self,
        origin: Origin,
        interface: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        is_unsafe_extension: bool,
    ) -> CompilerResult<Answer<Option<Vec<dir::MemberConformance>>>> {
        // validate compiler-known markers through their compiler rule
        let (_, application) = self.nominal_application(interface)?;
        let auto_interface = self
            .language_item(application.symbol)?
            .and_then(dir::AutoInterface::from_language_item)
            .filter(|interface| interface.is_marker());
        if let Some(interface) = auto_interface {
            if is_unsafe_extension && interface.permits_unsafe_implementation() {
                return Ok(Answer::Ready(Some(Vec::new())));
            }

            let conforms = answer!(self.satisfies_auto_interface(origin, target, interface)?);

            return Ok(Answer::Ready(conforms.then(Vec::new)));
        }

        // resolve associated projections through the declared implementation
        let substitution = TypeSubstitution::default().with_receiver(target);
        let Some(interface) = answer!(self.instantiate_interface_implementation(
            origin,
            interface,
            target,
            members,
            &substitution,
        )?) else {
            return Ok(Answer::Ready(None));
        };
        let requirements = answer!(self.interface_requirements(interface, target)?);
        let mut selected = Vec::new();

        // match each named requirement against declared or inherent members
        for requirement in &requirements.members {
            let mut candidates =
                SmallVec::<[(dir::GlobalSymbolId, Option<dir::GlobalTypeId>); 2]>::new();
            for member in members {
                if member.space() != requirement.space || member.key() != Some(requirement.key) {
                    continue;
                }
                let member = match self.body().declared_member(member)? {
                    Answer::Ready(Some(member)) => member,
                    Answer::Ready(None) => continue,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
                let ty = member
                    .ty
                    .map(|ty| self.substitute_type(ty, &substitution))
                    .transpose()?;
                candidates.push((member.symbol, ty));
            }

            // include matching members inherited by the target declaration
            if candidates.is_empty() {
                // use target members that do not come from this declaration
                let inherent =
                    answer!(self.inherent_member_candidates(origin, target, requirement)?);
                for candidate in inherent {
                    let ty = candidate.callable.unwrap_or(candidate.access_type);
                    candidates.push((candidate.symbol, Some(ty)));
                }
            }

            // resolve requirements with no implementation candidate
            if candidates.is_empty() {
                if requirement.has_default {
                    selected.push(dir::MemberConformance {
                        member: requirement.symbol,
                        requirement: requirement.symbol,
                    });
                } else if !requirement.is_optional {
                    return Ok(Answer::Ready(None));
                }

                continue;
            }

            // match typed requirements and accept abstract requirements by presence
            let member = if let Some(required) = requirement.ty {
                let mut selected = None;
                for (symbol, found) in candidates {
                    let Some(found) = found else {
                        continue;
                    };
                    let decision = self.decide_member_relation(
                        origin,
                        Relation::Assignable,
                        requirement.role,
                        found,
                        required,
                        substitution.receiver,
                    )?;
                    if answer!(decision) {
                        selected = Some(symbol);
                        break;
                    }
                }
                let Some(member) = selected else {
                    return Ok(Answer::Ready(None));
                };

                member
            } else {
                let (member, _) = candidates.remove(0);

                member
            };
            selected.push(dir::MemberConformance {
                member,
                requirement: requirement.symbol,
            });
        }

        // validate call, construct, and index requirements from the target
        let signatures = answer!(self.decide_interface_signatures(
            origin,
            Relation::Satisfies,
            target,
            &requirements,
        )?);
        if !signatures {
            return Ok(Answer::Ready(None));
        }

        // validate inherited interfaces through the same declaration
        for inherited in &requirements.inherited {
            let inherited = answer!(self.select_declared_conformance(
                origin,
                inherited.ty,
                target,
                members,
                is_unsafe_extension,
            )?);
            let Some(inherited) = inherited else {
                return Ok(Answer::Ready(None));
            };
            selected.extend(inherited);
        }

        Ok(Answer::Ready(Some(selected)))
    }

    /// Return target members matching one interface requirement.
    fn inherent_member_candidates(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        requirement: &InterfaceMember,
    ) -> CompilerResult<Answer<Vec<MemberCandidate>>> {
        let lookup = answer!(self.body().lookup_inherent_member(
            origin,
            origin.module(),
            target,
            requirement.space,
            requirement.key,
        )?);
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

        Ok(Answer::Ready(candidates))
    }

    /// Check whether one type satisfies one compiler-known auto interface.
    pub(in crate::check) fn check_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        // hold without checking once an operand already reported an error
        if self.any_error_operand(&[ty])? {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }
        if answer!(self.satisfies_auto_interface(origin, ty, interface)?) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let source = self.origin_source(origin)?;
        let failure = ObligationFailure::AutoInterfaceNotSatisfied {
            source,
            ty,
            interface,
        };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }
}
