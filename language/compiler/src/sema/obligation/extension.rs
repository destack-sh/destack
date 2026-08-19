use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CheckState, ExtensionCoherenceObligation, ImplementationCoherenceObligation,
    ObligationCheck, ObligationFailure, Origin,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Check one extension's implementation coherence.
    pub(in crate::sema) fn check_implementation_coherence(
        &mut self,
        origin: Origin,
        obligation: &ImplementationCoherenceObligation,
    ) -> CompilerResult<ObligationCheck> {
        let source = obligation.source;
        let symbol = obligation.symbol;
        let Some(dir::Definition::Extension(extension)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("extension obligation has no extension definition: {symbol:?}"),
            });
        };
        let target = extension.target;
        let form = extension.form;
        let interfaces = self
            .declared_interfaces(symbol)?
            .into_iter()
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

        // reject parameters the target and conformances leave unconstrained
        self.check_unconstrained_extension_parameters(
            symbol,
            target.r#type(),
            &interfaces,
            &mut failures,
        )?;

        // require every member of an unanchored blanket to implement a declared interface member
        if let dir::ExtensionTarget::Blanket { ty, .. } = target
            && !self.is_blanket_interface_anchored(ty)?
        {
            self.check_blanket_member_anchoring(symbol, source, &interfaces, &mut failures)?;
        }

        if interfaces.is_empty() {
            let check = ObligationCheck::from_failures(failures);

            return Ok(check);
        }
        let package = module.package_id;

        match target {
            // reject extension implementation pairs outside both packages
            dir::ExtensionTarget::Rooted { root, ty } => {
                let foreign_target = root.module_id.package_id != package;
                for implemented in &interfaces {
                    let (_, interface) = self.nominal_application(*implemented)?;
                    let interface = interface.symbol;
                    if foreign_target && interface.module_id.package_id != package {
                        failures.push(ObligationFailure::NonLocalImplementation {
                            source,
                            interface,
                            ty: root,
                        });
                    }
                }

                let conflicts = self.check_conflicting_implementations(
                    origin,
                    module,
                    source,
                    symbol,
                    root,
                    ty,
                    &interfaces,
                )?;
                failures.extend(conflicts);
            }
            _ => {
                // require open implementations beside their interface
                for implemented in &interfaces {
                    let (_, interface) = self.nominal_application(*implemented)?;
                    let interface = interface.symbol;
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

        Ok(check)
    }

    /// Return whether one blanket target's own constraint anchors an interface.
    fn is_blanket_interface_anchored(&mut self, target: dir::GlobalTypeId) -> CompilerResult<bool> {
        // read the constraint the blanket parameter declares
        let dir::Type::Parameter(parameter) = self.ty(target)? else {
            return Ok(false);
        };
        let Some(constraint) = self
            .generic_parameter(parameter)
            .and_then(|binding| binding.constraint)
        else {
            return Ok(false);
        };

        // answer whether that constraint applies an interface
        let Some((_, instance)) = self.nominal_application_maybe(constraint)? else {
            return Ok(false);
        };

        self.symbol_kind(instance.symbol)
            .map(|kind| kind.is_interface())
    }

    /// Reject blanket members outside the declared interfaces' member keys.
    fn check_blanket_member_anchoring(
        &mut self,
        symbol: dir::GlobalSymbolId,
        source: dir::GlobalNodeIdAny,
        interfaces: &[dir::GlobalTypeId],
        failures: &mut Vec<ObligationFailure>,
    ) -> CompilerResult<()> {
        // collect the member keys the declared interfaces admit
        let mut admitted = FxIndexSet::default();
        for implemented in interfaces {
            let (_, interface) = self.nominal_application(*implemented)?;
            if let Some(dir::Definition::Interface(definition)) =
                self.definition(interface.symbol)?
            {
                admitted.extend(definition.members.iter().filter_map(|member| member.key()));
            }
        }

        // read the keys the extension declares
        let Some(dir::Definition::Extension(extension)) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("blanket obligation symbol {symbol:?} names no extension"),
            });
        };
        let declared: SmallVec<[dir::StaticKey; 8]> = extension
            .members
            .iter()
            .filter_map(|member| member.key())
            .collect();

        // require every declared key to implement an admitted member
        for key in declared {
            if !admitted.contains(&key) {
                let member = self.format_static_key(&key);
                failures.push(ObligationFailure::UnanchoredBlanketMember { source, member });
            }
        }

        Ok(())
    }

    /// Reject extension parameters the target and conformances leave unconstrained.
    fn check_unconstrained_extension_parameters(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalTypeId,
        interfaces: &[dir::GlobalTypeId],
        failures: &mut Vec<ObligationFailure>,
    ) -> CompilerResult<()> {
        let Some(template) = self.symbol_template(symbol)? else {
            return Ok(());
        };

        // walk constraint reachability from the target and conformance headers
        let mut constrained = FxIndexSet::default();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(interfaces);
        pending.push(target);
        while let Some(ty) = pending.pop() {
            for parameter in self.type_parameters(ty)? {
                if !constrained.insert(parameter) {
                    continue;
                }
                let constraint = self
                    .generic_parameter(parameter)
                    .and_then(|binding| binding.constraint);
                pending.extend(constraint);
            }
        }

        // declared type parameters outside the constrained set are rejected
        for parameter in self.generic_template_parameters(template)? {
            let Some(binding) = self.generic_parameter(parameter).copied() else {
                continue;
            };
            if binding.kind != dir::GenericParameterKind::Type
                || binding.symbol.is_none()
                || constrained.contains(&parameter)
            {
                continue;
            }

            failures.push(ObligationFailure::UnconstrainedExtensionParameter {
                source: binding.source,
                parameter: self.format_type(binding.ty),
            });
        }

        Ok(())
    }

    /// Return the generic parameters one type graph mentions.
    fn type_parameters(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalGenericParameterId; 2]>> {
        // skip the scan when the interned flags name no parameter
        if !self.type_flags(id)?.has_parameter() {
            return Ok(SmallVec::new());
        }

        // scan the type graph, stopping at symbol references
        let mut parameters = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = FxIndexSet::default();
        pending.push(id);
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }

            let ty = self.ty(id)?;
            if let dir::Type::Parameter(parameter) = ty {
                if !parameters.contains(&parameter) {
                    parameters.push(parameter);
                }

                continue;
            }

            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(parameters)
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

        // accept a blanket extension, its bound interface names it
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

    /// Return one symbol's implemented interfaces as written.
    fn declared_interfaces(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // prefer the own module's authored implementations over a foreign definition
        if let Some(module) = self.module_maybe(symbol.module_id)
            && let Some(declared) = &module.declared
            && let Some(definition) = declared.definitions.definition(symbol)
        {
            let interfaces = definition
                .implementations()
                .iter()
                .map(|conformance| conformance.interface)
                .collect();

            return Ok(interfaces);
        }

        let Some(definition) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("implementation has no definition: {symbol:?}"),
            });
        };

        let interfaces = definition
            .implementations()
            .iter()
            .map(|conformance| conformance.interface)
            .collect();

        Ok(interfaces)
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
        interfaces: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<ObligationFailure>> {
        let mut failures = Vec::new();

        // collect comparable implementations before overlap checks
        let mut candidates = SmallVec::<
            [(
                dir::GlobalSymbolId,
                dir::GlobalTypeId,
                dir::GlobalTypeId,
                dir::GlobalTypeId,
            ); 2],
        >::new();
        for other in self.body().visible_extensions(module, root)? {
            if other == symbol {
                continue;
            }
            if !self.is_later_definition(source, other) {
                continue;
            }
            let (other_root, other_ty) = match self.definition(other)? {
                Some(dir::Definition::Extension(extension)) => match extension.target {
                    dir::ExtensionTarget::Rooted { root, ty } => (root, ty),
                    dir::ExtensionTarget::Blanket { .. } => continue,
                },
                Some(_) | None => continue,
            };
            if other_root != root {
                continue;
            }
            let other_interfaces = self.declared_interfaces(other)?;
            for other_interface_type in other_interfaces {
                let (_, other_interface) = self.nominal_application(other_interface_type)?;
                for interface_type in interfaces {
                    let (_, interface) = self.nominal_application(*interface_type)?;
                    if interface.symbol == other_interface.symbol {
                        candidates.push((other, other_ty, *interface_type, other_interface_type));
                        break;
                    }
                }
            }
        }

        // reject overlapping receivers under one unifiable interface instantiation
        for (other, other_ty, interface, other_interface) in candidates {
            if !self.types_may_overlap(origin, ty, other_ty)? {
                continue;
            }
            let (interface_module, interface_application) = self.nominal_application(interface)?;
            let (other_module, other_application) = self.nominal_application(other_interface)?;
            let interface_arguments =
                self.filled_application_arguments(interface_module, &interface_application)?;
            let other_arguments =
                self.filled_application_arguments(other_module, &other_application)?;
            if interface_arguments.len() == other_arguments.len() {
                let mut distinct = false;
                for (left, right) in interface_arguments
                    .iter()
                    .copied()
                    .zip(other_arguments.iter().copied())
                {
                    if !self.types_may_overlap(origin, left, right)? {
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
                interface: interface_application.symbol,
                ty,
            });
        }

        Ok(failures)
    }

    /// Return whether `source` is later than one other local definition.
    fn is_later_definition(
        &self,
        source: dir::GlobalNodeIdAny,
        other: dir::GlobalSymbolId,
    ) -> bool {
        let Some(state) = self.module_maybe(other.module_id) else {
            return true;
        };
        let Some(other_source) = state.definition_source_maybe(other) else {
            return true;
        };
        if other_source.module_id != source.module_id {
            return true;
        }

        source.local_id.id > other_source.local_id.id
    }
}

