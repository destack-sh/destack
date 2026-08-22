use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Answer, Ask, BodyState, CandidateOutcome, Cause, CauseKind, CheckOutcome, DeclaredMember,
    ExtensionSource, GenericParameterId, GenericTemplateId, Implementation, LookupReceiver,
    MemberCandidate, MemberLookup, Origin, ReceiverSteps, Relation, RelationCheck, Settle,
    TypeArgumentInference, TypeSubstitution, VariableRole, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// How one subject match treats parameters the target leaves unbound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum UnboundParameters {
    /// Open inference variables for the unbound parameters.
    Open,
    /// Reject the match while any parameter stays unbound.
    Reject,
}

/// How one extension match treats a bound that is still open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum OpenBounds {
    /// Probe the bound against the receiver's known bounds, disproving the candidate it fails.
    Probe,
    /// Keep the bound open, so only a closed bound disproves the candidate.
    Keep,
}

/// The polarity-aware outcome of matching one extension implementation.
#[derive(Debug, Clone)]
pub(in crate::sema) enum ExtensionMatch {
    /// The extension applies with this substitution and implementation.
    Matched(Box<TypeSubstitution>, dir::GlobalTypeId),
    /// The match is disproved: no instantiation can ever apply.
    Unmatched,
    /// The match stays undecided while a bound or operand is open.
    Unproven,
}

impl BodyState<'_, '_> {
    /// Look up one extension member on a declaration reference.
    pub(in crate::sema) fn lookup_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        root: dir::TypeRoot,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let extensions = self.reachable_extensions(origin, module, receiver, subject, root)?;

        // decide each extension once per subject identity, first declaration per symbol wins
        let mut candidates = Vec::new();
        let mut is_undecided = false;
        let mut seen = FxIndexSet::default();
        for extension in extensions {
            // consult a blanket only when it can expose the looked-up key
            if !self.extension_exposes_key(extension, key)? {
                continue;
            }

            let Some(matched) = self.extension_subject_candidates(
                origin,
                module,
                root,
                receiver,
                subject,
                extension,
                space,
                Some(key),
            )?
            else {
                // report the coinductive re-entry as undecided
                is_undecided = true;
                continue;
            };
            for (candidate_key, candidate) in matched.iter() {
                if *candidate_key != key {
                    continue;
                }
                if !seen.insert(candidate.symbol) {
                    continue;
                }

                candidates.push(candidate.clone());
            }
        }

