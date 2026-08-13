use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseKind, CheckState, InterfaceConformanceObligation, InterfaceMember, MemberCandidate,
    MemberLookup, ObligationCheck, ObligationFailure, Origin, Relation, TypeSubstitution, Verdict,
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
        let extension_target = match definition {
            dir::Definition::Extension(extension) => Some(extension.target.r#type()),
            dir::Definition::Struct(_) | dir::Definition::Class(_) | dir::Definition::Enum(_) => {
                None
            }
            dir::Definition::TypeAlias(_)
            | dir::Definition::Interface(_)
            | dir::Definition::Newtype(_) => return Ok(ObligationCheck::Holds),
        };

        // read the declared members and the interfaces they must satisfy
        let members = definition.members().to_vec();
        let implementations = definition
            .implementations()
            .iter()
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
                &members,
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

        // write each interface's selected members onto its conformance
        for (conformance, members) in conformances.iter_mut().zip(member_selections) {
            conformance.members = members;
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
        // validate compiler-known markers through their compiler rule
        let (_, application) = self.nominal_application(interface)?;
        let auto_interface = self
            .language_item(application.symbol)?
            .and_then(dir::AutoInterface::from_language_item)
            .filter(|interface| interface.is_intrinsic());
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
        match self.satisfies_intrinsic_interface(origin, target, interface, auto_interface)? {
            Verdict::Holds => Ok(ConformanceSelection::Selected(Vec::new())),
            Verdict::Fails => Ok(ConformanceSelection::Missing),
            // leave the conformance undecided while its variables stay open
            Verdict::Ambiguous => Ok(ConformanceSelection::Undecided(
                self.open_type_variables([target, interface])?,
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

        // read the members, signatures, and inherited interfaces it demands
        let mut requirements = self.interface_requirements(interface, target)?;

        // project interface-owner members through this implementation's refinements
        let mut base = interface;
        while let Some(refined) = self.refined_head(base)? {
            base = refined.base;
        }
        if base != interface
            && let dir::Type::Application(base_instance) = self.ty(base)?
        {
            let module = self.module_id;
            let base_arguments = self
                .type_ids(base.module_id, base_instance.arguments)?
                .to_vec();
            for index in 0..requirements.members.len() {
                let Some(ty) = requirements.members[index].ty else {
                    continue;
                };
                let plain =
                    self.plain_applications_of(ty, base_instance.symbol, &base_arguments)?;
                let mut rewritten = ty;
                for occurrence in plain {
                    rewritten = self.replace_type(module, rewritten, occurrence, interface)?;
                }
                requirements.members[index].ty = Some(rewritten);
            }
        }

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
                let inherent = self.inherent_member_candidates(origin, target, requirement)?;
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
                    return Ok(ConformanceSelection::Missing);
                }

                continue;
            }

            // match typed requirements and accept abstract requirements by presence
            let member = if let Some(required) = requirement.ty {
                let required = self.substitute_type(required, &substitution)?;
                let mut selected = None;
                for (symbol, found) in candidates {
                    let Some(found) = found else {
                        continue;
                    };
                    let decision = self.relate_member(
                        origin,
                        Relation::Assignable,
                        requirement.role,
                        found,
                        required,
                        substitution.receiver,
                    )?;
                    if decision {
                        selected = Some(symbol);
                        break;
                    }
                }
                let Some(member) = selected else {
                    return Ok(ConformanceSelection::Missing);
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
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let signatures = self.relate_interface_signatures(
            origin,
            cause,
            Relation::Satisfies,
            target,
            &requirements,
        )?;
        if !signatures {
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
    fn plain_applications_of(
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
    ) -> CompilerResult<Vec<MemberCandidate>> {
        let lookup = self.body().lookup_inherent_member(
            origin,
            origin.module(),
            target,
            requirement.space,
            requirement.key,
        )?;

        // yield declaration candidates for a nominal implementation
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
    pub(in crate::sema) fn check_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<ObligationCheck> {
        // hold without checking once an operand already reported an error
        if self.any_error_operand(&[ty])? {
            return Ok(ObligationCheck::holds());
        }

        // decide the interface's own conformance rule
        if self.satisfies_auto_interface(origin, ty, interface)? {
            return Ok(ObligationCheck::holds());
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