impl BodyState<'_, '_> {
    /// Check one extension against other visible extensions' properties.
    pub(in crate::sema) fn check_extension_coherence(
        &mut self,
        obligation: &ExtensionCoherenceObligation,
    ) -> CompilerResult<ObligationCheck> {
        // read the extension being checked
        let extension_symbol = obligation.symbol;
        let module = extension_symbol.module_id;
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)? else {
            return Err(CompilerError::Internal {
                message: format!(
                    "extension coherence has no extension definition: {extension_symbol:?}"
                ),
            });
        };
        let extension = extension.clone();
        let mut failures = Vec::new();

        // collect the properties this extension declares
        let declared = self.property_members(&extension.members)?;
        if declared.is_empty() {
            return Ok(ObligationCheck::holds());
        }

        // render the target once for the failure reports
        let target = self.format_type(extension.target.r#type());

        // gather competitors sharing the target root or ground head
        let root = extension.target.root();
        let competitors = match root {
            Some(root) => self.visible_extensions(module, root)?,
            None => self.visible_blanket_extensions(module)?,
        };
        let ground = match root {
            Some(_) => None,
            None => match self.ty(extension.target.r#type())? {
                dir::Type::Primitive(primitive) => Some(primitive),
                // leave parameterized blanket overlap to use sites
                _ => return Ok(ObligationCheck::holds()),
            },
        };

        // report every property a competing extension already declares
        for competitor_symbol in competitors {
            // leave same-module collisions to source order
            if competitor_symbol == extension_symbol || competitor_symbol.module_id == module {
                continue;
            }
            // require the competitor to extend the same target
            let Some(dir::Definition::Extension(competitor)) =
                self.definition(competitor_symbol)?
            else {
                continue;
            };
            let competes = match root {
                Some(root) => competitor.target.root() == Some(root),
                None => competitor.target.is_blanket(),
            };
            if !competes {
                continue;
            }

            // require a blanket competitor to share the ground head
            let competitor_target = competitor.target.r#type();
            let members = competitor.members.clone();
            if ground.is_some()
                && !matches!(
                    self.ty(competitor_target)?,
                    dir::Type::Primitive(primitive) if Some(primitive) == ground
                )
            {
                continue;
            }

            // report each declared property the competitor also declares
            let other = self.property_members(&members)?;
            for member in &declared {
                let duplicated = other.iter().any(|candidate| {
                    candidate.key == member.key
                        && candidate.space == member.space
                        && candidate.form == member.form
                        && ((candidate.reads && member.reads)
                            || (candidate.writes && member.writes))
                });
                if duplicated {
                    failures.push(ObligationFailure::DuplicateExtensionMember {
                        source: member.source,
                        member: member.key,
                        target: target.clone(),
                    });
                }
            }
        }

        Ok(ObligationCheck::from_failures(failures))
    }