        // answer undecided when re-entry emptied the candidates
        if candidates.is_empty() && is_undecided {
            return Ok(MemberLookup::Undecided);
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Decide every extension matching one settled subject pair with its deduced arguments.
    pub(in crate::sema) fn decided_extension_sources(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        root: dir::TypeRoot,
    ) -> CompilerResult<Vec<ExtensionSource>> {
        let canonical = self
            .check
            .canonicalize(origin, [receiver, subject], false)?;
        let question = match &canonical {
            Some(canonical) => Some(
                self.check
                    .question(Ask::Sources { root, module }, canonical)?,
            ),
            None => None,
        };

        // replay the composed answer at this ask's live roots
        if let Some(question) = &question
            && let Some(canonical) = &canonical
            && let Some(Answer::Sources(response)) = self.check.answers.get(question).cloned()
        {
            return self
                .check
                .instantiate_response(origin, canonical, &response);
        }

        // decide each reachable extension once, replaying decided matches
        let composed_before = self.check.fulfill.checks.count();
        let extensions = self.reachable_extensions(origin, module, receiver, subject, root)?;
        let mut sources = Vec::new();
        for extension in extensions {
            // admit the present extensions visible from the asking module
            if self.is_absent_symbol(extension) || !self.extension_is_visible(extension, module)? {
                continue;
            }

            // replay the extension's own canonical decision
            let extension_question = match &canonical {
                Some(canonical) => Some(
                    self.check
                        .question(Ask::Extension { extension }, canonical)?,
                ),
                None => None,
            };
            if let Some(extension_question) = &extension_question
                && let Some(canonical) = &canonical
                && let Some(Answer::Extension(stored)) =
                    self.check.answers.get(extension_question).cloned()
            {
                if let Some(response) = stored {
                    let arguments = self
                        .check
                        .instantiate_response(origin, canonical, &response)?;
                    sources.push(ExtensionSource {
                        extension,
                        arguments,
                    });
                }

                continue;
            }

            // decide the match live
            let checks_before = self.check.fulfill.checks.count();
            let arguments =
                self.match_extension_arguments(origin, module, receiver, subject, extension)?;

            // remember the deduction folded canonical over its ask
            match &arguments {
                Some(arguments) => {
                    if let (Some(extension_question), Some(canonical)) =
                        (&extension_question, &canonical)
                    {
                        self.check.remember_answer(
                            extension_question,
                            canonical,
                            checks_before,
                            arguments.clone(),
                            |response| Answer::Extension(Some(response)),
                        )?;
                    }
                }
                // remember a refusal taken outside re-entrant collection
                None if self.check.extending.is_empty() => {
                    if let Some(extension_question) = extension_question
                        && !self.check.answers.contains_key(&extension_question)
                    {
                        self.check
                            .answers
                            .insert(extension_question, Answer::Extension(None));
                    }
                }
                // leave a refusal reached under a cycle guard undecided
                None => {}
            }

            if let Some(arguments) = arguments {
                sources.push(ExtensionSource {
                    extension,
                    arguments,
                });
            }
        }

        // remember the composed answer folded canonical over its ask
        if let (Some(question), Some(canonical)) = (&question, &canonical) {
            self.check.remember_answer(
                question,
                canonical,
                composed_before,
                sources.clone(),
                Answer::Sources,
            )?;
        }

        Ok(sources)
    }

    /// Match one extension against a settled subject pair, deducing its template arguments.
    fn match_extension_arguments(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<SmallVec<[dir::GlobalTypeId; 4]>>> {
        // require a visible declaration with a template to deduce
        if self.is_absent_symbol(extension_symbol) {
            return Ok(None);
        }
        let target_type = match self.definition(extension_symbol)? {
            Some(dir::Definition::Extension(extension)) if extension.is_visible_from(module) => {
                extension.target.r#type()
            }
            _ => return Ok(None),
        };

        // hold the re-entry mark unless an enclosing collection already does
        let is_guarding = self.check.extending.insert((extension_symbol, subject));

        // match the receiver once, erasing the instantiations this probe left undeduced
        let template = self.symbol_template(extension_symbol)?;
        let own_module = self.check.module_id;
        let baseline = self.check.infer.variable_count();
        let is_settled =
            !(self.check.type_flags(receiver)? | self.check.type_flags(subject)?).has_variable();
        let deduced = self.probe_deduction(|state| {
            let Some(substitution) =
                state.match_extension(origin, receiver, subject, template, target_type)?
            else {
                return Ok(CandidateOutcome::Accepted(()));
            };

            // solve the declared bounds of settled pairs, binding their induced parameters
            if is_settled && let Some(template) = template {
                let constraints =
                    state.substitute_application_constraints(origin, template, &substitution)?;
                for constraint in constraints {
                    let id = state.check.register_relation(constraint)?;
                    state.check.solve_relation(id, Settle::Complete)?;
                }
            }

            let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
            if let Some(template) = template {
                for parameter in state.generic_template_parameters(template)? {
                    let argument = match substitution.argument(parameter) {
                        Some(argument) => {
                            let mut argument = state.check.shallow_resolve(argument)?;
                            for variable in state.check.type_variables(argument)? {
                                if (variable.0 as usize) < baseline {
                                    continue;
                                }
                                let VariableRole::Instantiation { parameter } =
                                    state.check.infer.variable_role(variable)?
                                else {
                                    continue;
                                };
                                let from = state.intern_type(dir::Type::Variable(variable))?;
                                let to = state.intern_type(dir::Type::Erased(parameter))?;
                                argument =
                                    state.check.replace_type(own_module, argument, from, to)?;
                            }

                            argument
                        }
                        None => state.intern_type(dir::Type::Erased(parameter))?,
                    };
                    arguments.push(argument);
                }
            }
            Ok(CandidateOutcome::Rejected(arguments))
        })?;
        if is_guarding {
            self.check
                .extending
                .swap_remove(&(extension_symbol, subject));
        }

        Ok(deduced)
    }

    /// Collect the erased parameters one type graph mentions.
    fn erased_parameters(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalGenericParameterId; 2]>> {
        let mut erased = SmallVec::new();
        if !self.type_flags(id)?.has_parameter() {
            return Ok(erased);
        }
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        pending.push(id);
        while let Some(id) = pending.pop() {
            if !self.type_flags(id)?.has_parameter() {
                continue;
            }
            let ty = self.ty(id)?;
            if let dir::Type::Erased(parameter) = ty {
                if !erased.contains(&parameter) {
                    erased.push(parameter);
                }
                continue;
            }
            self.check
                .for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(erased)
    }

    /// Return whether one subject's nominal head matches a concrete target head.
    ///
    /// Assignability into a non-interface nominal flows through declared heritage.
    fn root_matches_target(
        &mut self,
        subject: dir::GlobalTypeId,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // interface and alias targets admit every head
        let Some(target_symbol) = self.nominal_head(target_type)? else {
            return Ok(true);
        };
        if matches!(
            self.definition(target_symbol)?,
            None | Some(dir::Definition::Interface(_) | dir::Definition::TypeAlias(_))
        ) {
            return Ok(true);
        }

        // concrete targets admit matching heads and declared heritage
        match self.nominal_head(subject)? {
            Some(subject_symbol) => Ok(subject_symbol == target_symbol
                || self.reaches_heritage(subject_symbol, target_symbol)?),
            None => Ok(true),
        }
    }

    /// Return the closed nominal head symbol of one type, if it has one.
    fn nominal_head(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let ty = self.check.shallow_resolve(ty)?;

        Ok(match self.check.ty(ty)? {
            dir::Type::Application(instance) => Some(instance.symbol),
            dir::Type::Reference(reference) => Some(reference.symbol),
            _ => None,
        })
    }

    /// Return the interface whose requirement one extension exposes under a member key.
    pub(in crate::sema) fn requirement_interface(
        &mut self,
        extension: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // serve the memo
        if let Some(requirements) = self.check.requirement_interfaces.get(&extension) {
            return Ok(requirements.get(&key).copied());
        }

        // map each implemented interface's member keys to that interface
        let mut requirements = FxIndexMap::default();
        if matches!(
            self.definition(extension)?,
            Some(dir::Definition::Extension(_))
        ) {
            let interfaces = self.check.declared_interfaces(extension)?;
            for implemented in interfaces {
                let Some((_, interface)) = self.nominal_application_maybe(implemented)? else {
                    continue;
                };
                let Some(dir::Definition::Interface(declared)) =
                    self.definition(interface.symbol)?
                else {
                    continue;
                };
                for member in &declared.members {
                    if let Some(key) = member.key() {
                        requirements.entry(key).or_insert(interface.symbol);
                    }
                }
            }
        }
        let requirement = requirements.get(&key).copied();

        // memoize complete definitions only, declaration-time members are still arriving
        if !self.is_declaration() {
            self.check
                .requirement_interfaces
                .insert(extension, requirements);
        }

        Ok(requirement)
    }

    /// Return whether one extension can expose one member key.
    ///
    /// Rooted extensions are already scoped by their target root; a blanket exposes its declared
    /// keys and its interfaces' member keys.
    fn extension_exposes_key(
        &mut self,
        extension: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<bool> {
        if let Some(keys) = self.check.blanket_keys.get(&extension) {
            return Ok(keys.contains(&key));
        }

        // read the blanket's declared keys with its implemented interfaces
        let mut interfaces = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        let mut keys = FxIndexSet::default();
        match self.definition(extension)? {
            Some(dir::Definition::Extension(definition)) if definition.target.is_blanket() => {
                for member in &definition.members {
                    let Some(declared) = member.key() else {
                        return Ok(true);
                    };
                    keys.insert(declared);
                }
                interfaces.extend(
                    definition
                        .implements
                        .iter()
                        .map(|conformance| conformance.interface),
                );
            }
            _ => return Ok(true),
        }

        // admit every member key the implemented interfaces declare
        for implemented in interfaces {
            let Some((_, interface)) = self.nominal_application_maybe(implemented)? else {
                continue;
            };
            let Some(dir::Definition::Interface(definition)) = self.definition(interface.symbol)?
            else {
                continue;
            };
            for member in &definition.members {
                let Some(key) = member.key() else {
                    return Ok(true);
                };
                keys.insert(key);
            }
        }
        let exposes = keys.contains(&key);

        // memoize complete definitions only: declaration-time members are still arriving
        if !self.is_declaration() {
            self.check.blanket_keys.insert(extension, keys);
        }

        Ok(exposes)
    }

    /// Return whether one extension declaration is visible from one module.
    fn extension_is_visible(
        &mut self,
        extension: dir::GlobalSymbolId,
        module: ModuleId,
    ) -> CompilerResult<bool> {
        let Some(dir::Definition::Extension(definition)) = self.definition(extension)? else {
            return Ok(false);
        };

        Ok(definition.is_visible_from(module))
    }

    /// Collect the extension declarations one subject pair can reach from one module.
    pub(in crate::sema) fn reachable_extensions(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        root: dir::TypeRoot,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // collect the extension declarations this module can see
        let mut extensions = self.visible_extensions(module, root)?;

        // structural receivers also reach extensions declared over their constructor
        for ty in [receiver, subject] {
            let value = self.strip_form(origin, ty)?;
            let Some(structural_root) = self.structural_root(value)? else {
                continue;
            };
            if structural_root == root {
                continue;
            }

            for symbol in self.visible_extensions(module, structural_root)? {
                if !extensions.contains(&symbol) {
                    extensions.push(symbol);
                }
            }
        }

        // formed receivers also reach extensions rooted at their form constructors
        for ty in [receiver, subject] {
            let mut payload = self.shallow_resolve(ty)?;
            while let dir::Type::Form(form) = self.ty(payload)? {
                let form_root = dir::TypeRoot::Declaration(
                    self.check.language_symbol(form.form.language_item())?,
                );
                if form_root != root {
                    for symbol in self.visible_extensions(module, form_root)? {
                        if !extensions.contains(&symbol) {
                            extensions.push(symbol);
                        }
                    }
                }
                payload = self.shallow_resolve(form.value)?;
            }
        }

        Ok(extensions)
    }

    /// Look up one static extension member on a declaration reference.
    pub(super) fn lookup_static_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // collect the extension declarations this module can see
        let extensions = self.visible_extensions(module, dir::TypeRoot::Declaration(symbol))?;
        let mut candidates = Vec::new();
        let mut is_undecided = false;
        let mut seen = FxIndexSet::default();

        // visit extension declarations in resolution order
        for extension_symbol in extensions {
            // consult a blanket only when it can expose the looked-up key
            if !self.extension_exposes_key(extension_symbol, key)? {
                continue;
            }

            let lookup = self.lookup_one_static_extension(
                origin,
                module,
                symbol,
                arguments,
                extension_symbol,
                key,
            )?;

            let found = match lookup {
                MemberLookup::Found(found) => found,
                MemberLookup::Missing | MemberLookup::Field(_) => continue,
                MemberLookup::Undecided => {
                    // report the coinductive re-entry as undecided
                    is_undecided = true;
                    continue;
                }
                MemberLookup::Union(_) | MemberLookup::Intersection(_) => {
                    return Err(CompilerError::Internal {
                        message: "one static extension produced a union member lookup".to_string(),
                    });
                }
            };

            for candidate in found {
                if !seen.insert(candidate.symbol) {
                    continue;
                }

                candidates.push(candidate);
            }
        }

        // answer undecided when re-entry emptied the candidates
        if candidates.is_empty() && is_undecided {
            return Ok(MemberLookup::Undecided);
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Collect extension symbols visible from one module for one target.
    ///
    /// NOTE #Robustness: the memoized set reads lazily imported modules and the
    /// definitions tail, so every pass must import its externals before the first ask.
    pub(in crate::sema) fn visible_extensions(
        &mut self,
        module: ModuleId,
        root: dir::TypeRoot,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // serve the memo
        if let Some(symbols) = self.check.extension_sets.get(&(module, root)) {
            return Ok(symbols.clone());
        }

        let mut symbols = SmallVec::new();

        // collect extensions declared beside the looking module
        if let Some(state) = self.module_maybe(module) {
            symbols.extend(state.root_extensions(root));
            symbols.extend(state.blanket_extensions());
        }

        // collect inherent extensions beside the head declaration
        if let dir::TypeRoot::Declaration(target) = root
            && target.module_id != module
        {
            if let Some(state) = self.module_maybe(target.module_id) {
                symbols.extend(state.root_extensions(root));
                symbols.extend(state.blanket_extensions());
            }

            if let Some(external) = self.external_modules.get(&target.module_id) {
                symbols.extend(external.definitions.root_extensions(root));
                symbols.extend(external.definitions.blanket_extensions());
            }
        }

        // collect the implicit extensions declared over the head
        if let Some(environment) = &self.check.environment_declared
            && let Some(implicit) = environment.extensions_by_root.get(&root)
        {
            for symbol in implicit {
                if !symbols.contains(symbol) {
                    symbols.push(*symbol);
                }
            }
        }

        // collect the implicit open blanket extensions
        if let Some(environment) = &self.check.environment_declared {
            for symbol in &environment.blanket_extensions {
                if !symbols.contains(symbol) {
                    symbols.push(*symbol);
                }
            }
        }

        // collect explicitly imported extension symbols
        let imported = self
            .module(module)
            .resolved
            .imports
            .symbol_targets()
            .map(|(_, symbol)| symbol)
            .collect::<SmallVec<[_; 8]>>();
        for symbol in imported {
            if self
                .definition(symbol)?
                .is_some_and(|definition| matches!(definition, dir::Definition::Extension(_)))
                && !symbols.contains(&symbol)
            {
                symbols.push(symbol);
            }
        }

        // decide the visible set once per head, after every declaration has landed
        if !self.is_declaration() {
            self.check
                .extension_sets
                .insert((module, root), symbols.clone());
        }

        Ok(symbols)
    }

    /// Collect the declared members of one extension matching a selection key.
    pub(in crate::sema) fn matching_extension_members(
        &mut self,
        members: &[dir::DefinitionMember],
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<SmallVec<[DeclaredMember; 2]>> {
        let mut matched = SmallVec::new();
        for member in members {
            // match the declared key before resolving the member type
            if member.space() != space || member.key() != Some(key) {
                continue;
            }
            let Some(member) = self.declared_member(member)? else {
                continue;
            };

            matched.push(member);
        }

        Ok(matched)
    }

    /// Decide whether a visible extension implements one interface for a receiver.
    pub(in crate::sema) fn decide_extension_implementation(
        &mut self,
        origin: Origin,
        relation: Relation,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericApplication,
        excluded: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Verdict> {
        // intern the requested interface application in this module
        let arguments: SmallVec<[_; 8]> =
            self.type_ids(interface_module, interface.arguments)?.into();
        let arguments = self.intern_type_ids(&arguments)?;
        let module = self.module_id;
        let mut instance = dir::GenericApplication {
            symbol: interface.symbol,
            arguments,
        };

        let mut instance_module = module;
        let mut interface_type = self.intern_type(dir::Type::Application(instance))?;

        // fill elided interface arguments before matching declared entries
        if let Some(filled) = self.fill_elided_application(interface_type.module_id, &instance)? {
            interface_type = filled;
            let (filled_module, filled_instance) = self.nominal_application(filled)?;
            instance_module = filled_module;
            instance = filled_instance;
        }

        // serve a repeated canonical goal from the memo, applying the stored hole solutions
        let asked = self.check.ask(
            origin,
            Ask::Implementation(relation),
            &[receiver, interface_type],
            true,
        )?;
        if excluded.is_none()
            && let Some((question, canonical)) = &asked
            && let Some(Answer::Implement(response)) = self.check.answers.get(question).cloned()
        {
            let implementation = self
                .check
                .instantiate_response(origin, canonical, &response)?;

            return Ok(implementation.verdict);
        }

        // replay the winner the receiver's owning module already decided
        let owner = match self.ty(receiver)? {
            dir::Type::Application(applied) if applied.arguments.is_empty() => Some(applied.symbol),
            dir::Type::Reference(reference) => Some(reference.symbol),
            _ => None,
        };
        let decided = owner
            .filter(|_| relation == Relation::Satisfies)
            .filter(|owner| !self.check.is_own_module(owner.module_id))
            .and_then(|owner| {
                let external = self.check.external_modules.get(&owner.module_id)?;
                let auto = external.auto.as_ref()?;

                auto.selected(owner, instance.symbol).flatten()
            });
        if excluded.is_none()
            && let Some(winner) = decided
            && self.extension_is_visible(winner, module)?
        {
            let replayed = self.confirm_extension(
                origin,
                relation,
                instance_module,
                receiver,
                &instance,
                winner,
            )?;
            if replayed {
                return Ok(Verdict::Holds);
            }
        }

        // break inductive applicability cycles: goals reached from themselves fail
        let active = (relation, receiver, interface_type);
        if !self.check.deciding.insert(active) {
            return Ok(Verdict::Fails);
        }

        // try each visible implementation declaration
        let checks_before = self.check.fulfill.checks.count();
        let decision = self.decide_visible_extensions(
            origin,
            relation,
            module,
            instance_module,
            receiver,
            &instance,
            excluded,
        );
        self.check.deciding.swap_remove(&active);

        // remember decided canonical goals folded over this ask
        let (decision, winner) = decision?;
        if excluded.is_none()
            && let Some((question, canonical)) = &asked
            && decision != Verdict::Ambiguous
        {
            self.check.remember_answer(
                question,
                canonical,
                checks_before,
                Implementation {
                    verdict: decision,
                    winner,
                },
                Answer::Implement,
            )?;
        }

        Ok(decision)
    }

    /// Confirm one known extension implementation against a goal.
    fn confirm_extension(
        &mut self,
        origin: Origin,
        relation: Relation,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericApplication,
        extension_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)? else {
            return Ok(false);
        };

        // read the declaration's target and implemented interfaces
        let interfaces = extension
            .implements
            .iter()
            .map(|conformance| conformance.interface)
            .collect::<SmallVec<[_; 2]>>();
        let target_type = extension.target.r#type();
        let template = self.symbol_template(extension_symbol)?;

        // replay the match, committing the inference it binds
        self.check.counters.extension_probes += 1;
        let matched = self.confirm_candidate(|state| {
            let matched = state.match_extension_implementation(
                origin,
                relation,
                interface_module,
                receiver,
                receiver,
                interface,
                template,
                target_type,
                &interfaces,
                OpenBounds::Probe,
            )?;
            Ok(match matched {
                ExtensionMatch::Matched(..) => CandidateOutcome::Accepted(()),
                ExtensionMatch::Unmatched | ExtensionMatch::Unproven => {
                    CandidateOutcome::Rejected(())
                }
            })
        })?;

        Ok(matched.is_some())
    }

    /// Decide whether any visible extension satisfies one applicability goal.
    fn decide_visible_extensions(
        &mut self,
        origin: Origin,
        relation: Relation,
        module: ModuleId,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericApplication,
        excluded: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<(Verdict, Option<dir::GlobalSymbolId>)> {
        // collect the implementation declarations this module can see
        let extensions =
            self.visible_implementation_extensions(origin, module, receiver, interface.symbol)?;

        // load each declaration once
        let mut entries = Vec::new();
        for extension_symbol in extensions {
            // skip the excluded declaration, absent gates, and invisible targets
            if Some(extension_symbol) == excluded || self.is_absent_symbol(extension_symbol) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)?
            else {
                continue;
            };
            if !extension.is_visible_from(module) {
                continue;
            }

            // read the target and the interfaces the declaration implements
            let target_type = extension.target.r#type();
            let interfaces = extension
                .implements
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();

            // keep only declarations whose conformances reach the requested interface
            if !self.implements_requested_interface(&interfaces, interface.symbol)? {
                continue;
            }

            let template = self.symbol_template(extension_symbol)?;
            entries.push((extension_symbol, template, target_type, interfaces));
        }

        // try each declaration, holding matches that solve outer variables
        let mut speculative = Vec::new();
        let mut is_unproven = false;
        for entry in &entries {
            let (extension_symbol, template, target_type, interfaces) = entry;
            let trail_from = self.check.infer.trail.len();
            let variables = self.check.infer.variable_count();
            let mut solves_outer = false;
            self.check.counters.extension_probes += 1;
            let matched = self.confirm_candidate(|state| {
                let matched = state.match_extension_implementation(
                    origin,
                    relation,
                    interface_module,
                    receiver,
                    receiver,
                    interface,
                    *template,
                    *target_type,
                    interfaces,
                    OpenBounds::Probe,
                )?;
                solves_outer = state
                    .check
                    .infer
                    .solves_variable_before(trail_from, variables);
                is_unproven |= matches!(matched, ExtensionMatch::Unproven);
                Ok(match (matched, solves_outer) {
                    (ExtensionMatch::Matched(..), false) => CandidateOutcome::Accepted(()),
                    _ => CandidateOutcome::Rejected(()),
                })
            })?;

            // coherence leaves one applicable implementation per concrete receiver
            if matched.is_some() {
                return Ok((Verdict::Holds, Some(*extension_symbol)));
            }

            // hold a match that solves outer variables until one candidate stands alone
            if solves_outer {
                speculative.push(entry);
            }
        }

        // a sole speculative candidate selects, committing its inference
        if let [(extension_symbol, template, target_type, interfaces)] = speculative.as_slice() {
            self.check.counters.extension_probes += 1;
            let matched = self.confirm_candidate(|state| {
                let matched = state.match_extension_implementation(
                    origin,
                    relation,
                    interface_module,
                    receiver,
                    receiver,
                    interface,
                    *template,
                    *target_type,
                    interfaces,
                    OpenBounds::Probe,
                )?;
                Ok(match matched {
                    ExtensionMatch::Matched(..) => CandidateOutcome::Accepted(()),
                    ExtensionMatch::Unmatched | ExtensionMatch::Unproven => {
                        CandidateOutcome::Rejected(())
                    }
                })
            })?;

            if matched.is_some() {
                return Ok((Verdict::Holds, Some(*extension_symbol)));
            }
        }

        // fail only on disproof: unproven candidates keep the goal open
        if speculative.is_empty() && !is_unproven {
            Ok((Verdict::Fails, None))
        } else {
            Ok((Verdict::Ambiguous, None))
        }
    }

    /// Return whether one declaration's conformances reach a requested interface.
    fn implements_requested_interface(
        &mut self,
        interfaces: &[dir::GlobalTypeId],
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        for implemented in interfaces {
            // a conformance written over a parameter or an alias reaches every interface
            let Some((_, instance)) = self.nominal_application_maybe(*implemented)? else {
                return Ok(true);
            };

            // follow the heritage above the named declaration
            if self.reaches_heritage(instance.symbol, interface)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Match one extension target and implemented interface.
    pub(in crate::sema) fn match_extension_implementation(
        &mut self,
        origin: Origin,
        relation: Relation,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        interface: &dir::GenericApplication,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
        interfaces: &[dir::GlobalTypeId],
        open_bounds: OpenBounds,
    ) -> CompilerResult<ExtensionMatch> {
        // require the receiver head to reach the declared target head
        if !self.root_matches_target(lookup_receiver, target_type)? {
            return Ok(ExtensionMatch::Unmatched);
        }

        // bind extension parameters through its declared target
        let parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };

        let mut substitution = TypeSubstitution::default().with_receiver(receiver);
        let matched = self.bind_extension_target(
            origin,
            &parameters,
            &mut substitution,
            target_type,
            lookup_receiver,
        )?;
        if !matched {
            return Ok(ExtensionMatch::Unmatched);
        }

        // open the parameters the target leaves unbound
        let Some(mut substitution) = self.instantiate_parameters(
            origin,
            &parameters,
            &[],
            substitution,
            TypeArgumentInference::Exact,
        )?
        else {
            return Ok(ExtensionMatch::Unmatched);
        };

        // require the lookup receiver to satisfy the applied target
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let target = self.substitute_type(target_type, &substitution)?;
        match self.constrain_type(origin, cause, Relation::Assignable, lookup_receiver, target)? {
            Verdict::Holds => {}
            Verdict::Ambiguous => return Ok(ExtensionMatch::Unproven),
            Verdict::Fails => return Ok(ExtensionMatch::Unmatched),
        }

        // match one declared interface with the requested interface
        let implementation = self.match_implemented_interface(
            origin,
            relation,
            interface_module,
            &parameters,
            &mut substitution,
            interfaces,
            interface,
        )?;
        let Some(implementation) = implementation else {
            return Ok(ExtensionMatch::Unmatched);
        };

        // prove the declared bounds and predicates for the candidate
        if let Some(template) = template {
            let constraints =
                self.substitute_application_constraints(origin, template, &substitution)?;
            for constraint in constraints {
                let closed = self.check.type_variables(constraint.source)?.is_empty()
                    && self.check.type_variables(constraint.target)?.is_empty();

                // decide a closed bound now, disproving an inapplicable candidate
                if closed {
                    let id = self.check.register_relation(constraint)?;
                    self.check.solve_relation(id, Settle::Complete)?;

                    // a disproved bound rejects, an undecided one stays unproven
                    match self.check.fulfill.checks.result(id)? {
                        Some(CheckOutcome::Holds) => {}
                        Some(_) => return Ok(ExtensionMatch::Unmatched),
                        None => return Ok(ExtensionMatch::Unproven),
                    }
                }
                // keep an open bound under overlap proving, where it may still hold
                else if open_bounds == OpenBounds::Keep {
                    self.check.register_relation(constraint)?;
                }
                // probe an open bound, keeping an undecided one open
                else {
                    let verdict = self.probe_candidate(|state| {
                        let id = state.check.register_relation(constraint)?;
                        state.check.solve_relation(id, Settle::Bounded)?;
                        Ok(CandidateOutcome::<(), ()>::Accepted(()))
                    })?;
                    if verdict == Verdict::Fails {
                        return Ok(ExtensionMatch::Unmatched);
                    }
                    self.check.register_relation(constraint)?;
                }
            }
        }

        Ok(ExtensionMatch::Matched(
            Box::new(substitution),
            implementation,
        ))
    }

    /// Bind extension parameters by matching the target against the receiver.
    fn bind_extension_target(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        target: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // a direct structural match binds most extension targets
        let mut scratch = substitution.clone();
        if self.extend_generic_substitution(
            origin,
            parameters,
            &mut scratch,
            &[(target, receiver)],
        )? {
            *substitution = scratch;

            return Ok(true);
        }

        // nominal receivers bind through their substituted heritage
        let receiver_value = self.strip_form(origin, receiver)?;
        let is_nominal = self.nominal_application_maybe(receiver_value)?.is_some();
        if !is_nominal {
            return Ok(false);
        }

        let closure = self.heritage_closure(origin, receiver_value)?;
        for ancestor in &closure.applications {
            let mut scratch = substitution.clone();
            if self.extend_generic_substitution(
                origin,
                parameters,
                &mut scratch,
                &[(target, ancestor.ty)],
            )? {
                *substitution = scratch;

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the program type witnessing that one other implementation overlaps this one.
    pub(in crate::sema) fn extension_implementations_overlap(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        target_type: dir::GlobalTypeId,
        interface_type: dir::GlobalTypeId,
        other: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the other declaration's target and implemented interfaces
        let Some(dir::Definition::Extension(extension)) = self.definition(other)? else {
            return Ok(None);
        };
        let other_target = extension.target.r#type();
        let other_interfaces = extension
            .implements
            .iter()
            .map(|conformance| conformance.interface)
            .collect::<SmallVec<[_; 2]>>();
        let other_template = self.symbol_template(other)?;
        let template = self.symbol_template(symbol)?;
        let parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };

        // enumerate the declared inhabitants of each interface bounded parameter
        let predicates = self.check.template_predicates(template);
        let mut inhabitants = Vec::with_capacity(parameters.len());
        for parameter in &parameters {
            let mut bounds = SmallVec::<[dir::GlobalTypeId; 2]>::new();
            bounds.extend(self.require_generic_parameter(*parameter)?.constraint);
            for predicate in &predicates {
                if predicate.relation == dir::WhereRelation::Satisfies
                    && matches!(self.ty(predicate.left)?, dir::Type::Parameter(bounded) if bounded == *parameter)
                {
                    bounds.push(predicate.right);
                }
            }
            let mut witnesses = None;
            for bound in bounds {
                witnesses = self.interface_inhabitants(module, bound)?;
                if witnesses.is_some() {
                    break;
                }
            }
            if witnesses.as_ref().is_some_and(Vec::is_empty) {
                return Ok(None);
            }
            inhabitants.push(witnesses);
        }

        // try every assignment of witnesses, leaving unbounded parameters open
        let mut assignment = vec![0usize; inhabitants.len()];
        loop {
            let witnesses = inhabitants
                .iter()
                .zip(&assignment)
                .map(|(witnesses, index)| witnesses.as_ref().map(|witnesses| witnesses[*index]))
                .collect::<SmallVec<[Option<dir::GlobalTypeId>; 4]>>();
            if let Some(witness) = self.overlap_under_witnesses(
                origin,
                &parameters,
                &witnesses,
                target_type,
                interface_type,
                other_template,
                other_target,
                &other_interfaces,
                template,
            )? {
                return Ok(Some(witness));
            }

            // advance the odometer over the witness lists
            let mut position = assignment.len();
            loop {
                if position == 0 {
                    return Ok(None);
                }
                position -= 1;
                let count = inhabitants[position].as_ref().map_or(1, Vec::len);
                assignment[position] += 1;
                if assignment[position] < count {
                    break;
                }
                assignment[position] = 0;
            }
        }
    }

    /// Prove one overlap under a witness assignment, returning the witnessed receiver.
    fn overlap_under_witnesses(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        witnesses: &[Option<dir::GlobalTypeId>],
        target_type: dir::GlobalTypeId,
        interface_type: dir::GlobalTypeId,
        other_template: Option<GenericTemplateId>,
        other_target: dir::GlobalTypeId,
        other_interfaces: &[dir::GlobalTypeId],
        template: Option<GenericTemplateId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // admit the other implementation once it matches the witnessed instantiation
        self.check.counters.extension_probes += 1;
        let mut receiver_witness = None;
        let verdict = self.probe_candidate(|state| {
            // open this declaration's parameters and pin the witnessed ones
            let Some(substitution) = state.instantiate_parameters(
                origin,
                parameters,
                &[],
                TypeSubstitution::default(),
                TypeArgumentInference::Exact,
            )?
            else {
                return Ok(CandidateOutcome::<(), ()>::Rejected(()));
            };
            for (parameter, witness) in parameters.iter().zip(witnesses) {
                let Some(witness) = witness else {
                    continue;
                };
                let Some(opened) = substitution.argument(*parameter) else {
                    continue;
                };
                let dir::Type::Variable(variable) = state.ty(opened)? else {
                    continue;
                };
                state.check.commit_solution(variable, *witness)?;
            }
            let receiver = state.substitute_type(target_type, &substitution)?;
            let interface = state.substitute_type(interface_type, &substitution)?;
            let (interface_module, interface) = state.nominal_application(interface)?;

            // bind the witnessed instantiation through the other implementation
            let matched = state.match_extension_implementation(
                origin,
                Relation::Assignable,
                interface_module,
                receiver,
                receiver,
                &interface,
                other_template,
                other_target,
                other_interfaces,
                OpenBounds::Keep,
            )?;
            if matches!(matched, ExtensionMatch::Unmatched) {
                return Ok(CandidateOutcome::<(), ()>::Rejected(()));
            }

            // hold this declaration's own bounds over the witnessed instantiation
            if let Some(template) = template {
                for constraint in
                    state.substitute_application_constraints(origin, template, &substitution)?
                {
                    let closed = state.check.type_variables(constraint.source)?.is_empty()
                        && state.check.type_variables(constraint.target)?.is_empty();
                    let id = state.check.register_relation(constraint)?;
                    if closed {
                        state.check.solve_relation(id, Settle::Complete)?;
                        let outcome = state.check.fulfill.checks.result(id)?;
                        if matches!(outcome, Some(CheckOutcome::Fails(_))) {
                            return Ok(CandidateOutcome::Rejected(()));
                        }
                    }
                }
            }
            receiver_witness = Some(state.check.deeply_resolve(origin, receiver)?);

            Ok(CandidateOutcome::Accepted(()))
        });

        Ok(match verdict? {
            Verdict::Fails => None,
            _ => receiver_witness,
        })
    }

    /// Return the declared types implementing one interface bound, or `None` for every other bound.
    fn interface_inhabitants(
        &mut self,
        module: ModuleId,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        let mut inhabitants = Vec::new();
        let mut visited = FxIndexSet::default();
        let is_interface =
            self.collect_interface_inhabitants(module, constraint, &mut visited, &mut inhabitants)?;

        Ok(is_interface.then_some(inhabitants))
    }

    /// Collect the rooted implementation targets of one interface, through blanket implementors.
    fn collect_interface_inhabitants(
        &mut self,
        module: ModuleId,
        constraint: dir::GlobalTypeId,
        visited: &mut FxIndexSet<dir::GlobalSymbolId>,
        inhabitants: &mut Vec<dir::GlobalTypeId>,
    ) -> CompilerResult<bool> {
        // read the bound's interface, other bounds leave the parameter open
        let Some((_, instance)) = self.nominal_application_maybe(constraint)? else {
            return Ok(false);
        };
        if !self.symbol_kind(instance.symbol)?.is_interface() {
            return Ok(false);
        }
        if !visited.insert(instance.symbol) {
            return Ok(true);
        }

        // rooted implementations witness themselves, blankets witness through their bound
        for implementor in self.visible_implementations(module, instance.symbol)? {
            let Some(dir::Definition::Extension(extension)) = self.definition(implementor)? else {
                continue;
            };
            match extension.target {
                dir::ExtensionTarget::Rooted { ty, .. } => {
                    if !inhabitants.contains(&ty) {
                        inhabitants.push(ty);
                    }
                }
                dir::ExtensionTarget::Blanket { ty, .. } => {
                    let dir::Type::Parameter(parameter) = self.ty(ty)? else {
                        continue;
                    };
                    let Some(bound) = self
                        .generic_parameter(parameter)
                        .and_then(|binding| binding.constraint)
                    else {
                        continue;
                    };
                    self.collect_interface_inhabitants(module, bound, visited, inhabitants)?;
                }
            }
        }

        Ok(true)
    }

    /// Collect the visible implementation declarations of one interface.
    pub(in crate::sema) fn visible_implementations(
        &mut self,
        module: ModuleId,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // gather the implementations the program declares, then the local extensions
        let mut candidates = self
            .module(module)
            .iter_definitions()
            .filter_map(|(symbol, definition)| {
                matches!(definition, dir::Definition::Extension(_)).then_some(symbol)
            })
            .collect::<SmallVec<[_; 8]>>();
        if let Some(environment) = &self.check.environment_declared
            && let Some(implementors) = environment.implementations_by_interface.get(&interface)
        {
            candidates.extend(implementors.iter().copied());
        }
        candidates.extend(
            self.check
                .program_implementations(interface)?
                .into_iter()
                .map(|(symbol, _)| symbol),
        );
        let imported = self
            .module(module)
            .resolved
            .imports
            .symbol_targets()
            .map(|(_, symbol)| symbol)
            .collect::<SmallVec<[_; 8]>>();
        candidates.extend(imported);

        // keep the declarations whose conformances reach the interface
        let mut symbols = SmallVec::new();
        for symbol in candidates {
            if symbols.contains(&symbol) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(symbol)? else {
                continue;
            };
            let interfaces = extension
                .implements
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            if self.implements_requested_interface(&interfaces, interface)? {
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Collect visible extensions that may implement an interface for one receiver.
    pub(in crate::sema) fn visible_implementation_extensions(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // collect the extensions in scope for the receiver's own head
        let mut symbols = self.collect_implementation_extensions(origin, module, receiver)?;

        // admit the implicit implementors of the interface
        if let Some(environment) = &self.check.environment_declared
            && let Some(implementors) = environment.implementations_by_interface.get(&interface)
        {
            for symbol in implementors.clone() {
                if !symbols.contains(&symbol) {
                    symbols.push(symbol);
                }
            }
        }

        // admit the program's implementors rooted like the receiver, and every blanket
        let receiver_root = self.receiver_root_symbol(origin, receiver)?;
        for (symbol, root) in self.check.program_implementations(interface)? {
            let is_rooted_alike = match (root, receiver_root) {
                (Some(root), Some(receiver_root)) => root == receiver_root,
                (None, _) => true,
                (Some(_), None) => true,
            };
            if is_rooted_alike && !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Return the declaration symbol one receiver roots at, when it names one.
    fn receiver_root_symbol(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let receiver = self.strip_form(origin, receiver)?;
        if let Some((_, instance)) = self.nominal_application_maybe(receiver)? {
            return Ok(Some(instance.symbol));
        }

        self.ty(receiver)?
            .representation_item()
            .map(|item| self.language_symbol(item))
            .transpose()
    }

    /// Collect the implementation extension candidates for one receiver head.
    fn collect_implementation_extensions(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        let mut parameters = SmallVec::new();
        let mut symbols = SmallVec::new();
        self.collect_receiver_extensions(origin, module, receiver, &mut parameters, &mut symbols)?;

        Ok(symbols)
    }

    /// Collect extension candidates through one receiver, guarding parameter cycles.
    fn collect_receiver_extensions(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
        symbols: &mut SmallVec<[dir::GlobalSymbolId; 4]>,
    ) -> CompilerResult<()> {
        let receiver = self.strip_form(origin, receiver)?;
        match self.ty(receiver)? {
            // rigid parameters implement through blankets and their bound scopes
            dir::Type::Parameter(parameter) => {
                if parameters.contains(&parameter) {
                    return Ok(());
                }

                for symbol in self.visible_blanket_extensions(module)? {
                    if !symbols.contains(&symbol) {
                        symbols.push(symbol);
                    }
                }

                // descend into each declared bound under the parameter
                let bounds = self.parameter_bounds(origin, parameter)?;
                parameters.push(parameter);
                for bound in bounds {
                    let collected = self
                        .collect_receiver_extensions(origin, module, bound, parameters, symbols);
                    match collected {
                        Ok(()) => {}
                        other => {
                            parameters.pop();

                            return other;
                        }
                    }
                }

                parameters.pop();

                Ok(())
            }

            // composite receivers implement through their element scopes
            dir::Type::Union(dir::UnionType { elements, .. })
            | dir::Type::Intersection(dir::IntersectionType { elements, .. }) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(receiver.module_id, elements)?.into();
                for element in elements {
                    self.collect_receiver_extensions(origin, module, element, parameters, symbols)?;
                }

                Ok(())
            }

            // every other receiver implements through its declaration scope
            _ => {
                let scope = match self.nominal_application_maybe(receiver)? {
                    Some((_, instance)) => Some(instance.symbol),
                    None => self
                        .ty(receiver)?
                        .representation_item()
                        .map(|item| self.language_symbol(item))
                        .transpose()?,
                };

                let Some(scope) = scope else {
                    for symbol in self.visible_blanket_extensions(module)? {
                        if !symbols.contains(&symbol) {
                            symbols.push(symbol);
                        }
                    }

                    return Ok(());
                };

                if !self.is_own_module(scope.module_id) {
                    self.import_external_module(scope.module_id)?;
                }

                // visible extensions already include the prelude's symbols
                for symbol in self.visible_extensions(module, dir::TypeRoot::Declaration(scope))? {
                    if !symbols.contains(&symbol) {
                        symbols.push(symbol);
                    }
                }

                Ok(())
            }
        }
    }

    /// Collect blanket extension symbols visible from one module.
    pub(in crate::sema) fn visible_blanket_extensions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        let mut symbols = SmallVec::new();

        // collect local blanket extensions
        if let Some(state) = self.module_maybe(module) {
            symbols.extend(state.blanket_extensions());
        }

        // collect the implicit open blanket extension symbols
        if let Some(environment) = &self.check.environment_declared {
            for symbol in &environment.blanket_extensions {
                if !symbols.contains(symbol) {
                    symbols.push(*symbol);
                }
            }
        }

        // collect imported blanket extension symbols
        let imported = self
            .module(module)
            .resolved
            .imports
            .symbol_targets()
            .map(|(_, symbol)| symbol)
            .collect::<SmallVec<[_; 8]>>();
        for symbol in imported {
            let is_blanket = matches!(
                self.definition(symbol)?,
                Some(dir::Definition::Extension(extension)) if extension.target.is_blanket()
            );

            if is_blanket && !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Collect one extension's member candidates for a subject, paired with their keys.
    ///
    /// The extension target matches once for every member it declares.
    pub(in crate::sema) fn extension_subject_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        root: dir::TypeRoot,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        key: Option<dir::StaticKey>,
    ) -> CompilerResult<Option<Vec<(dir::StaticKey, MemberCandidate)>>> {
        // skip extensions removed by statically false gates
        if self.is_absent_symbol(extension_symbol) {
            return Ok(Some(Vec::new()));
        }

        // require a declaration this module can see
        let extension = match self.definition(extension_symbol)? {
            Some(dir::Definition::Extension(extension)) => extension,
            Some(_) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "indexed extension symbol {extension_symbol:?} has no extension definition"
                    ),
                });
            }
            None => return Ok(Some(Vec::new())),
        };

        if !extension.is_visible_from(module) {
            return Ok(Some(Vec::new()));
        }

        // close recursive lookups of this extension coinductively, leaving them undecided
        if !self.check.extending.insert((extension_symbol, subject)) {
            return Ok(None);
        }

        // collect the candidates, then release the re-entry mark
        let result = self.collect_extension_candidates(
            origin,
            module,
            root,
            receiver,
            subject,
            extension_symbol,
            space,
            key,
        );
        self.check
            .extending
            .swap_remove(&(extension_symbol, subject));

        result.map(Some)
    }

    /// Collect one extension's declared and conformance member candidates.
    fn collect_extension_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        root: dir::TypeRoot,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        key: Option<dir::StaticKey>,
    ) -> CompilerResult<Vec<(dir::StaticKey, MemberCandidate)>> {
        // read the extension's target and the interfaces it implements
        let Some(dir::Definition::Extension(extension)) = self.definition_maybe(extension_symbol)
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "loaded extension symbol {extension_symbol:?} has no extension definition"
                ),
            });
        };
        let target_type = extension.target.r#type();
        let implements = extension
            .implements
            .iter()
            .map(|conformance| conformance.interface)
            .collect::<SmallVec<[_; 2]>>();

        // keep the members this extension declares in the requested space
        let mut members = Vec::new();
        for member in &extension.members {
            if member.space() != space {
                continue;
            }

            let Some(declared) = DeclaredMember::from_definition(member)? else {
                continue;
            };
            if let Some(key) = key
                && declared.key != key
            {
                continue;
            }

            members.push(declared);
        }

        // fill the keys the extension left open with conformance defaults
        let mut conformances = SmallVec::<[(ModuleId, dir::GenericApplication); 2]>::new();
        for implemented in implements {
            let Some((interface_module, interface)) =
                self.nominal_application_maybe(implemented)?
            else {
                continue;
            };

            // load the interface declaration
            if !self.check.is_loaded_module(interface.symbol.module_id) {
                self.check
                    .import_external_module(interface.symbol.module_id)?;
            }
            let Some(dir::Definition::Interface(definition)) =
                self.check.definition_maybe(interface.symbol)
            else {
                continue;
            };

            // take each default member whose key the extension left open
            let first_default = members.len();
            for member in &definition.members {
                if member.space() != space || !self.check.definition_member_has_default(member) {
                    continue;
                }
                if let Some(key) = key
                    && member.key() != Some(key)
                {
                    continue;
                }

                let Some(declared) = DeclaredMember::from_definition(member)? else {
                    continue;
                };
                if members.iter().any(|kept| kept.key == declared.key) {
                    continue;
                }

                members.push(declared);
            }

            // keep the interface whose defaults the extension took
            if members.len() > first_default {
                conformances.push((interface_module, interface));
            }
        }

        if members.is_empty() {
            return Ok(Vec::new());
        }

        // read the decided arguments for this extension
        let sources = self.decided_extension_sources(origin, module, receiver, subject, root)?;
        let Some(source) = sources
            .into_iter()
            .find(|source| source.extension == extension_symbol)
        else {
            return Ok(Vec::new());
        };
        let arguments = source.arguments;
        let template = self.symbol_template(extension_symbol)?;
        let own_module = self.check.module_id;
        let mut substitution = TypeSubstitution::default();
        let mut reopened_parameters: FxIndexMap<GenericParameterId, dir::GlobalTypeId> =
            FxIndexMap::default();
        if let Some(template) = template {
            let parameters = self.generic_template_parameters(template)?;
            for (parameter, argument) in parameters.iter().zip(arguments.iter()) {
                // stand fresh site variables in for undetermined parameters
                let mut argument = *argument;
                for erased in self.erased_parameters(argument)? {
                    let fresh = match reopened_parameters.get(&erased) {
                        Some(fresh) => *fresh,
                        None => {
                            let _binding = *self.require_generic_parameter(erased)?;
                            let variable = self.allocate_variable(
                                origin,
                                VariableRole::Instantiation { parameter: erased },
                            );
                            let fresh = self.variable_type(variable)?;
                            reopened_parameters.insert(erased, fresh);

                            fresh
                        }
                    };
                    let from = self.intern_type(dir::Type::Erased(erased))?;
                    argument = self.check.replace_type(own_module, argument, from, fresh)?;
                }
                substitution.bind(*parameter, argument)?;
            }
        }

        // bind this to the matched target, the value beneath the receiver's forms
        let this = self.substitute_type(target_type, &substitution)?;
        let mut substitution = substitution.with_receiver(this);

        // bind reopened parameters through the declared bounds and the decided target match
        let is_open = !reopened_parameters.is_empty()
            || (self.check.type_flags(receiver)? | self.check.type_flags(subject)?).has_variable();
        let mut bounds = Vec::new();
        let mut target = None;
        if is_open {
            if let Some(template) = template {
                bounds.extend(self.substitute_application_constraints(
                    origin,
                    template,
                    &substitution,
                )?);
            }
            let matched = self.substitute_type(target_type, &substitution)?;
            let cause = self
                .check
                .intern_cause(Cause::root(origin, CauseKind::Expression));
            target = Some(RelationCheck {
                origin,
                relation: Relation::Assignable,
                source: receiver,
                target: matched,
                cause,
                application: None,
            });
        }

        // read the checked type of each member the matched extension exposes
        self.resolve_declared_types(&mut members)?;

        // rewrite conformance interface parameters into subject terms
        for (interface_module, interface) in &conformances {
            let interface_substitution =
                self.instance_substitution(*interface_module, interface)?;
            for binding in interface_substitution.bindings {
                let argument = self.substitute_type(binding.argument, &substitution)?;
                substitution.bind(binding.parameter, argument)?;
            }
        }

        // substitute the decided arguments into every declared member
        let mut candidates =
            self.extension_member_candidates(origin, extension_symbol, &substitution, &members)?;
        for (_, candidate) in &mut candidates {
            candidate.bounds = bounds.clone();
            candidate.target = target;
        }

        Ok(candidates)
    }

    /// Look up matching static members from one extension declaration.
    fn lookup_one_static_extension(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        extension_symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // skip extensions removed by statically false gates
        if self.is_absent_symbol(extension_symbol) {
            return Ok(MemberLookup::Missing);
        }

        // read matching static members from extensions of this declaration
        let extension = match self.definition(extension_symbol)? {
            Some(dir::Definition::Extension(extension)) => extension,
            Some(_) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "indexed extension symbol {extension_symbol:?} has no extension definition"
                    ),
                });
            }
            None => return Ok(MemberLookup::Missing),
        };

        if !extension.is_visible_from(module) {
            return Ok(MemberLookup::Missing);
        }

        if extension.target.root() != Some(dir::TypeRoot::Declaration(symbol)) {
            return Ok(MemberLookup::Missing);
        }

        // clone the members matching the requested key, releasing the definition borrow
        let target_type = extension.target.r#type();
        let keyed: SmallVec<[dir::DefinitionMember; 2]> = extension
            .members
            .iter()
            .filter(|member| {
                member.space() == dir::MemberSpace::Static && member.key() == Some(key)
            })
            .cloned()
            .collect();
        let members = self.matching_extension_members(&keyed, dir::MemberSpace::Static, key)?;
        if members.is_empty() {
            return Ok(MemberLookup::Missing);
        }

        // bind extension parameters through the written receiver arguments
        let substitution = match arguments.is_empty() {
            // a bare declaration name reads the statics of its default form
            true => {
                let chain = self.form_chain(origin, target_type)?;
                if chain.ownership_form().is_some() {
                    return Ok(MemberLookup::Missing);
                }

                TypeSubstitution::default()
            }
            // a written application binds the extension through its target
            false => {
                let arguments = self.intern_type_ids(arguments)?;
                let instance = dir::GenericApplication { symbol, arguments };
                let applied = self.intern_type(dir::Type::Application(instance))?;
                let template = self.symbol_template(extension_symbol)?;

                // match the written application against the extension target
                let matched = self.confirm_candidate(|state| {
                    let matched = state.match_extension_subject(
                        origin,
                        applied,
                        applied,
                        template,
                        target_type,
                        UnboundParameters::Open,
                    )?;
                    Ok(match matched {
                        Some(substitution) => CandidateOutcome::Accepted(substitution),
                        None => CandidateOutcome::Rejected(()),
                    })
                })?;
                let Some(substitution) = matched else {
                    return Ok(MemberLookup::Missing);
                };

                substitution
            }
        };

        // expose the matching members under that substitution
        let candidates =
            self.extension_member_candidates(origin, extension_symbol, &substitution, &members)?;
        let candidates = candidates
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect();

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Return the substituted member candidates one matched extension exposes per key.
    pub(in crate::sema) fn extension_member_candidates(
        &mut self,
        origin: Origin,
        extension_symbol: dir::GlobalSymbolId,
        substitution: &TypeSubstitution,
        members: &[DeclaredMember],
    ) -> CompilerResult<Vec<(dir::StaticKey, MemberCandidate)>> {
        // blanket declarations sit farther than rooted extensions
        let member_origin = match self.definition(extension_symbol)? {
            Some(dir::Definition::Extension(extension)) if extension.target.is_blanket() => {
                dir::MemberOrigin::BlanketExtension
            }
            _ => dir::MemberOrigin::RootedExtension,
        };

        // substitute extension parameters in each matching member
        let mut candidates = Vec::new();
        for member in members {
            let Some(ty) = member.ty else {
                continue;
            };

            let ty = self.substitute_type(ty, substitution)?;
            let callable = member.callable_type(ty);
            let access_type = member.access_type(self, ty)?;
            let access_type = self.projected_member_type(
                origin,
                substitution.receiver,
                member.role,
                access_type,
            )?;

            // substitute static projections through the same extension instance
            let written = match self.static_value(member.symbol) {
                Some(written) => Some(self.substitute_type(written, substitution)?),
                None => None,
            };

            // carry the solved extension arguments onto the candidate
            let generic_arguments = self.settled_argument_bindings(&substitution.bindings)?;
            let requirement = self.requirement_interface(extension_symbol, member.key)?;
            let candidate = MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                origin: member_origin,
                requirement,
                space: member.space,
                role: member.role,
                kind: member.kind,
                is_writable: member.is_writable,
                access_type,
                callable,
                is_optional: member.is_optional,
                generic_arguments,
                value: None,
                value_type: written,
                receiver: LookupReceiver::Direct(ReceiverSteps::new()),
                bounds: Vec::new(),
                target: None,
            };

            candidates.push((member.key, candidate));
        }

        Ok(candidates)
    }

