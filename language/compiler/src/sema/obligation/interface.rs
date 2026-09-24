use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseKind, CheckState, InterfaceConformanceObligation, InterfaceMember, MemberCandidate,
    ObligationCheck, ObligationFailure, Origin, Relation, TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check one declaration's interface conformance.
    pub(in crate::sema) fn check_interface_conformance(
        &mut self,
        origin: Origin,
        obligation: &InterfaceConformanceObligation,
    ) -> CompilerResult<ObligationCheck> {
        // read the declaration this obligation proves conformance for
        let symbol = obligation.symbol;
        let Some(definition) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("interface conformance has no definition: {symbol:?}"),
            });
        };

        // extensions implement for their target, nominals for themselves
        let extension_target = match &*definition {
            dir::Definition::Extension(extension) => Some(extension.target.r#type()),
            dir::Definition::Struct(_) | dir::Definition::Class(_) | dir::Definition::Enum(_) => {
                None
            }
            dir::Definition::TypeAlias(_)
            | dir::Definition::Interface(_)
            | dir::Definition::Newtype(_) => return Ok(ObligationCheck::Holds),
        };

        // read the declared members and the interfaces they must satisfy
        let members = definition.members();
        let implementations = definition
            .implementations()
            .map(|conformance| (conformance.source, conformance.interface))
            .collect::<SmallVec<[_; 2]>>();
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
                .decorators_tail
                .applications_for_owner(source)
                .any(|application| {
                    application.resolution.target.language_item() == Some(dir::LanguageItem::Unsafe)
                });

        // collect the failures and the members selected per interface
        let mut failures = Vec::new();
        let mut member_selections = Vec::with_capacity(implementations.len());

        // prove each declared interface
        for (source, interface) in implementations {
            let selected_members = self.select_declared_conformance(
                origin,
                interface,
                target,
                members,
                is_unsafe_extension,
            )?;
            match selected_members {
                ConformanceSelection::Selected(members) => member_selections.push(members),
                ConformanceSelection::Missing => {
                    failures.push(ObligationFailure::InterfaceNotImplemented {
                        source,
                        ty: target,
                        interface,
                    });
                    member_selections.push(Vec::new());
                }
                // re-run the whole obligation once the variables solve
                ConformanceSelection::Undecided(stalls) => {
                    return Ok(ObligationCheck::Ambiguous(stalls));
                }
            }
        }

        // publish each interface's selected members on its `implements` clause
        let sources: Vec<_> = self
            .definition(symbol)?
            .map(|definition| {
                definition
                    .implementations()
                    .map(|conformance| conformance.source)
                    .collect()
            })
            .unwrap_or_default();
        if sources.len() != member_selections.len() {
            return Err(CompilerError::Internal {
                message: format!(
                    "definition {symbol:?} has {} conformances, expected {}",
                    sources.len(),
                    member_selections.len()
                ),
            });
        }
        for (source, members) in sources.into_iter().zip(member_selections) {
            self.module_mut(symbol.module_id)
                .members_tail
                .set_conformance_members(source, members);
        }

        Ok(ObligationCheck::from_failures(failures))
    }

    /// Select the members satisfying one applied interface.
    fn select_declared_conformance(
        &mut self,
        origin: Origin,
        interface: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        is_unsafe_extension: bool,
    ) -> CompilerResult<ConformanceSelection> {
        // refuse an interface written with an error
        if self.type_flags(interface)?.has_error() {
            return Ok(ConformanceSelection::Missing);
        }

        // validate compiler-known markers through their compiler rule
        let (_, application) = self.nominal_application(interface)?;
        let auto_interface = self
            .language_item(application.symbol)?
            .and_then(dir::AutoInterface::from_language_item)
            .filter(|interface| interface.has_builtin_implementation());
        if let Some(auto_interface) = auto_interface {
            // decide markers by their compiler rule alone
            if auto_interface.is_marker() {
                if is_unsafe_extension && auto_interface.permits_unsafe_implementation() {
                    return Ok(ConformanceSelection::Selected(Vec::new()));
                }

                return self.select_intrinsic_conformance(
                    origin,
                    interface,
                    target,
                    auto_interface,
                );
            }

            // select declared members before intrinsic derivation
            let declared = self.select_conformance_members(
                origin,
                interface,
                target,
                members,
                is_unsafe_extension,
            )?;
            if matches!(declared, ConformanceSelection::Missing) {
                return self.select_intrinsic_conformance(
                    origin,
                    interface,
                    target,
                    auto_interface,
                );
            }

            return Ok(declared);
        }

        self.select_conformance_members(origin, interface, target, members, is_unsafe_extension)
    }

    /// Map one intrinsic conformance verdict onto a member selection.
    fn select_intrinsic_conformance(
        &mut self,
        origin: Origin,
        interface: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        auto_interface: dir::AutoInterface,
    ) -> CompilerResult<ConformanceSelection> {
        match self.decide_intrinsic_interface(origin, target, interface, auto_interface)? {
            Verdict::Holds => Ok(ConformanceSelection::Selected(Vec::new())),
            Verdict::Fails => Ok(ConformanceSelection::Missing),
            // leave the conformance undecided while its variables stay open
            Verdict::Ambiguous => Ok(ConformanceSelection::Undecided(
                self.collect_open_variables([target, interface])?,
            )),
        }
    }

    /// Select the declared members satisfying one applied interface.
    fn select_conformance_members(
        &mut self,
        origin: Origin,
        interface: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        is_unsafe_extension: bool,
    ) -> CompilerResult<ConformanceSelection> {
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
            return Ok(ConformanceSelection::Missing);
        };

        // read the requirements and the instantiation of their bounds
        let mut requirements = self.interface_requirements(interface, target)?;
        let assumed = match self.ty(interface)? {
            dir::Type::Application(instance) => {
                self.instance_substitution(interface.module_id, &instance)?
            }
            _ => TypeSubstitution::default(),
        };

        // project interface-owner members through this implementation's refinements
        let mut base = interface;
        while let Some(refined) = self.refined_head(base)? {
            base = refined.base;
        }
        if base != interface
            && let dir::Type::Application(base_instance) = self.ty(base)?
        {
            let base_arguments = self.type_ids(base.module_id, base_instance.arguments)?;
            for index in 0..requirements.members.len() {
                let Some(ty) = requirements.members[index].ty else {
                    continue;
                };
                let plain = self.plain_applications_of(ty, base_instance.symbol, base_arguments)?;
                let mut rewritten = ty;
                for occurrence in plain {
                    rewritten = self.replace_type(rewritten, occurrence, interface)?;
                }
                requirements.members[index].ty = Some(rewritten);
            }
        }

        // match each named requirement against declared or inherent members
        let mut selected = Vec::new();
        let (_, bindings) = self.refinements(interface)?;
        for requirement in &requirements.members {
            // take a declared or refined associated member
            if requirement.role == dir::MemberRole::Associated {
                let declared = members
                    .iter()
                    .find(|member| member.is_associated_at(requirement.key))
                    .and_then(dir::DefinitionMember::symbol);
                let is_bound = bindings.iter().any(|(key, _)| *key == requirement.key);
                if declared.is_none() && !is_bound && !requirement.has_default {
                    return Ok(ConformanceSelection::Missing);
                }
                selected.push(dir::MemberConformance {
                    member: declared.unwrap_or(requirement.symbol),
                    requirement: requirement.symbol,
                });

                continue;
            }

            let mut candidates =
                SmallVec::<[(dir::GlobalSymbolId, Option<dir::GlobalTypeId>); 2]>::new();
            for member in members {
                if member.space() != requirement.space || member.key() != Some(requirement.key) {
                    continue;
                }
                let member = match self.declared_member(member)? {
                    Some(member) => member,
                    None => continue,
                };
                let ty = member
                    .ty
                    .map(|ty| self.substitute_type(ty, &substitution))
                    .transpose()?;
                candidates.push((member.symbol, ty));
            }

            // include matching members inherited by the target declaration
            if candidates.is_empty() {
                let Some(inherent) =
                    self.inherent_member_candidates(origin, target, requirement)?
                else {
                    return Ok(ConformanceSelection::Undecided(
                        self.collect_open_variables([target, interface])?,
                    ));
                };
                for candidate in inherent {
                    let Some(symbol) = candidate.symbol() else {
                        continue;
                    };
                    let ty = candidate.callable.unwrap_or(candidate.access.store());
                    candidates.push((symbol, Some(ty)));
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
                    return Ok(ConformanceSelection::Missing);
                }

                continue;
            }

            // match typed requirements and accept abstract requirements by presence
            let member = if let Some(required) = requirement.ty {
                // read the requirement through this implementation
                let required = self.substitute_type(required, &substitution)?;
                let required = self.instantiate_interface_type(required, interface, target)?;
                let required = self.bind_projections(required, target, &bindings)?;

                // select the first candidate relating to the requirement
                let mut selected = None;
                for (symbol, found) in candidates {
                    let Some(found) = found else {
                        continue;
                    };
                    let found = self.bind_projections(found, target, &bindings)?;

                    // skip a candidate whose parameter list shape differs from the requirement
                    if !self.signature_shapes_match(found, required)? {
                        continue;
                    }

                    let decision = self.relate_member(
                        origin,
                        Relation::Storable,
                        requirement.role,
                        found,
                        required,
                        substitution.receiver,
                        Some(&assumed),
                    )?;
                    if decision.holds() {
                        selected = Some(symbol);
                        break;
                    }
                }
                match selected {
                    Some(member) => member,
                    // take the default over every unimplementing candidate
                    None if requirement.has_default => requirement.symbol,
                    None => return Ok(ConformanceSelection::Missing),
                }
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
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let signatures = self.relate_interface_signatures(
            origin,
            cause,
            Relation::Subtype,
            target,
            &requirements,
        )?;
        if !signatures.holds() {
            return Ok(ConformanceSelection::Missing);
        }

        // validate inherited interfaces through the same declaration
        for inherited in &requirements.inherited {
            let inherited = self.select_declared_conformance(
                origin,
                inherited.ty,
                target,
                members,
                is_unsafe_extension,
            )?;
            match inherited {
                ConformanceSelection::Selected(inherited) => selected.extend(inherited),
                ConformanceSelection::Missing => {
                    return Ok(ConformanceSelection::Missing);
                }
                ConformanceSelection::Undecided(stalls) => {
                    return Ok(ConformanceSelection::Undecided(stalls));
                }
            }
        }

        Ok(ConformanceSelection::Selected(selected))
    }

    /// Collect the applications of one symbol with the given arguments.
    pub(in crate::sema) fn plain_applications_of(
        &self,
        ty: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        // walk the type tree from the root, visiting each node once
        let mut found = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = SmallVec::<[dir::GlobalTypeId; 16]>::new();
        pending.push(ty);
        while let Some(id) = pending.pop() {
            if visited.contains(&id) {
                continue;
            }

            visited.push(id);
            let node = self.ty(id)?;

            // match the application's symbol and exact arguments
            if let dir::Type::Application(instance) = node
                && instance.symbol == symbol
                && self.type_ids(id.module_id, instance.arguments)? == arguments
                && !found.contains(&id)
            {
                found.push(id);

                continue;
            }

            // descend into every child of an unmatched node
            self.for_each_type_child(id.module_id, &node, |child| pending.push(child))?;
        }

        Ok(found)
    }

    /// Return target members matching one interface requirement.
    fn inherent_member_candidates(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        requirement: &InterfaceMember,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let lookup = self.lookup_inherent_member(
            origin,
            origin.module(),
            target,
            requirement.space,
            requirement.key,
        )?;

        // requirements are satisfied by every member visible on the target
        let mut is_visible = false;
        let candidates = if lookup.is_empty() && !requirement.has_default {
            is_visible = true;
            self.lookup_visible_member(
                origin,
                origin.module(),
                target,
                requirement.space,
                requirement.key,
            )?
        } else {
            lookup
        };

        // keep the public members, a visible one declared beside its target
        let mut kept = Vec::with_capacity(candidates.candidates.len());
        for candidate in candidates.candidates {
            let Some(declared) = candidate.declaration() else {
                return Err(CompilerError::Internal {
                    message: "nominal implementation has a structural member".into(),
                });
            };
            if !self.is_public_member(declared.symbol)? {
                continue;
            }
            if is_visible
                && (declared.origin == dir::MemberOrigin::BlanketExtension
                    || matches!(
                        self.definition(declared.owner)?.as_deref(),
                        Some(dir::Definition::Interface(_))
                    ))
            {
                continue;
            }
            kept.push(candidate);
        }

        Ok(Some(kept))
    }

    /// Return whether one member is part of its declaration's public membership.
    fn is_public_member(&mut self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        // read the visibility the declaration gives the member
        let visibility = self.member_visibility(symbol)?;

        Ok(!matches!(
            visibility,
            Some((_, dir::Visibility::Private | dir::Visibility::Protected))
        ))
    }

    /// Check whether one type satisfies one compiler-known auto interface.
    pub(in crate::sema) fn check_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<ObligationCheck> {
        // hold without checking once an operand already reported an error
        if self.has_error_operand(&[ty])? {
            return Ok(ObligationCheck::holds());
        }

        // decide the conformance rule of the interface
        match self.decide_auto_interface(origin, ty, interface)? {
            Verdict::Holds => return Ok(ObligationCheck::holds()),
            // stall the obligation while an open variable leaves the rule undecided
            Verdict::Ambiguous => {
                return Ok(ObligationCheck::Ambiguous(
                    self.collect_open_variables([ty])?,
                ));
            }
            Verdict::Fails => {}
        }

        // report the unsatisfied interface against the obligation's source
        let source = self.origin_source(origin)?;
        let failure = ObligationFailure::AutoInterfaceNotSatisfied {
            source,
            ty,
            interface,
        };

        Ok(ObligationCheck::fail(failure))
    }
}

/// One declared conformance selection outcome.
enum ConformanceSelection {
    /// The interface is implemented by these member selections.
    Selected(Vec<dir::MemberConformance>),
    /// The interface has no implementation.
    Missing,
    /// Open variables leave the conformance undecided.
    Undecided(SmallVec<[dir::TypeVariableId; 2]>),
}