    /// Collect the property members one extension declares.
    fn property_members(
        &mut self,
        members: &[dir::DefinitionMember],
    ) -> CompilerResult<Vec<PropertyMember>> {
        let mut properties = Vec::new();
        for member in members {
            let (reads, writes) = match member {
                dir::DefinitionMember::Field(field) => (true, !field.is_readonly),
                dir::DefinitionMember::Method(method) => match method.role {
                    Some(dir::FunctionRole::Getter) => (true, false),
                    Some(dir::FunctionRole::Setter) => (false, true),
                    // methods union as overloads and never collide
                    _ => continue,
                },
                _ => continue,
            };
            let Some(key) = member.key() else {
                continue;
            };
            let Some(ty) = self.definition_member_type(member)? else {
                continue;
            };
            let this = self
                .signature_head(ty)?
                .and_then(|signature| signature.this_parameter);
            let Some(form) = self.property_receiver(this)? else {
                continue;
            };

            properties.push(PropertyMember {
                key,
                space: member.space(),
                reads,
                writes,
                form,
                source: member.source(),
            });
        }

        Ok(properties)
    }

    /// Return the comparable declared receiver of one property member.
    fn property_receiver(
        &mut self,
        this: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<PropertyReceiver>> {
        let Some(this) = this else {
            return Ok(Some(PropertyReceiver::Default));
        };
        let form = match self.ty(this)? {
            dir::Type::Form(form) => match form.form {
                // borrows compare by their access value
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.check.type_borrow(this.module_id, borrow)?;
                    let access = borrow.access;
                    let access = match self.ty(access)? {
                        dir::Type::Literal(dir::ScalarLiteral::String(name)) => Some(name),
                        _ => None,
                    };

                    Some(PropertyReceiver::Borrowed(access))
                }
                dir::Form::Owned => Some(PropertyReceiver::Owned),
                dir::Form::Raw => Some(PropertyReceiver::Raw),
                _ => Some(PropertyReceiver::Default),
            },
            // conversion receivers never collide with plain slots
            dir::Type::Application(_) => None,
            _ => Some(PropertyReceiver::Default),
        };

        Ok(form)
    }
}

/// One exclusive property member compared for duplicates.
struct PropertyMember {
    /// The member key.
    key: dir::StaticKey,
    /// The member space declaring the property.
    space: dir::MemberSpace,
    /// Whether the property serves reads.
    reads: bool,
    /// Whether the property serves writes.
    writes: bool,
    /// The comparable declared receiver form.
    form: PropertyReceiver,
    /// The declaring member source node.
    source: dir::GlobalNodeIdAny,
}

/// The comparable declared receiver of one property member.
#[derive(PartialEq)]
enum PropertyReceiver {
    /// The family-default managed receiver.
    Default,
    /// A borrowed receiver compared by access.
    Borrowed(Option<dir::StringId>),
    /// An owned receiver.
    Owned,
    /// A raw pointer receiver.
    Raw,
}