    /// Collect the applied interfaces extensions conform one owner to under one associated key.
    pub(in crate::sema) fn conformed_interfaces(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let module = self.module_id;
        let symbols = self.collect_implementation_extensions(origin, module, owner)?;

        let mut interfaces = SmallVec::new();
        for extension_symbol in symbols {
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)?
            else {
                continue;
            };
            if !extension.is_visible_from(module) {
                continue;
            }
            let conformances = extension
                .implements
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            let target_type = extension.target.r#type();

            // retain conformances whose interface declares the projected member
            let mut declaring = SmallVec::<[dir::GlobalTypeId; 2]>::new();
            for conformance in conformances {
                if self.declares_associated_type(conformance, key)? {
                    declaring.push(conformance);
                }
            }
            if declaring.is_empty() {
                continue;
            }

            // a written projection qualifies through target-closed rows only
            let template = self.symbol_template(extension_symbol)?;
            let matched = self.match_extension_subject(
                origin,
                owner,
                owner,
                template,
                target_type,
                UnboundParameters::Reject,
            )?;
            let Some(substitution) = matched else {
                continue;
            };

            for interface in declaring {
                let applied = self.substitute_type(interface, &substitution)?;
                if !interfaces.contains(&applied) {
                    interfaces.push(applied);
                }
            }
        }

