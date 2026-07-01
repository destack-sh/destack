use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, Dependency, Origin, Relation, answer};
use crate::{CheckError, CompilerResult};

impl CheckState<'_> {
    /// Check one extension's implemented interfaces.
    pub(in crate::check) fn check_extension_conformance(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let Some(dir::Definition::Extension(extension)) = self.definition(symbol) else {
            return Ok(Answer::Ready(None));
        };
        let members = extension.members.clone();
        let implements = extension
            .implements
            .iter()
            .cloned()
            .collect::<SmallVec<[_; 2]>>();
        if implements.is_empty() {
            return Ok(Answer::Ready(None));
        }
        let module = source.module_id;

        // require each declared implementation to satisfy its interface
        let conformance =
            self.check_extension_members(source, extension.target.r#type(), &members, &implements)?;
        let diagnostics = answer!(conformance);
        self.module_mut(module).diagnostics.extend(diagnostics);

        Ok(Answer::Ready(None))
    }

    /// Check one extension's implementation coherence.
    pub(in crate::check) fn check_implementation_coherence(
        &mut self,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let Some(dir::Definition::Extension(extension)) = self.definition(symbol) else {
            return Ok(Answer::Ready(None));
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
        if self.is_unnamed_exported_nonlocal_extension(module, symbol, form, target) {
            self.report_unnamed_exported_nonlocal_extension(source, target.r#type());
        }

        if implements.is_empty() {
            return Ok(Answer::Ready(None));
        }
        let package = module.package_id;

        match target {
            // reject extension implementation pairs outside both packages
            dir::ExtensionTarget::Rooted { root, ty } => {
                let foreign_target = root.module_id.package_id != package;
                for interface in implements.iter().map(|heritage| heritage.symbol) {
                    if foreign_target && interface.module_id.package_id != package {
                        self.report_non_local_implementation(source, interface, root);
                    }
                }

                let coherence = self.check_conflicting_implementations(
                    module,
                    source,
                    symbol,
                    root,
                    ty,
                    &implements,
                )?;
                answer!(coherence);
            }
            _ => {
                // require open implementations beside their interface
                for interface in implements.iter().map(|heritage| heritage.symbol) {
                    if interface.module_id.package_id != package {
                        self.report_foreign_blanket_implementation(source, interface);
                    }
                }
            }
        }

        Ok(Answer::Ready(None))
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
        source: dir::GlobalNodeIdAny,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        implements: &[dir::NominalHeritage],
    ) -> CompilerResult<Answer<Vec<DiagnosticBuilder<CheckError>>>> {
        let mut diagnostics = Vec::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();

        // check each implemented interface independently
        for heritage in implements {
            let interface = dir::GenericInstance {
                symbol: heritage.symbol,
                arguments: heritage.arguments.clone(),
            };
            let result = self.check_extension_interface(
                source,
                heritage.source,
                target,
                members,
                &interface,
            )?;

            match result {
                Answer::Ready(Some(diagnostic)) => diagnostics.push(diagnostic),
                Answer::Ready(None) => {}
                Answer::Pending(pending) => blockers.extend(pending),
            }
        }

        let blockers = self.live_blockers(blockers);

        Ok(Answer::ready_unless_blocked(diagnostics, blockers))
    }

    /// Check one extension's declared members against one implemented interface.
    fn check_extension_interface(
        &mut self,
        source: dir::GlobalNodeIdAny,
        anchor_source: dir::GlobalNodeIdAny,
        target: dir::GlobalTypeId,
        members: &[dir::DefinitionMember],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let Some(definition) = self.definition(interface.symbol) else {
            return Ok(Answer::Ready(None));
        };
        if !matches!(definition, dir::Definition::Interface(_)) {
            return Ok(Answer::Ready(None));
        }

        let required = answer!(self.interface_members(Origin::Node(source), interface, target)?);

        // compare each required member with the extension's declared member
        for interface_member in required {
            let mut found = None;
            for member in members {
                let member = match self.declared_member(member)? {
                    Answer::Ready(Some(member)) => member,
                    Answer::Ready(None) => continue,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };
                if member.matches(interface_member.space, interface_member.key) {
                    found = Some(member);
                    break;
                }
            }
            let Some(found) = found else {
                return Ok(Answer::Ready(Some(self.extension_interface_error(
                    anchor_source,
                    target,
                    interface.symbol,
                ))));
            };

            // associated types without values only need presence
            let Some(required_type) = interface_member.ty else {
                continue;
            };
            let Some(found_type) = found.ty else {
                return Ok(Answer::Ready(Some(self.extension_interface_error(
                    anchor_source,
                    target,
                    interface.symbol,
                ))));
            };

            let assignment = if found.role.uses_method_assignability()
                && interface_member.role.uses_method_assignability()
            {
                self.decide_method_assignable(Origin::Node(source), found_type, required_type)?
            } else {
                self.decide_relation(
                    Origin::Node(source),
                    Relation::Assignable,
                    found_type,
                    required_type,
                )?
            };

            if !answer!(assignment) {
                return Ok(Answer::Ready(Some(self.extension_interface_error(
                    anchor_source,
                    target,
                    interface.symbol,
                ))));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Return one extension interface diagnostic.
    fn extension_interface_error(
        &mut self,
        source: dir::GlobalNodeIdAny,
        target: dir::GlobalTypeId,
        interface: dir::GlobalSymbolId,
    ) -> DiagnosticBuilder<CheckError> {
        let (module, anchor) = self.source_anchor(source);
        let error = CheckError::InterfaceNotImplemented {
            anchor,
            module,
            source: self.format_type(target),
            target: self.format_symbol(interface),
        };

        error.into()
    }

    /// Report visible implementations conflicting with one new extension.
    fn check_conflicting_implementations(
        &mut self,
        module: ModuleId,
        source: dir::GlobalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        root: dir::GlobalSymbolId,
        ty: dir::GlobalTypeId,
        implements: &[dir::NominalHeritage],
    ) -> CompilerResult<Answer<()>> {
        // collect comparable implementations before overlap checks
        let mut candidates =
            SmallVec::<[(dir::GlobalSymbolId, dir::GlobalTypeId, dir::GlobalSymbolId); 2]>::new();
        for other in self.visible_extensions(module, root) {
            if other == symbol {
                continue;
            }
            if !self.is_later_definition(source, other) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(other) else {
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
            let shared = extension
                .implements
                .iter()
                .map(|heritage| heritage.symbol)
                .find(|interface| {
                    implements
                        .iter()
                        .any(|heritage| heritage.symbol == *interface)
                });
            if let Some(interface) = shared {
                candidates.push((other, other_ty, interface));
            }
        }

        // reject overlapping receivers for the same interface
        let origin = Origin::Node(source);
        for (other, other_ty, interface) in candidates {
            if !answer!(self.types_may_overlap(origin, ty, other_ty)?) {
                continue;
            }

            self.report_conflicting_implementation(source, other, interface, ty)?;
        }

        Ok(Answer::Ready(()))
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
