use smallvec::SmallVec;
use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    CheckState, ExtensionCoherenceObligation, ExtensionHead, ImplementationCoherenceObligation,
    ObligationCheck, ObligationFailure, Origin, Relation,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Report the member conflicts inside every definition the checked module declares.
    pub(in crate::sema) fn report_member_conflicts(&mut self) -> CompilerResult<()> {
        // collect the definitions this module declares
        let module = self.module_id;
        let definitions = self
            .module
            .iter_definitions()
            .filter(|(symbol, _)| symbol.module_id == module)
            .map(|(_, definition)| definition.clone())
            .collect::<Vec<_>>();

        // report the conflicts inside each of them
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
        // track the keys seen so far and the overloads recorded under each
        let mut seen = FxIndexMap::<(dir::MemberSpace, dir::StaticKey), bool>::default();
        let mut overloads =
            FxIndexMap::<(dir::MemberSpace, dir::StaticKey), Vec<dir::GlobalSymbolId>>::default();

        // report each member whose key repeats without overloading
        for member in definition.members() {
            // read the key this member declares
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

            // limit the overload check to methods without a role
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
                {
                    let mut has_type_parameter = false;
                    for parameter in self.generic_template_parameters(template)? {
                        has_type_parameter |= self
                            .generic_parameter(parameter)?
                            .is_none_or(|binding| binding.memory_parameter().is_none());
                    }
                    if has_type_parameter {
                        continue;
                    }
                }

                // report the later overload once the earlier signature takes its calls
                let origin = Origin::Node(method.source, None);
                if self.has_every_arity_of(earlier, later)?
                    && self
                        .decide_relation(origin, Relation::Subtype, earlier, later)?
                        .holds()
                {
                    self.report_unreachable_overload(method.source, &key);
                    break;
                }
            }

            // keep this overload for the members that follow
            overloads.entry(entry).or_default().push(method.symbol);
        }

        Ok(())
    }

    /// Return whether one signature takes every call arity another signature takes.
    fn has_every_arity_of(
        &mut self,
        earlier: dir::GlobalTypeId,
        later: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // require both sides to be signatures
        let (Some(earlier_head), Some(later_head)) =
            (self.signature_head(earlier)?, self.signature_head(later)?)
        else {
            return Ok(false);
        };

        // read both parameter lists
        let earlier = self.signature_parameters(earlier.module_id, earlier_head.parameters)?;
        let later = self.signature_parameters(later.module_id, later_head.parameters)?;

        // count the parameters every call must pass
        let required = |parameters: &[dir::FunctionParameterType]| {
            parameters
                .iter()
                .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
                .count()
        };

        // note which side absorbs the trailing arguments
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
        // read the extension this obligation names
        let source = obligation.source;
        let symbol = obligation.symbol;
        let definition = self.definition(symbol)?;
        let Some(dir::Definition::Extension(extension)) = definition.as_deref() else {
            return Err(CompilerError::Internal {
                message: format!("extension obligation has no extension definition: {symbol:?}"),
            });
        };

        // read the target it extends and the interfaces it implements
        let target = extension.target;
        let form = extension.form;
        let interfaces = self
            .declared_interfaces(symbol)?
            .into_iter()
            .collect::<SmallVec<[_; 2]>>();

        // collect the failures against the module of the extension
        let module = source.module_id;
        let mut failures = Vec::new();

        // reject anonymous exported extensions on nonlocal targets
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

        // check the implemented pairs against the package of the extension
        let package = module.package_id;
        match target {
            // reject headed implementation pairs outside both packages
            dir::ExtensionTarget::Rooted { root, .. } => {
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

                // reject the pairs an earlier visible implementation already covers
                let conflicts = self.check_conflicting_implementations(
                    origin,
                    module,
                    source,
                    symbol,
                    target,
                    &interfaces,
                )?;
                failures.extend(conflicts);
            }
            // require blanket implementations beside their interface
            dir::ExtensionTarget::Blanket { .. } => {
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

                // reject the pairs an earlier visible implementation already covers
                let conflicts = self.check_conflicting_implementations(
                    origin,
                    module,
                    source,
                    symbol,
                    target,
                    &interfaces,
                )?;
                failures.extend(conflicts);
            }
        }

        let check = ObligationCheck::from_failures(failures);

        Ok(check)
    }

    /// Return whether the constraint of one blanket target anchors an interface.
    fn is_blanket_interface_anchored(&mut self, target: dir::GlobalTypeId) -> CompilerResult<bool> {
        // read the constraint the blanket parameter declares
        let dir::Type::Parameter(parameter) = self.ty(target)? else {
            return Ok(false);
        };
        let Some(constraint) = self
            .generic_parameter(parameter)?
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
                self.definition(interface.symbol)?.as_deref()
            {
                admitted.extend(definition.members.iter().filter_map(|member| member.key()));
            }
        }

        // read the keys the extension declares
        let definition = self.definition(symbol)?;
        let Some(dir::Definition::Extension(extension)) = definition.as_deref() else {
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
        // skip extensions that declare no generic parameters
        let Some(template) = self.symbol_template(symbol)? else {
            return Ok(());
        };

        // visit the parameters the headers constrain, then the ones their predicates refine
        let predicates = self.template_predicates(Some(template))?;
        let mut constrained = FxIndexSet::default();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 4]>::from_slice(interfaces);
        pending.push(target);
        while let Some(ty) = pending.pop() {
            // visit each parameter once, following the constraint it declares
            for parameter in self.type_parameters(ty)? {
                if constrained.insert(parameter) {
                    let binding = self.generic_parameter(parameter)?;
                    pending.extend(binding.and_then(|binding| binding.constraint));
                }
            }

            // visit the refinements a predicate over visited parameters binds
            if pending.is_empty() {
                for predicate in &predicates {
                    let left = self.type_parameters(predicate.left)?;
                    let is_constrained = !left.is_empty()
                        && left.iter().all(|parameter| constrained.contains(parameter));
                    if !is_constrained {
                        continue;
                    }
                    let (_, bindings) = self.refinements(predicate.right)?;
                    for (_, value) in bindings {
                        let is_open = self
                            .type_parameters(value)?
                            .iter()
                            .any(|parameter| !constrained.contains(parameter));
                        if is_open {
                            pending.push(value);
                        }
                    }
                }
            }
        }

        // reject declared type parameters outside the constrained set
        for parameter in self.generic_template_parameters(template)? {
            let Some(binding) = self.generic_parameter(parameter)?.cloned() else {
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
    pub(in crate::sema) fn type_parameters(
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
            // visit each type once
            if !visited.insert(id) {
                continue;
            }

            // record a parameter, else descend into the children
            let id = self.shallow_resolve(id)?;
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
        // limit the rule to exported extensions
        if form != dir::ExtensionForm::Exported {
            return Ok(false);
        }

        // accept a blanket extension, its bound interface names it
        if matches!(self.ty(target.r#type())?, dir::Type::Parameter(_)) {
            return Ok(false);
        }

        // accept a target the checked module declares itself
        let target_is_local = target
            .declaration()
            .is_some_and(|root| root.module_id == module);
        if target_is_local {
            return Ok(false);
        }

        // require a written name on everything else
        let is_unnamed = self
            .binding_table(symbol.module_id)?
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
        // prefer this module's authored implementations over a foreign definition
        if let Some(module) = self.module_maybe(symbol.module_id)
            && let Some(declared) = &module.declared
            && let Some(definition) = declared.definitions.definition(symbol)
        {
            let interfaces = definition
                .implementations()
                .map(|conformance| conformance.interface)
                .collect();

            return Ok(interfaces);
        }

        // read the conformances off the resolved definition
        let Some(definition) = self.definition(symbol)? else {
            return Err(CompilerError::Internal {
                message: format!("implementation has no definition: {symbol:?}"),
            });
        };

        // read the interfaces the conformances name
        let interfaces = definition
            .implementations()
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
        target: dir::ExtensionTarget,
        interfaces: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<ObligationFailure>> {
        // collect the failures against the extended type
        let ty = target.r#type();
        let mut failures = Vec::new();

        // collect earlier visible implementations sharing one declared interface
        let mut candidates = SmallVec::<[(dir::GlobalSymbolId, dir::GlobalTypeId); 2]>::new();
        for interface_type in interfaces {
            let (_, interface) = self.nominal_application(*interface_type)?;
            for (other, other_root) in
                self.implementations_over(origin, module, None, interface.symbol)?
            {
                // keep the implementations declared earlier than this one
                if other == symbol || !self.is_later_definition(source, other) {
                    continue;
                }

                // skip rooted pairs over distinct roots, no type inhabits both targets
                if let dir::ExtensionTarget::Rooted { root, .. } = target
                    && let Some(other_root) = other_root
                    && other_root != root
                {
                    continue;
                }

                candidates.push((other, *interface_type));
            }
        }

        // reject implementations some declared type satisfies together with this one
        for (other, interface_type) in candidates {
            let overlap =
                self.extension_implementations_overlap(origin, symbol, ty, interface_type, other)?;
            let Some(witness) = overlap else {
                continue;
            };

            // name the interface both implementations share
            let (_, interface) = self.nominal_application(interface_type)?;
            failures.push(ObligationFailure::ConflictingImplementation {
                source,
                conflict: other,
                interface: interface.symbol,
                witness,
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
        // treat a definition from outside the checked modules as earlier
        let Some(state) = self.module_maybe(other.module_id) else {
            return true;
        };

        // treat a definition without a source as earlier
        let Some(other_source) = state.definition_source_maybe(other) else {
            return true;
        };

        // treat a definition from another module as earlier
        if other_source.module_id != source.module_id {
            return true;
        }

        source.local_id.id > other_source.local_id.id
    }
}

impl CheckState<'_> {
    /// Check one extension against other visible extensions' members.
    pub(in crate::sema) fn check_extension_coherence(
        &mut self,
        obligation: &ExtensionCoherenceObligation,
    ) -> CompilerResult<ObligationCheck> {
        // read the extension being checked
        let extension_symbol = obligation.symbol;
        let module = extension_symbol.module_id;
        let definition = self.definition(extension_symbol)?;
        let Some(dir::Definition::Extension(extension)) = definition.as_deref() else {
            return Err(CompilerError::Internal {
                message: format!(
                    "extension coherence has no extension definition: {extension_symbol:?}"
                ),
            });
        };

        // collect the failures against this extension
        let extension = extension.clone();
        let mut failures = Vec::new();

        // collect the inherent members this extension declares
        let requirements = self.requirement_keys(extension.implementations())?;
        let target_form = self
            .receiver_form(extension.target.r#type())?
            .unwrap_or(ReceiverForm::MANAGED);
        let declared = self.keyed_members(&extension.members, target_form, &requirements)?;

        // stop where the extension declares no inherent member
        if declared.is_empty() {
            return Ok(ObligationCheck::holds());
        }

        // render the target once for the failure reports
        let target = self.format_type(extension.target.r#type());

        // gather competitors sharing the target head, leaving blanket overlap to use sites
        let root = extension.target.root();
        let competitors = match root {
            Some(root) => self.extensions_over(module, &[ExtensionHead::Root(root)])?,
            None => return Ok(ObligationCheck::holds()),
        };

        // report every member the root declaration itself declares
        if let Some(dir::TypeRoot::Declaration(declaration)) = root
            && let Some(definition) = self.definition(declaration)?
        {
            let members = definition.members();
            let inherent =
                self.keyed_members(members, ReceiverForm::MANAGED, &FxIndexSet::default())?;
            for member in &declared {
                let redeclared = inherent.iter().any(|candidate| {
                    candidate.key == member.key
                        && candidate.space == member.space
                        && candidate.form.is_overlapping(member.form)
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
                || !self.is_later_definition(obligation.source, competitor_symbol)
            {
                continue;
            }

            // require the competitor to extend the same target
            let definition = self.definition(competitor_symbol)?;
            let Some(dir::Definition::Extension(competitor)) = definition.as_deref() else {
                continue;
            };

            if competitor.target.root() != root {
                continue;
            }

            // read what the competitor declares
            let members = competitor.members.clone();
            let requirements = self.requirement_keys(competitor.implementations())?;
            let competitor_target = competitor.target.r#type();

            // report each declared member the competitor also declares inherently
            let competitor_form = self
                .receiver_form(competitor_target)?
                .unwrap_or(ReceiverForm::MANAGED);
            let other = self.keyed_members(&members, competitor_form, &requirements)?;
            for member in &declared {
                let duplicated = other.iter().any(|candidate| {
                    candidate.key == member.key
                        && candidate.space == member.space
                        && candidate.form.is_overlapping(member.form)
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
    fn requirement_keys<'a>(
        &mut self,
        implements: impl IntoIterator<Item = &'a dir::NominalConformance>,
    ) -> CompilerResult<FxIndexSet<dir::StaticKey>> {
        let mut keys = FxIndexSet::default();
        for conformance in implements {
            // read the interface this conformance names
            let Some((_, interface)) = self.nominal_application_maybe(conformance.interface)?
            else {
                continue;
            };

            let declared = self.definition(interface.symbol)?;
            let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
                continue;
            };

            // take every key the interface requires
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
            // keep the keyed members no interface already requires
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
                    let borrow = self.type_borrow(this.module_id, borrow)?;

                    Some(ReceiverForm {
                        ownership: dir::Ownership::Borrowed,
                        access: self.access_set(borrow.access)?,
                    })
                }
                dir::Form::Owned => Some(ReceiverForm {
                    ownership: dir::Ownership::Owned,
                    access: AccessSet::ALL,
                }),
                dir::Form::Raw => Some(ReceiverForm {
                    ownership: dir::Ownership::Raw,
                    access: AccessSet::ALL,
                }),
                dir::Form::Readonly => Some(ReceiverForm::MANAGED),
            },
            // written memory applications compare like the forms they name
            dir::Type::Application(instance) => {
                let arguments = self.type_ids(this.module_id, instance.arguments)?;
                match self.language_item(instance.symbol)? {
                    Some(dir::LanguageItem::Owned) => Some(ReceiverForm {
                        ownership: dir::Ownership::Owned,
                        access: AccessSet::ALL,
                    }),
                    Some(dir::LanguageItem::Raw) => Some(ReceiverForm {
                        ownership: dir::Ownership::Raw,
                        access: AccessSet::ALL,
                    }),
                    Some(dir::LanguageItem::Borrowed) => {
                        let access = match arguments.get(2) {
                            Some(access) => self.access_set(*access)?,
                            None => AccessSet::ALL,
                        };

                        Some(ReceiverForm {
                            ownership: dir::Ownership::Borrowed,
                            access,
                        })
                    }
                    Some(dir::LanguageItem::Readonly) => Some(ReceiverForm::MANAGED),
                    // read the form through the access application
                    Some(dir::LanguageItem::WithAccess) => match arguments.first() {
                        Some(underlying) => self.receiver_form(*underlying)?,
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

    /// Return the accesses one access term admits, every access while it stays open.
    pub(in crate::sema) fn access_set(
        &mut self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<AccessSet> {
        let access = self.shallow_resolve(access)?;

        Ok(match self.ty(access)? {
            // admit the literal itself
            dir::Type::Literal(dir::Literal::String(text)) => {
                match dir::Access::from_text(self.strings().get(text)) {
                    Some(access) => AccessSet::of(access),
                    None => {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "an access literal outside the access set: {access:?}"
                            ),
                        });
                    }
                }
            }
            // admit what every bound on a parameter admits
            dir::Type::Parameter(parameter) => {
                let mut admitted = AccessSet::ALL;
                for bound in self.declared_parameter_bounds(parameter)? {
                    admitted = admitted.intersection(self.access_set(bound)?);
                }

                admitted
            }
            // admit every alternative of a union
            dir::Type::Union(union) => {
                let mut admitted = AccessSet::NONE;
                for element in self.type_ids(access.module_id, union.elements)? {
                    admitted = admitted.union(self.access_set(*element)?);
                }

                admitted
            }
            // admit either branch of a conditional
            dir::Type::Operation(operation)
                if let dir::TypeOperation::Conditional(conditional) =
                    self.type_operation(access.module_id, operation)? =>
            {
                self.access_set(conditional.then_type)?
                    .union(self.access_set(conditional.else_type)?)
            }
            // grant the empty access set at `never`
            dir::Type::Never => AccessSet::NONE,
            // read a named access domain through its alias
            dir::Type::Reference(dir::TypeReference { symbol, .. })
            | dir::Type::Application(dir::GenericApplication { symbol, .. })
                if let symbol = self.resolve_symbol_alias(symbol)?
                    && let Some(dir::Definition::TypeAlias(alias)) =
                        self.definition(symbol)?.as_deref() =>
            {
                self.access_set(alias.value)?
            }
            // admit every access at an open or failed term
            dir::Type::Variable(_) | dir::Type::Error => AccessSet::ALL,
            // fail on every other head
            _ => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "an access term outside the access domain: {}",
                        self.format_type(access)
                    ),
                });
            }
        })
    }
}

/// The set of access rungs one receiver admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct AccessSet(u8);

impl AccessSet {
    /// No rung.
    const NONE: Self = Self(0);
    /// Every rung.
    pub(in crate::sema) const ALL: Self = Self((1 << dir::Access::ALL.len()) - 1);

    /// The set of one rung.
    fn of(access: dir::Access) -> Self {
        Self(1 << access as u8)
    }

    /// Join two sets.
    fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Meet two sets.
    fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Return whether two sets share a rung.
    fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Return whether the set admits one rung.
    pub(in crate::sema) fn contains(self, access: dir::Access) -> bool {
        self.0 & (1 << access as u8) != 0
    }

    /// Return whether the set admits every rung.
    pub(in crate::sema) fn is_all(self) -> bool {
        self == Self::ALL
    }
}

/// One keyed member compared for duplicates across visible extensions.
struct DeclaredMember {
    /// The member key.
    key: dir::StaticKey,
    /// The space the member is declared in.
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
    /// The borrowed accesses the receiver admits.
    pub(in crate::sema) access: AccessSet,
}

impl ReceiverForm {
    /// The owned value receiver.
    pub(in crate::sema) const OWNED: Self = Self {
        ownership: dir::Ownership::Owned,
        access: AccessSet::ALL,
    };

    /// The default managed receiver.
    pub(in crate::sema) const MANAGED: Self = Self {
        ownership: dir::Ownership::Managed,
        access: AccessSet::ALL,
    };

    /// Return whether two receiver forms share one common receiver.
    pub(in crate::sema) fn is_overlapping(self, other: Self) -> bool {
        self.ownership == other.ownership && self.access.intersects(other.access)
    }
}