        Ok(interfaces)
    }

    /// Match one extension target against a receiver and its widened lookup subject.
    pub(in crate::sema) fn match_extension(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        // match the exact receiver first, then its widened lookup subject
        let substitution = self.match_extension_subject(
            origin,
            receiver,
            receiver,
            template,
            target_type,
            UnboundParameters::Open,
        )?;
        if substitution.is_none() && subject != receiver {
            return self.match_extension_subject(
                origin,
                receiver,
                subject,
                template,
                target_type,
                UnboundParameters::Open,
            );
        }

        Ok(substitution)
    }

    /// Match one lookup subject against an extension target and its constraints.
    pub(in crate::sema) fn match_extension_subject(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
        unbound: UnboundParameters,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        // written class names match as their canonical declaration instance
        let subject = match self.ty(subject)? {
            dir::Type::Reference(reference) => {
                let instance = self.declaration_instance(reference.symbol)?;

                self.intern_type(dir::Type::Application(instance))?
            }
            _ => subject,
        };

        // require the subject head to reach the declared target head
        if !self.root_matches_target(subject, target_type)? {
            return Ok(None);
        }

        // matching binds the extension parameters through the target header
        let mut substitution = TypeSubstitution::default();
        if let Some(template) = template {
            let parameters = self.generic_template_parameters(template)?;
            let subject_pair = [(target_type, subject)];
            if !self.extend_generic_substitution(
                origin,
                &parameters,
                &mut substitution,
                &subject_pair,
            )? {
                return Ok(None);
            }

            match unbound {
                // open the parameters the target leaves unbound
                UnboundParameters::Open => {
                    let opened = self.instantiate_parameters(
                        origin,
                        &parameters,
                        &[],
                        substitution,
                        TypeArgumentInference::Exact,
                    )?;
                    let Some(opened) = opened else {
                        return Ok(None);
                    };
                    substitution = opened;
                }
                // require the target header to bind every parameter
                UnboundParameters::Reject => {
                    let is_unbound = parameters
                        .iter()
                        .any(|parameter| substitution.argument(*parameter).is_none());
                    if is_unbound {
                        return Ok(None);
                    }
                }
            }
        }

        let substitution = substitution.with_receiver(receiver);

        // require the subject to satisfy the bound target
        let target = self.substitute_type(target_type, &substitution)?;
        if self.evaluate_relation(origin, Relation::Assignable, subject, target)? == Verdict::Fails
        {
            return Ok(None);
        }

        // filter on the declared constraints without committing their obligations
        if let Some(template) = template
            && !self.substitution_constraints_may_hold(origin, template, &substitution)?
        {
            return Ok(None);
        }

        Ok(Some(substitution))
    }
}
