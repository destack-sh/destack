use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, ObligationCheck, ObligationFailure, Origin, TypeSubstitution,
    answer,
};

impl CheckState<'_> {
    /// Check one extension's implemented interfaces.
    pub(in crate::check) fn check_extension_conformance(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let source = self.origin_source(origin)?;
        let Some(dir::Definition::Extension(extension)) = self.definition(symbol)? else {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        };
        let members = extension.members.clone();
        let target = extension.target.r#type();
        let implements = extension
            .implements
            .iter()
            .cloned()
            .collect::<SmallVec<[_; 2]>>();
        if implements.is_empty() {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        // require each declared implementation to satisfy its interface
        let failures =
            answer!(self.check_extension_members(origin, source, target, &members, &implements)?);
        let check = ObligationCheck::from_failures(failures);

        Ok(Answer::Ready(check))
    }

    /// Check one extension's implementation coherence.
    pub(in crate::check) fn check_implementation_coherence(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let source = self.origin_source(origin)?;
        let Some(dir::Definition::Extension(extension)) = self.definition(symbol)? else {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        };
        let target = extension.target;
        let form = extension.form;
        let implements = extension
            .implements
            .iter()
            .cloned()
            .collect::<SmallVec<[_; 2]>>();

        // reject anonymous exported extensions on nonlocal targets
        let module = source.module_id;
        let mut failures = Vec::new();
        if self.is_unnamed_exported_nonlocal_extension(module, symbol, form, target) {
            failures.push(ObligationFailure::UnnamedExportedNonlocalExtension {
                source,
                target: target.r#type(),
            });
        }

        if implements.is_empty() {
            let check = ObligationCheck::from_failures(failures);

            return Ok(Answer::Ready(check));
        }
        let package = module.package_id;

        match target {
            // reject extension implementation pairs outside both packages
            dir::ExtensionTarget::Rooted { root, ty } => {
                let foreign_target = root.module_id.package_id != package;
                for interface in implements.iter().map(|heritage| heritage.symbol) {
                    if foreign_target && interface.module_id.package_id != package {
                        failures.push(ObligationFailure::NonLocalImplementation {
                            source,
                            interface,
                            ty: root,
                        });
                    }
                }

                let conflicts = answer!(self.check_conflicting_implementations(
                    origin,
                    module,
                    source,
                    symbol,
                    root,
                    ty,
                    &implements,
                )?);
                failures.extend(conflicts);
            }
            _ => {
                // require open implementations beside their interface
                for interface in implements.iter().map(|heritage| heritage.symbol) {
                    if interface.module_id.package_id != package {
                        failures.push(ObligationFailure::ForeignBlanketImplementation {
                            source,
                            interface,
                        });
                    }
                }
            }
        }

        let check = ObligationCheck::from_failures(failures);

        Ok(Answer::Ready(check))
    }

    /// Return whether an exported extension needs a source-level name.
    fn is_unnamed_exported_nonlocal_extension(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        form: dir::ExtensionForm,
        target: dir::ExtensionTarget,
    ) -> bool {
        if form != dir::ExtensionForm::Exported {
            return false;
        }

        // blanket extensions are anchored by their bound interface
        if matches!(self.ty(target.r#type()), Ok(dir::Type::Parameter(_))) {
            return false;
        }

        let target_is_local = target.root().is_some_and(|root| root.module_id == module);
        if target_is_local {
            return false;
        }

        self.binding_table(symbol.module_id)
            .get_symbol(symbol.local_id)
            .key
            .is_none()
    }

    /// Check one extension's declared members against its implemented interfaces.
    fn check_extension_members(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        implements: &[dir::NominalHeritage],
    ) -> CompilerResult<Answer<Vec<ObligationFailure>>> {
        let mut failures = Vec::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // check each implemented interface independently
        for heritage in implements {
            let arguments = self.intern_type_ids(source.module_id, &heritage.arguments)?;
            let interface = dir::GenericInstance {
                symbol: heritage.symbol,
                arguments,
            };
            let reported = self.intern_type(source.module_id, dir::Type::Instance(interface))?;
            let result = self.check_extension_interface(
                origin,
                source,
                heritage.source,
                reported,
                target,
                members,
                &interface,
            )?;

            match result {
                Answer::Ready(ObligationCheck::Fails(result)) => failures.extend(result),
                Answer::Ready(ObligationCheck::Holds) => {}
                Answer::Pending(pending) => blockers.extend(pending),
            }
        }

        Ok(Answer::ready_unless_blocked(failures, blockers))
    }

    /// Check one extension's declared members against one implemented interface.
    fn check_extension_interface(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        anchor_source: dir::GlobalNodeIdAny,
        reported: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let Some(definition) = self.definition(interface.symbol)? else {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        };
        if !matches!(definition, dir::Definition::Interface(_)) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let interface_substitution =
            self.instance_substitution_with_defaults(source.module_id, interface, Some(target))?;
        let requirements = answer!(self.interface_requirements_with_substitution(
            origin,
            interface.symbol,
            &interface_substitution,
        )?);

        // compare each required member with the extension's declared
        //  members, where any matching overload satisfies the contract
        let extension_substitution = TypeSubstitution::default().with_receiver(target);
        for interface_member in requirements.members {
            let mut candidates = SmallVec::<[_; 2]>::new();
            for member in members {
                let member = match self.body(origin.module()).declared_member(member)? {
                    Answer::Ready(Some(member)) => member,
                    Answer::Ready(None) => continue,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
                if member.matches(interface_member.space, interface_member.key) {
                    candidates.push(member);
                }
            }
            if candidates.is_empty() {
                // defaulted members satisfy their own contract
                if interface_member.has_default {
                    continue;
                }

                let failure = ObligationFailure::InterfaceNotImplemented {
                    source: anchor_source,
                    ty: target,
                    interface: reported,
                };

                return Ok(Answer::Ready(ObligationCheck::fail(failure)));
            }

            // require presence when an associated type stays abstract
            let Some(required_type) = interface_member.ty else {
                continue;
            };

            let mut satisfied = false;
            for found in candidates {
                let Some(found_type) = found.ty else {
                    continue;
                };
                // the found member binds this to the implementing target
                let found_type =
                    self.substitute_type(source.module_id, found_type, &extension_substitution)?;

                let relation = interface_member.role.conformance_relation();
                let assignment =
                    self.decide_relation(origin, relation, found_type, required_type)?;
                if answer!(assignment) {
                    satisfied = true;
                    break;
                }
            }

            if !satisfied {
                let failure = ObligationFailure::InterfaceNotImplemented {
                    source: anchor_source,
                    ty: target,
                    interface: reported,
                };

                return Ok(Answer::Ready(ObligationCheck::fail(failure)));
            }
        }

        // check inherited interfaces against the same extension members
        for application in requirements.inherited {
            let check = self.check_extension_interface(
                origin,
                source,
                anchor_source,
                reported,
                target,
                members,
                &application.instance,
            )?;
            match check {
                Answer::Ready(ObligationCheck::Holds) => {}
                Answer::Ready(failure) => return Ok(Answer::Ready(failure)),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        Ok(Answer::Ready(ObligationCheck::holds()))
    }

    /// Check visible implementations conflicting with one new extension.
    fn check_conflicting_implementations(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        root: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
        implements: &[dir::NominalHeritage],
    ) -> CompilerResult<Answer<Vec<ObligationFailure>>> {
        let mut failures = Vec::new();

        // collect comparable implementations before overlap checks
        let mut candidates = SmallVec::<
            [(
                dir::GlobalSymbolId,
                dir::GlobalTypeId,
                dir::NominalHeritage,
                dir::NominalHeritage,
            ); 2],
        >::new();
        for other in self
            .body(origin.module())
            .visible_extensions(module, root)?
        {
            if other == symbol {
                continue;
            }
            if !self.is_later_definition(source, other) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(other)? else {
                continue;
            };
            let dir::ExtensionTarget::Rooted {
                root: other_root,
                ty: other_ty,
            } = extension.target
            else {
                continue;
            };
            if other_root != root {
                continue;
            }
            for other_heritage in &extension.implements {
                let shared = implements
                    .iter()
                    .find(|heritage| heritage.symbol == other_heritage.symbol);
                if let Some(heritage) = shared {
                    candidates.push((other, other_ty, heritage.clone(), other_heritage.clone()));
                }
            }
        }

        // reject overlapping receivers for one unifiable interface
        //  instantiation: distinct interface arguments never conflict
        for (other, other_ty, heritage, other_heritage) in candidates {
            if !answer!(self.types_may_overlap(origin, ty, other_ty)?) {
                continue;
            }
            if heritage.arguments.len() == other_heritage.arguments.len() {
                let mut distinct = false;
                for (left, right) in heritage
                    .arguments
                    .iter()
                    .copied()
                    .zip(other_heritage.arguments.iter().copied())
                {
                    if !answer!(self.types_may_overlap(origin, left, right)?) {
                        distinct = true;
                        break;
                    }
                }
                if distinct {
                    continue;
                }
            }

            failures.push(ObligationFailure::ConflictingImplementation {
                source,
                conflict: other,
                interface: heritage.symbol,
                ty,
            });
        }

        Ok(Answer::Ready(failures))
    }

    /// Return whether `source` is later than one other local definition.
    fn is_later_definition(
        &self,
        source: dir::GlobalNodeIdAny,
        other: dir::GlobalSymbolId,
    ) -> bool {
        let Some(state) = self.modules.get(&other.module_id) else {
            return true;
        };
        let other_source = state.definitions.definition_source(other);
        if other_source.module_id != source.module_id {
            return true;
        }

        source.local_id.id > other_source.local_id.id
    }
}
