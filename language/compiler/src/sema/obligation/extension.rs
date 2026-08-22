use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    BodyState, CheckState, ExtensionCoherenceObligation, ImplementationCoherenceObligation,
    ObligationCheck, ObligationFailure, Origin, Relation,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Report the member conflicts inside every definition the checked module declares.
    pub(in crate::sema) fn report_member_conflicts(&mut self) -> CompilerResult<()> {
        let module = self.module_id;
        let definitions = self
            .module
            .iter_definitions()
            .filter(|(symbol, _)| symbol.module_id == module)
            .map(|(_, definition)| definition.clone())
            .collect::<Vec<_>>();
        for definition in &definitions {
            self.report_definition_member_conflicts(definition)?;
        }

        Ok(())
    }

    /// Report the member conflicts inside one definition.
    fn report_definition_member_conflicts(
        &mut self,
        definition: &dir::Definition,
    ) -> CompilerResult<()> {
        let mut seen = FxIndexMap::<(dir::MemberSpace, dir::StaticKey), bool>::default();
        let mut overloads =
            FxIndexMap::<(dir::MemberSpace, dir::StaticKey), Vec<dir::GlobalSymbolId>>::default();

        for member in definition.members() {
            let Some(key) = member.key() else {
                continue;
            };
            let entry = (member.space(), key);
            let is_overloadable = member.is_overloadable();

            // a repeated key is a duplicate unless both declarations overload
            if let Some(previous_is_overloadable) = seen.get(&entry) {
                if !*previous_is_overloadable || !is_overloadable {
                    self.report_duplicate_definition_member(member.source(), &key);
                }
            } else {
                seen.insert(entry, is_overloadable);
            }

            // report a later overload an earlier one already subsumes
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            if method.role.is_some() {
                continue;
            }

            // compare the later signature against each earlier overload of the key
            let later = self.symbol_type(method.symbol)?;
            let earlier_overloads = overloads.entry(entry).or_default().clone();
            for earlier in earlier_overloads {
                // skip earlier overloads generic in a type parameter
                let earlier = self.symbol_type(earlier)?;
                if let Some(template) = self.signature_head(earlier)?.and_then(|head| head.template)
                    && self
                        .generic_template_parameters(template)?
                        .iter()
                        .any(|parameter| {
                            self.generic_parameter(*parameter)
                                .is_none_or(|binding| binding.memory_parameter().is_none())
                        })
                {
                    continue;
                }

                // report the later overload once the earlier signature takes its calls
                let origin = Origin::Node(method.source, None);
                if self.accepts_arities_of(earlier, later)?
                    && self
                        .evaluate_relation(origin, Relation::Assignable, earlier, later)?
                        .holds()
                {
                    self.report_unreachable_overload(method.source, &key);
                    break;
                }
            }

            // record this overload for the members that follow
            overloads.entry(entry).or_default().push(method.symbol);
        }

        Ok(())
    }

    /// Return whether one signature takes every call arity another signature takes.
    fn accepts_arities_of(
        &mut self,
        earlier: dir::GlobalTypeId,
        later: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let (Some(earlier_head), Some(later_head)) =
            (self.signature_head(earlier)?, self.signature_head(later)?)
        else {
            return Ok(false);
        };
        let earlier = self.signature_parameters(earlier.module_id, earlier_head.parameters)?;
        let later = self.signature_parameters(later.module_id, later_head.parameters)?;
        let required = |parameters: &[dir::FunctionParameterType]| {
            parameters
                .iter()
                .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
                .count()
        };
        let earlier_rest = earlier.iter().any(|parameter| parameter.is_rest);
        let later_rest = later.iter().any(|parameter| parameter.is_rest);

        Ok(required(earlier) <= required(later)
            && (earlier_rest || (!later_rest && later.len() <= earlier.len())))
    }

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
        if self.is_unnamed_exported_nonlocal_extension(module, symbol, form, target)? {
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
            && matches!(self.ty(ty)?, dir::Type::Parameter(_))
            && !self.is_blanket_interface_anchored(ty)?
        {
            self.check_blanket_member_anchoring(symbol, source, &interfaces, &mut failures)?;
        }

        // finish a plain extension with the failures collected so far
        if interfaces.is_empty() {
            let check = ObligationCheck::from_failures(failures);

            return Ok(check);
        }

        // judge the implemented pairs against the extension's own package
        let package = module.package_id;
        match target {
            // reject headed implementation pairs outside both packages
            dir::ExtensionTarget::Rooted { root, ty } => {
                // a head is foreign outside its owning package
                let foreign_target = match root {
                    dir::TypeRoot::Declaration(root) => root.module_id.package_id != package,
                    dir::TypeRoot::Primitive(_) | dir::TypeRoot::Tuple => {
                        package != self.language_package()?
                    }
                };
                for implemented in &interfaces {
                    let (_, interface) = self.nominal_application(*implemented)?;
                    let interface = interface.symbol;
                    if foreign_target && interface.module_id.package_id != package {
                        failures.push(ObligationFailure::NonLocalImplementation {
                            source,
                            interface,
                            root,
                        });
                    }
                }

                let conflicts = self.check_conflicting_implementations(
                    origin,
                    module,
                    source,
                    symbol,
                    ty,
                    &interfaces,
                )?;
                failures.extend(conflicts);
            }
            // require blanket implementations beside their interface
            dir::ExtensionTarget::Blanket { ty, .. } => {
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

                let conflicts = self.check_conflicting_implementations(
                    origin,
                    module,
                    source,
                    symbol,
                    ty,
                    &interfaces,
                )?;
                failures.extend(conflicts);
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
    ) -> CompilerResult<bool> {
        if form != dir::ExtensionForm::Exported {
            return Ok(false);
        }

        // accept a blanket extension, its bound interface names it
        if matches!(self.ty(target.r#type())?, dir::Type::Parameter(_)) {
            return Ok(false);
        }

        let target_is_local = target
            .declaration()
            .is_some_and(|root| root.module_id == module);
        if target_is_local {
            return Ok(false);
        }

        let is_unnamed = self
            .binding_table(symbol.module_id)
            .get_symbol(symbol.local_id)
            .key
            .is_none();

        Ok(is_unnamed)
    }

    /// Return one symbol's implemented interfaces as written.
    pub(in crate::sema) fn declared_interfaces(
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
        ty: dir::GlobalTypeId,
        interfaces: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<ObligationFailure>> {
        let mut failures = Vec::new();

        // collect earlier visible implementations sharing one declared interface
        let mut candidates = SmallVec::<[(dir::GlobalSymbolId, dir::GlobalTypeId); 2]>::new();
        for interface_type in interfaces {
            let (_, interface) = self.nominal_application(*interface_type)?;
            for other in self
                .body()
                .visible_implementations(module, interface.symbol)?
            {
                if other == symbol || !self.is_later_definition(source, other) {
                    continue;
                }
                candidates.push((other, *interface_type));
            }
        }

        // reject implementations some declared type satisfies together with this one
        for (other, interface_type) in candidates {
            let Some(witness) = self.body().extension_implementations_overlap(
                origin,
                module,
                symbol,
                ty,
                interface_type,
                other,
            )?
            else {
                continue;
            };
            let (_, interface) = self.nominal_application(interface_type)?;
            failures.push(ObligationFailure::ConflictingImplementation {
                source,
                conflict: other,
                interface: interface.symbol,
                ty: witness,
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
    /// Check one extension against other visible extensions' members.
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

        // collect the inherent members this extension declares
        let requirements = self.requirement_keys(&extension.implements)?;
        let target_form = self
            .receiver_form(extension.target.r#type())?
            .unwrap_or(ReceiverForm::MANAGED);
        let declared = self.keyed_members(&extension.members, target_form, &requirements)?;
        if declared.is_empty() {
            return Ok(ObligationCheck::holds());
        }

        // render the target once for the failure reports
        let target = self.format_type(extension.target.r#type());

        // gather competitors sharing the target head, leaving blanket overlap to use sites
        let root = extension.target.root();
        let competitors = match root {
            Some(root) => self.visible_extensions(module, root)?,
            None => return Ok(ObligationCheck::holds()),
        };

        // report every member the root declaration itself declares
        if let Some(dir::TypeRoot::Declaration(declaration)) = root
            && let Some(definition) = self.definition(declaration)?
        {
            let members = definition.members().to_vec();
            let inherent =
                self.keyed_members(&members, ReceiverForm::MANAGED, &FxIndexSet::default())?;
            for member in &declared {
                let redeclared = inherent.iter().any(|candidate| {
                    candidate.key == member.key
                        && candidate.space == member.space
                        && candidate.form.overlaps(member.form)
                });
                if redeclared {
                    failures.push(ObligationFailure::InherentMemberRedeclared {
                        source: member.source,
                        member: member.key,
                        target: target.clone(),
                    });
                }
            }
        }

        // report every member an earlier competing extension already declares
        for competitor_symbol in competitors {
            if competitor_symbol == extension_symbol
                || !self
                    .check
                    .is_later_definition(obligation.source, competitor_symbol)
            {
                continue;
            }
            // require the competitor to extend the same target
            let Some(dir::Definition::Extension(competitor)) =
                self.definition(competitor_symbol)?
            else {
                continue;
            };
            if competitor.target.root() != root {
                continue;
            }
            let members = competitor.members.clone();
            let implements = competitor.implements.clone();
            let competitor_target = competitor.target.r#type();

            // report each declared member the competitor also declares inherently
            let requirements = self.requirement_keys(&implements)?;
            let competitor_form = self
                .receiver_form(competitor_target)?
                .unwrap_or(ReceiverForm::MANAGED);
            let other = self.keyed_members(&members, competitor_form, &requirements)?;
            for member in &declared {
                let duplicated = other.iter().any(|candidate| {
                    candidate.key == member.key
                        && candidate.space == member.space
                        && candidate.form.overlaps(member.form)
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

    /// Collect the member keys the implemented interfaces require.
    fn requirement_keys(
        &mut self,
        implements: &[dir::NominalConformance],
    ) -> CompilerResult<FxIndexSet<dir::StaticKey>> {
        let mut keys = FxIndexSet::default();
        for conformance in implements {
            let Some((_, interface)) = self.nominal_application_maybe(conformance.interface)?
            else {
                continue;
            };
            let Some(dir::Definition::Interface(definition)) = self.definition(interface.symbol)?
            else {
                continue;
            };
            keys.extend(definition.members.iter().filter_map(|member| member.key()));
        }

        Ok(keys)
    }

    /// Collect the inherent keyed members one extension declares.
    fn keyed_members(
        &mut self,
        members: &[dir::DefinitionMember],
        target_form: ReceiverForm,
        requirements: &FxIndexSet<dir::StaticKey>,
    ) -> CompilerResult<Vec<DeclaredMember>> {
        let mut keyed = Vec::new();
        for member in members {
            let Some(key) = member.key() else {
                continue;
            };
            if requirements.contains(&key) {
                continue;
            }
            let Some(ty) = self.definition_member_type(member)? else {
                continue;
            };

            // compare by an explicit receiver type, else by the extension target's form
            let this = self
                .signature_head(ty)?
                .and_then(|signature| signature.this_parameter);
            let form = match this {
                Some(this) if !matches!(self.ty(this)?, dir::Type::This) => {
                    match self.receiver_form(this)? {
                        Some(form) => form,
                        None => continue,
                    }
                }
                _ => target_form,
            };

            keyed.push(DeclaredMember {
                key,
                space: member.space(),
                form,
                source: member.source(),
            });
        }

        Ok(keyed)
    }

    /// Return the receiver form one explicit `this` type writes.
    pub(in crate::sema) fn receiver_form(
        &mut self,
        this: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ReceiverForm>> {
        let form = match self.ty(this)? {
            dir::Type::Form(form) => match form.form {
                // borrows compare by their access value
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.check.type_borrow(this.module_id, borrow)?;

                    Some(ReceiverForm {
                        ownership: dir::Ownership::Borrowed,
                        access: self.written_access(borrow.access)?,
                    })
                }
                dir::Form::Owned => Some(ReceiverForm {
                    ownership: dir::Ownership::Owned,
                    access: None,
                }),
                dir::Form::Raw => Some(ReceiverForm {
                    ownership: dir::Ownership::Raw,
                    access: None,
                }),
                _ => Some(ReceiverForm::MANAGED),
            },
            // written memory applications compare like the forms they name
            dir::Type::Application(instance) => {
                let arguments = self.type_ids(this.module_id, instance.arguments)?.to_vec();
                match self.language_item(instance.symbol)? {
                    Some(dir::LanguageItem::Owned) => Some(ReceiverForm {
                        ownership: dir::Ownership::Owned,
                        access: None,
                    }),
                    Some(dir::LanguageItem::Raw) => Some(ReceiverForm {
                        ownership: dir::Ownership::Raw,
                        access: None,
                    }),
                    Some(dir::LanguageItem::Borrowed) => {
                        let access = match arguments.get(2) {
                            Some(access) => self.written_access(*access)?,
                            None => None,
                        };

                        Some(ReceiverForm {
                            ownership: dir::Ownership::Borrowed,
                            access,
                        })
                    }
                    Some(dir::LanguageItem::Managed | dir::LanguageItem::Readonly) => {
                        Some(ReceiverForm::MANAGED)
                    }
                    Some(
                        dir::LanguageItem::Placed
                        | dir::LanguageItem::WithBase
                        | dir::LanguageItem::WithOwnership
                        | dir::LanguageItem::WithPlace
                        | dir::LanguageItem::WithSpace
                        | dir::LanguageItem::WithLifetime
                        | dir::LanguageItem::WithAccess,
                    ) => match arguments.first() {
                        Some(inner) => self.receiver_form(*inner)?,
                        None => None,
                    },
                    // keep conversion receivers out of the plain slot
                    Some(_) => None,
                    None => Some(ReceiverForm::MANAGED),
                }
            }
            _ => Some(ReceiverForm::MANAGED),
        };

        Ok(form)
    }

    /// Return the access one access type writes, open for an access parameter.
    fn written_access(&mut self, access: dir::GlobalTypeId) -> CompilerResult<Option<dir::Access>> {
        Ok(match self.ty(access)? {
            dir::Type::Memory(dir::MemoryLiteral::Access(access)) => Some(access),
            _ => None,
        })
    }
}

/// One keyed member compared for duplicates across visible extensions.
struct DeclaredMember {
    /// The member key.
    key: dir::StaticKey,
    /// The member space declaring the member.
    space: dir::MemberSpace,
    /// The receiver form the member takes.
    form: ReceiverForm,
    /// The declaring member source node.
    source: dir::GlobalNodeIdAny,
}

/// The ownership and access one member receiver takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::sema) struct ReceiverForm {
    /// The receiver ownership.
    pub(in crate::sema) ownership: dir::Ownership,
    /// The borrowed access, open for an access parameter.
    pub(in crate::sema) access: Option<dir::Access>,
}

impl ReceiverForm {
    /// The family-default managed receiver.
    pub(in crate::sema) const MANAGED: Self = Self {
        ownership: dir::Ownership::Managed,
        access: None,
    };

    /// Return whether two receiver forms admit one common receiver.
    pub(in crate::sema) fn overlaps(self, other: Self) -> bool {
        self.ownership == other.ownership
            && match (self.access, other.access) {
                (Some(left), Some(right)) => left == right,
                _ => true,
            }
    }
}
