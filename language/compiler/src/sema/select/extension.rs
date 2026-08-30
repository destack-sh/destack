use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    ActiveGoal, Answer, CandidateOutcome, Cause, CauseKind, CheckOutcome, CheckState,
    DeclaredCandidate, DeclaredMember, ExtensionSource, GenericParameterId, GenericTemplateId,
    Goal, Implementation, MemberCandidate, MemberLookup, Origin, Relation, RelationCheck, Settle,
    TypeArgumentInference, TypeSubstitution, Value, Verdict,
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
    /// Decide the bound against the receiver's known bounds, rejecting a candidate that fails it.
    Decide,
    /// Keep the bound open, so a closed bound alone rejects the candidate.
    Keep,
}

/// Whether one extension exposes a looked-up member key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyExposure {
    /// The extension enumerates its keys and declares this one.
    Present,
    /// The extension enumerates its keys and declares no such key.
    Absent,
    /// The extension exposes keys beyond the ones this decision enumerates.
    Ambiguous,
}

impl KeyExposure {
    /// Classify one enumerated key set's answer.
    fn decided(is_present: bool) -> Self {
        match is_present {
            true => Self::Present,
            false => Self::Absent,
        }
    }
}

/// The outcome of matching one extension implementation.
#[derive(Debug, Clone)]
pub(in crate::sema) enum ExtensionMatch {
    /// The extension applies with this substitution and matched interface.
    Matched(Box<TypeSubstitution>, dir::GlobalTypeId),
    /// Every instantiation of the extension fails to apply.
    Unmatched,
    /// The match stays undecided while a bound or operand is open.
    Unproven,
}

/// The target head one extension query selects by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum ExtensionHead {
    /// Extensions over one declared root, blankets included.
    Root(dir::TypeRoot),
    /// Blanket extensions alone.
    Blanket,
    /// Every extension in scope.
    Any,
}

impl ExtensionHead {
    /// Return whether an extension over one target root answers this head.
    fn admits(self, root: Option<dir::TypeRoot>) -> bool {
        match (self, root) {
            (Self::Any, _) | (_, None) => true,
            (Self::Root(head), Some(root)) => head == root,
            (Self::Blanket, Some(_)) => false,
        }
    }
}

impl CheckState<'_> {
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
        // collect the extensions this subject pair sees
        let extensions = self.subject_extensions(origin, module, receiver, subject, root)?;

        // decide each extension once per subject identity, first declaration per symbol wins
        let mut candidates = Vec::new();
        let mut seen = FxIndexSet::default();
        for extension in extensions {
            // skip a blanket whose enumerated keys exclude the looked-up key
            if self.decide_extension_key(extension, key)? == KeyExposure::Absent {
                continue;
            }

            // require a match against the subject pair
            let Some(source) =
                self.decide_extension_source(origin, module, receiver, subject, extension)?
            else {
                continue;
            };
            let matched =
                self.extension_candidates(origin, receiver, subject, &source, space, Some(key))?;

            // keep the first candidate each symbol exposes under the key
            for (candidate_key, candidate) in matched.iter() {
                if *candidate_key != key {
                    continue;
                }
                if !seen.insert(candidate.symbol()) {
                    continue;
                }

                candidates.push(candidate.clone());
            }
        }

        Ok(candidates)
    }

    /// Match one extension against a subject pair, deducing its template arguments.
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

        // match the receiver once, erasing the instantiations this probe left undeduced
        let template = self.symbol_template(extension_symbol)?;
        let own_module = self.module_id;
        let baseline = self.infer.variable_count();
        let is_settled = !(self.type_flags(receiver)? | self.type_flags(subject)?).has_variable();
        let deduced = self.decide_deduction(|state| {
            let Some(substitution) =
                state.match_extension(origin, receiver, subject, template, target_type)?
            else {
                return Ok(None);
            };

            // solve the declared bounds of resolved pairs, binding their induced parameters
            if is_settled && let Some(template) = template {
                let constraints =
                    state.substitute_application_constraints(origin, template, &substitution)?;
                for constraint in constraints {
                    let id = state.queue_relation(constraint)?;
                    state.solve_relation(id, Settle::Possible)?;
                }
            }

            // read each parameter's deduced argument, erasing the ones left open
            let mut arguments = SmallVec::<[dir::GlobalTypeId; 4]>::new();
            if let Some(template) = template {
                for parameter in state.generic_template_parameters(template)? {
                    let argument = match substitution.argument(parameter) {
                        Some(argument) => {
                            let mut argument = state.shallow_resolve(argument)?;
                            for variable in state.type_variables(argument)? {
                                if (variable.0 as usize) < baseline {
                                    continue;
                                }
                                let Some(parameter) = state.infer.variable(variable)?.parameter
                                else {
                                    continue;
                                };
                                let from = state.intern_type(dir::Type::Variable(variable))?;
                                let to = state.intern_type(dir::Type::Erased(parameter))?;
                                argument = state.replace_type(own_module, argument, from, to)?;
                            }

                            argument
                        }
                        None => state.intern_type(dir::Type::Erased(parameter))?,
                    };
                    arguments.push(argument);
                }
            }

            Ok(Some(arguments))
        })?;

        Ok(deduced)
    }

    /// Collect the erased parameters one type graph mentions.
    fn erased_parameters(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalGenericParameterId; 2]>> {
        // stop where the graph mentions no parameter
        let mut erased = SmallVec::new();
        if !self.type_flags(id)?.has_parameter() {
            return Ok(erased);
        }

        // walk the graph, collecting each erased parameter once
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
            self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?;
        }

        Ok(erased)
    }

    /// Return whether one subject's nominal head matches a concrete target head.
    fn is_matching_root(
        &mut self,
        subject: dir::GlobalTypeId,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // interface and alias targets match every head
        let Some(target_symbol) = self.nominal_head(target_type)? else {
            return Ok(true);
        };
        if matches!(
            self.definition(target_symbol)?,
            None | Some(dir::Definition::Interface(_) | dir::Definition::TypeAlias(_))
        ) {
            return Ok(true);
        }

        // concrete targets match their own head and declared heritage
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
        let ty = self.shallow_resolve(ty)?;

        Ok(match self.ty(ty)? {
            dir::Type::Application(instance) => Some(instance.symbol),
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
        if let Some(requirements) = self.requirement_interfaces.get(&extension) {
            return Ok(requirements.get(&key).copied());
        }

        // map each implemented interface's member keys to that interface
        let mut requirements = FxIndexMap::default();
        if matches!(
            self.definition(extension)?,
            Some(dir::Definition::Extension(_))
        ) {
            let interfaces = self.declared_interfaces(extension)?;
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

        // read the requested key's interface
        let requirement = requirements.get(&key).copied();

        // memoize complete definitions only: declaration-time members are still arriving
        if !self.is_declaring() {
            self.requirement_interfaces.insert(extension, requirements);
        }

        Ok(requirement)
    }

    /// Decide whether one extension exposes one member key, memoizing the keys it enumerates.
    ///
    /// A rooted extension's own target root already scopes it, while a blanket exposes its
    /// declared keys and its interfaces' member keys.
    /// A member whose key stays unknown leaves the whole set unenumerable.
    fn decide_extension_key(
        &mut self,
        extension: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<KeyExposure> {
        // serve the memo
        if let Some(keys) = self.blanket_keys.get(&extension) {
            return Ok(KeyExposure::decided(keys.contains(&key)));
        }

        // read the blanket's declared keys with its implemented interfaces
        let mut interfaces = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        let mut keys = FxIndexSet::default();
        match self.definition(extension)? {
            Some(dir::Definition::Extension(definition)) if definition.target.is_blanket() => {
                for member in &definition.members {
                    let Some(declared) = member.key() else {
                        return Ok(KeyExposure::Ambiguous);
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
            _ => return Ok(KeyExposure::Ambiguous),
        }

        // add every member key the implemented interfaces declare
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
                    return Ok(KeyExposure::Ambiguous);
                };
                keys.insert(key);
            }
        }

        // decide the looked-up key against the enumerated set
        let exposes = keys.contains(&key);

        // memoize complete definitions only: declaration-time members are still arriving
        if !self.is_declaring() {
            self.blanket_keys.insert(extension, keys);
        }

        Ok(KeyExposure::decided(exposes))
    }

    /// Return whether one extension declaration is visible from one module.
    fn is_extension_visible(
        &mut self,
        extension: dir::GlobalSymbolId,
        module: ModuleId,
    ) -> CompilerResult<bool> {
        let Some(dir::Definition::Extension(definition)) = self.definition(extension)? else {
            return Ok(false);
        };

        Ok(definition.is_visible_from(module))
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
        let head = ExtensionHead::Root(dir::TypeRoot::Declaration(symbol));
        let extensions = self.extensions_over(module, &[head])?;
        let mut candidates = Vec::new();
        let mut seen = FxIndexSet::default();

        // visit extension declarations in resolution order
        for extension_symbol in extensions {
            // skip a blanket whose enumerated keys exclude the looked-up key
            if self.decide_extension_key(extension_symbol, key)? == KeyExposure::Absent {
                continue;
            }

            let found = self.lookup_one_static_extension(
                origin,
                module,
                symbol,
                arguments,
                extension_symbol,
                key,
            )?;
            for candidate in found {
                if !seen.insert(candidate.symbol()) {
                    continue;
                }

                candidates.push(candidate);
            }
        }

        Ok(candidates)
    }

    /// Collect the extension declarations one module sees over the target heads, each once.
    pub(in crate::sema) fn extensions_over(
        &mut self,
        module: ModuleId,
        heads: &[ExtensionHead],
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // collect each extension once over every head
        let mut symbols = SmallVec::new();
        for head in heads {
            for symbol in self.extensions_over_head(module, *head)? {
                if !symbols.contains(&symbol) {
                    symbols.push(symbol);
                }
            }
        }

        Ok(symbols)
    }

    /// Collect the extension declarations one module sees over one target head, memoized.
    fn extensions_over_head(
        &mut self,
        module: ModuleId,
        head: ExtensionHead,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // serve the memo
        if let Some(symbols) = self.visible_extensions.get(&(module, head)) {
            return Ok(symbols.clone());
        }

        // collect the extensions declared beside the looking module
        let mut symbols = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        if let Some(state) = self.module_maybe(module) {
            let declared = match head {
                ExtensionHead::Root(root) => state.root_extensions(root),
                ExtensionHead::Blanket => Vec::new(),
                ExtensionHead::Any => state
                    .iter_definitions()
                    .filter_map(|(symbol, definition)| {
                        matches!(definition, dir::Definition::Extension(_)).then_some(symbol)
                    })
                    .collect(),
            };
            symbols.extend(declared);
            symbols.extend(state.blanket_extensions());
        }

        // collect the inherent extensions declared beside a foreign head declaration
        if let ExtensionHead::Root(root @ dir::TypeRoot::Declaration(target)) = head
            && target.module_id != module
        {
            if !self.is_loaded_module(target.module_id) {
                self.import_external_module(target.module_id)?;
            }
            if let Some(state) = self.module_maybe(target.module_id) {
                symbols.extend(state.root_extensions(root));
                symbols.extend(state.blanket_extensions());
            }
            if let Some(external) = self.external_modules.get(&target.module_id) {
                symbols.extend(external.definitions.root_extensions(root));
                symbols.extend(external.definitions.blanket_extensions());
            }
        }

        // collect the implicit environment extensions over the head
        if let Some(environment) = &self.environment_declared {
            let implicit = match head {
                ExtensionHead::Root(root) => environment
                    .extensions_by_root
                    .get(&root)
                    .map(|symbols| symbols.as_slice())
                    .unwrap_or_default()
                    .to_vec(),
                ExtensionHead::Blanket => Vec::new(),
                ExtensionHead::Any => environment
                    .extensions_by_root
                    .values()
                    .flatten()
                    .copied()
                    .collect(),
            };
            symbols.extend(implicit);
            symbols.extend(environment.blanket_extensions.iter().copied());
        }

        // collect the imported extensions over the head
        let imported = self
            .module(module)
            .resolved
            .imports
            .symbol_targets()
            .map(|(_, symbol)| symbol)
            .collect::<SmallVec<[_; 8]>>();
        for symbol in imported {
            if let Some(dir::Definition::Extension(extension)) = self.definition(symbol)?
                && head.admits(extension.target.root())
            {
                symbols.push(symbol);
            }
        }

        // keep each symbol once, memoizing after the declare pass
        let mut seen = FxIndexSet::default();
        symbols.retain(|symbol| seen.insert(*symbol));
        if !self.is_declaring() {
            self.visible_extensions
                .insert((module, head), symbols.clone());
        }

        Ok(symbols)
    }

    /// Collect the extension declarations one subject pair sees through its heads.
    pub(in crate::sema) fn subject_extensions(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        root: dir::TypeRoot,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // head at the subject's declared root
        let mut heads = SmallVec::<[ExtensionHead; 4]>::new();
        heads.push(ExtensionHead::Root(root));

        // structural subjects also head at their constructor
        for ty in [receiver, subject] {
            let value = self.strip_form(origin, ty)?;
            if let Some(structural_root) = self.structural_root(value)? {
                heads.push(ExtensionHead::Root(structural_root));
            }
        }

        // formed subjects also head at each form constructor
        for ty in [receiver, subject] {
            let mut payload = self.shallow_resolve(ty)?;
            while let dir::Type::Form(form) = self.ty(payload)? {
                let constructor = self.language_symbol(form.form.language_item())?;
                heads.push(ExtensionHead::Root(dir::TypeRoot::Declaration(constructor)));
                payload = self.shallow_resolve(form.value)?;
            }
        }

        self.extensions_over(module, &heads)
    }

    /// Collect the implementations of one interface a module sees for one receiver.
    pub(in crate::sema) fn implementations_over(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: Option<dir::GlobalTypeId>,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[(dir::GlobalSymbolId, Option<dir::TypeRoot>); 4]>> {
        // collect the extensions over the receiver's heads
        let heads = match receiver {
            Some(receiver) => self.receiver_heads(origin, receiver)?,
            None => SmallVec::from_slice(&[ExtensionHead::Any]),
        };
        let mut symbols = self.extensions_over(module, &heads)?;

        // admit the program's implementations headed alike, and every blanket
        for (symbol, root) in self.program_implementations(interface)? {
            let root = root.map(dir::TypeRoot::Declaration);
            let is_headed_alike = heads.iter().any(|head| head.admits(root));
            if is_headed_alike && !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
        }

        // keep the declarations whose conformances reach the interface, with their roots
        let mut implementations = SmallVec::new();
        for symbol in symbols {
            let Some(dir::Definition::Extension(extension)) = self.definition(symbol)? else {
                continue;
            };
            let root = extension.target.root();
            let interfaces = extension
                .implements
                .iter()
                .map(|conformance| conformance.interface)
                .collect::<SmallVec<[_; 2]>>();
            if self.has_requested_interface(&interfaces, interface)? {
                implementations.push((symbol, root));
            }
        }

        Ok(implementations)
    }

    /// Collect the heads one receiver implements through.
    fn receiver_heads(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[ExtensionHead; 4]>> {
        // collect the heads under a parameter cycle guard
        let mut heads = SmallVec::new();
        let mut parameters = SmallVec::new();
        self.collect_receiver_heads(origin, receiver, &mut parameters, &mut heads)?;

        Ok(heads)
    }

    /// Collect the heads one receiver implements through, guarding parameter cycles.
    fn collect_receiver_heads(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
        heads: &mut SmallVec<[ExtensionHead; 4]>,
    ) -> CompilerResult<()> {
        // head the receiver by the value beneath its forms
        let receiver = self.strip_form(origin, receiver)?;
        match self.ty(receiver)? {
            // parameters implement through blankets and their bound heads
            dir::Type::Parameter(parameter) => {
                if parameters.contains(&parameter) {
                    return Ok(());
                }
                heads.push(ExtensionHead::Blanket);

                // descend into each declared bound under the parameter
                let bounds = self.parameter_bounds(origin, parameter)?;
                parameters.push(parameter);
                for bound in bounds {
                    self.collect_receiver_heads(origin, bound, parameters, heads)?;
                }
                parameters.pop();

                Ok(())
            }

            // composite receivers implement through their element heads
            dir::Type::Union(dir::UnionType { elements, .. })
            | dir::Type::Intersection(dir::IntersectionType { elements, .. }) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(receiver.module_id, elements)?.into();
                for element in elements {
                    self.collect_receiver_heads(origin, element, parameters, heads)?;
                }

                Ok(())
            }

            // every other receiver implements through its declaration head
            _ => {
                let declaration = match self.nominal_application_maybe(receiver)? {
                    Some((_, instance)) => Some(instance.symbol),
                    None => self
                        .ty(receiver)?
                        .representation_item()
                        .map(|item| self.language_symbol(item))
                        .transpose()?,
                };
                let head = match declaration {
                    Some(declaration) => {
                        ExtensionHead::Root(dir::TypeRoot::Declaration(declaration))
                    }
                    None => ExtensionHead::Blanket,
                };
                heads.push(head);

                Ok(())
            }
        }
    }

    /// Decide whether one extension matches a subject pair, with its deduced arguments.
    ///
    /// A re-entered pair misses. Equal asks remember the verdict.
    pub(in crate::sema) fn decide_extension_source(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        extension: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<ExtensionSource>> {
        // admit the present extensions visible from the asking module
        if self.is_absent_symbol(extension) || !self.is_extension_visible(extension, module)? {
            return Ok(None);
        }

        // skip an extension the goal already refused
        let goal = self.goal_key(origin, Goal::Extension { extension }, &[receiver, subject])?;
        if let Some(goal) = &goal
            && let Some(Answer::Extension(false)) = self.answers.get(goal)
        {
            return Ok(None);
        }

        // decide the match live, missing on a re-entered pair
        let active = ActiveGoal::Extension(extension, subject);
        if !self.active.insert(active) {
            return Ok(None);
        }
        let arguments =
            self.match_extension_arguments(origin, module, receiver, subject, extension);
        self.active.swap_remove(&active);
        let arguments = arguments?;

        // remember the verdict for equal asks
        if let Some(goal) = goal {
            self.answers
                .entry(goal)
                .or_insert(Answer::Extension(arguments.is_some()));
        }

        Ok(arguments.map(|arguments| ExtensionSource {
            extension,
            arguments,
        }))
    }

    /// Collect the declared members of one extension matching a selection key.
    pub(in crate::sema) fn matching_extension_members(
        &mut self,
        members: &[dir::DefinitionMember],
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<SmallVec<[DeclaredMember; 2]>> {
        // keep the declared members exposing the selection key
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
    ) -> CompilerResult<Implementation> {
        // intern the requested interface application in this module
        let arguments: SmallVec<[_; 8]> =
            self.type_ids(interface_module, interface.arguments)?.into();
        let arguments = self.intern_type_ids(&arguments)?;
        let module = self.module_id;
        let mut instance = dir::GenericApplication {
            symbol: interface.symbol,
            arguments,
        };

        // name the interface type this module asks with
        let mut instance_module = module;
        let mut interface_type = self.intern_type(dir::Type::Application(instance))?;

        // fill elided interface arguments before matching declared entries
        if let Some(filled) = self.fill_elided_application(interface_type.module_id, &instance)? {
            interface_type = filled;
            let (filled_module, filled_instance) = self.nominal_application(filled)?;
            instance_module = filled_module;
            instance = filled_instance;
        }

        // name the winner or the refusal from a remembered choice at an equal ask
        let asked = match self.fresh_interface_arguments(&instance)? {
            true => self.intern_type(dir::Type::Reference(dir::TypeReference {
                symbol: instance.symbol,
            }))?,
            false => interface_type,
        };
        let goal = match excluded {
            None => self.goal_key(origin, Goal::Implementation(relation), &[receiver, asked])?,
            Some(_) => None,
        };
        let remembered = match &goal {
            Some(goal) => match self.answers.get(goal) {
                Some(Answer::Implement(None)) => {
                    return Ok(Implementation::FAILS);
                }
                Some(Answer::Implement(winner)) => *winner,
                _ => None,
            },
            None => None,
        };

        // break inductive applicability cycles: goals reached from themselves fail
        let active = ActiveGoal::Implementation(relation, receiver, interface_type);
        if !self.active.insert(active) {
            return Ok(Implementation::FAILS);
        }

        // decide the remembered winner alone, else every visible declaration
        let decision = self.decide_visible_extensions(
            origin,
            relation,
            module,
            instance_module,
            receiver,
            &instance,
            excluded,
            remembered,
        );
        self.active.swap_remove(&active);

        // propagate a decision failure after leaving the goal
        let decision = decision?;

        // constrain this site's operands by the winner's target and interface
        if decision.verdict != Verdict::Fails {
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            if let Some(target) = decision.target {
                self.constrain_type(origin, cause, Relation::Storable, receiver, target)?;
            }
            if let Some(matched) = decision.interface {
                self.constrain_type(origin, cause, relation, matched, interface_type)?;
            }
        }

        // remember the decided winner for equal asks
        if let Some(goal) = goal
            && decision.verdict != Verdict::Ambiguous
        {
            self.answers
                .entry(goal)
                .or_insert(Answer::Implement(decision.winner));
        }

        Ok(decision)
    }

    /// Return whether every argument of one asked interface is an open variable without bounds.
    fn fresh_interface_arguments(
        &mut self,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<bool> {
        // require every argument to be an open variable without bounds
        let arguments =
            SmallVec::<[_; 4]>::from_slice(self.type_ids(self.module_id, instance.arguments)?);
        if arguments.is_empty() {
            return Ok(false);
        }
        for argument in arguments {
            let Some(variable) = self.root_variable(argument)? else {
                return Ok(false);
            };
            let state = *self.infer.variable(variable)?;
            if state.parameter.is_some() || !state.lower.is_empty() || !state.upper.is_empty() {
                return Ok(false);
            }
        }

        Ok(true)
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
        only: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Implementation> {
        // collect the implementation declarations this module can see
        let extensions =
            self.implementations_over(origin, module, Some(receiver), interface.symbol)?;

        // load each declaration once
        let mut entries = Vec::new();
        for (extension_symbol, _) in extensions {
            // skip declarations outside the ask, absent gates, and invisible targets
            if Some(extension_symbol) == excluded
                || only.is_some_and(|only| only != extension_symbol)
                || self.is_absent_symbol(extension_symbol)
            {
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

            // skip a declaration whose conformance names other argument heads than the ask
            if !self.conformance_heads_admit(interface_module, interface, &interfaces)? {
                continue;
            }

            // keep the declaration with its template, target, and conformances
            let template = self.symbol_template(extension_symbol)?;
            entries.push((extension_symbol, template, target_type, interfaces));
        }

        // read whether the asked interface still holds an open argument
        let mut is_open_interface = false;
        for argument in self.type_ids(interface_module, interface.arguments)? {
            is_open_interface |= self.type_flags(*argument)?.has_variable();
        }

        // decide each declaration, holding matches that bind outer variables
        let mut undecided = Vec::new();
        let mut is_unproven = false;
        for entry in &entries {
            let (_, template, target_type, interfaces) = entry;
            let verdict = self.decide_candidate(|state| {
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
                    OpenBounds::Decide,
                )?;
                is_unproven |= matches!(matched, ExtensionMatch::Unproven);
                Ok(match matched {
                    ExtensionMatch::Matched(..) => CandidateOutcome::Accepted(()),
                    ExtensionMatch::Unmatched | ExtensionMatch::Unproven => {
                        CandidateOutcome::Rejected(())
                    }
                })
            })?;
            match verdict {
                // coherence leaves one applicable implementation per concrete interface
                Verdict::Holds if !is_open_interface => {
                    undecided.clear();
                    undecided.push(entry);

                    break;
                }
                Verdict::Holds | Verdict::Ambiguous => undecided.push(entry),
                Verdict::Fails => {}
            }
        }

        // select a sole candidate, constraining its match
        if let [(winner, template, target_type, interfaces)] = undecided.as_slice() {
            let matched = self.match_extension_implementation(
                origin,
                relation,
                interface_module,
                receiver,
                receiver,
                interface,
                *template,
                *target_type,
                interfaces,
                OpenBounds::Decide,
            )?;
            if let ExtensionMatch::Matched(substitution, matched_interface) = matched {
                let target = self.substitute_type(*target_type, &substitution)?;

                return Ok(Implementation {
                    verdict: Verdict::Holds,
                    winner: Some(*winner),
                    target: Some(target),
                    interface: Some(matched_interface),
                });
            }
        }

        // fail once every candidate fails, since unproven candidates keep the goal open
        let verdict = if undecided.is_empty() && !is_unproven {
            Verdict::Fails
        } else {
            Verdict::Ambiguous
        };

        Ok(Implementation {
            verdict,
            winner: None,
            target: None,
            interface: None,
        })
    }

    /// Bind one extension's parameters through its declared target.
    ///
    /// A receiver the target leaves unreached binds nothing.
    fn match_extension_target(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        lookup_receiver: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        // require the receiver head to reach the declared target head
        if !self.is_matching_root(lookup_receiver, target_type)? {
            return Ok(None);
        }

        // read the parameters the extension declares
        let parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };

        // bind the extension parameters through the written target
        let target_type = self.normalize(origin, target_type)?;
        let mut substitution = TypeSubstitution::default().with_receiver(receiver);
        let matched = self.bind_extension_target(
            origin,
            &parameters,
            &mut substitution,
            target_type,
            lookup_receiver,
        )?;
        if !matched {
            return Ok(None);
        }

        // open the parameters the target leaves unbound
        self.instantiate_parameters(
            origin,
            &parameters,
            &[],
            substitution,
            TypeArgumentInference::Exact,
        )
    }

    /// Return whether one declaration's conformances can head the asked interface arguments.
    fn conformance_heads_admit(
        &mut self,
        interface_module: ModuleId,
        interface: &dir::GenericApplication,
        interfaces: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        // compare each declared conformance's argument heads against the ask
        let asked =
            SmallVec::<[_; 4]>::from_slice(self.type_ids(interface_module, interface.arguments)?);
        let mut is_named = false;
        for implemented in interfaces {
            let Some((_, instance)) = self.nominal_application_maybe(*implemented)? else {
                return Ok(true);
            };
            if instance.symbol != interface.symbol {
                continue;
            }
            is_named = true;
            let declared = SmallVec::<[_; 4]>::from_slice(
                self.type_ids(implemented.module_id, instance.arguments)?,
            );
            if declared.len() != asked.len() {
                return Ok(true);
            }
            // admit heads that agree, and heads either side leaves open
            let mut admits = true;
            for (declared, asked) in declared.iter().zip(asked.iter()) {
                let declared_root = self.closed_nominal_root(*declared)?;
                let asked_root = self.closed_nominal_root(*asked)?;
                admits &= match (declared_root, asked_root) {
                    (Some(declared), Some(asked)) => declared == asked,
                    _ => true,
                };
            }
            if admits {
                return Ok(true);
            }
        }

        // admit a declaration reaching the interface through inheritance alone
        Ok(!is_named)
    }

    /// Return the declaration one closed nominal type applies, none for every other type.
    fn closed_nominal_root(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // require a closed nominal type
        let ty = self.shallow_resolve(ty)?;
        let flags = self.type_flags(ty)?;
        if flags.has_variable() || flags.has_parameter() || flags.has_this() {
            return Ok(None);
        }

        Ok(self
            .nominal_application_maybe(ty)?
            .map(|(_, instance)| instance.symbol))
    }

    /// Return whether one declaration's conformances reach a requested interface.
    fn has_requested_interface(
        &mut self,
        interfaces: &[dir::GlobalTypeId],
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        // reach the requested interface from any declared conformance
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
        // fail a closed receiver every time once it failed the declared target
        let lookup_receiver = self.shallow_resolve(lookup_receiver)?;
        let is_closed = !self.type_flags(lookup_receiver)?.has_variable();
        let target_key = (target_type, lookup_receiver);
        if is_closed && self.unmatched_targets.contains(&target_key) {
            return Ok(ExtensionMatch::Unmatched);
        }

        // bind the extension parameters through the declared target
        let Some(mut substitution) =
            self.match_extension_target(origin, receiver, lookup_receiver, template, target_type)?
        else {
            if is_closed {
                self.unmatched_targets.insert(target_key);
            }

            return Ok(ExtensionMatch::Unmatched);
        };

        // require the lookup receiver to satisfy the applied target
        let target_type = self.normalize(origin, target_type)?;
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let target = self.substitute_type(target_type, &substitution)?;
        let verdict =
            self.constrain_type(origin, cause, Relation::Storable, lookup_receiver, target)?;
        match verdict {
            Verdict::Holds => {}
            Verdict::Ambiguous => return Ok(ExtensionMatch::Unproven),
            Verdict::Fails => {
                if is_closed && !self.type_flags(target)?.has_variable() {
                    self.unmatched_targets.insert(target_key);
                }

                return Ok(ExtensionMatch::Unmatched);
            }
        }

        // match one declared interface with the requested interface
        let parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
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

        // decide the declared bounds and predicates for the candidate
        if let Some(template) = template {
            let constraints =
                self.substitute_application_constraints(origin, template, &substitution)?;
            for constraint in constraints {
                let closed = self.type_variables(constraint.source)?.is_empty()
                    && self.type_variables(constraint.target)?.is_empty();

                // decide a closed bound in place, rejecting an inapplicable candidate
                if closed {
                    let id = self.queue_relation(constraint)?;
                    self.solve_relation(id, Settle::Possible)?;

                    // a failed bound rejects, an undecided one stays unproven
                    match self.fulfill.checks.result(id)? {
                        Some(CheckOutcome::Holds) => {}
                        Some(_) => return Ok(ExtensionMatch::Unmatched),
                        None => return Ok(ExtensionMatch::Unproven),
                    }
                }
                // keep an open bound while deciding overlap, where it may still hold
                else if open_bounds == OpenBounds::Keep {
                    self.queue_relation(constraint)?;
                }
                // probe an open bound, keeping an undecided one open
                else {
                    let verdict = self.decide_candidate(|state| {
                        let id = state.queue_relation(constraint)?;
                        state.solve_relation(id, Settle::Possible)?;
                        Ok(CandidateOutcome::<(), ()>::Accepted(()))
                    })?;
                    if verdict == Verdict::Fails {
                        return Ok(ExtensionMatch::Unmatched);
                    }
                    self.push_relation(constraint)?;
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

        // try each ancestor application in the heritage closure
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

        // read this declaration's own parameters
        let other_template = self.symbol_template(other)?;
        let template = self.symbol_template(symbol)?;
        let parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };

        // enumerate the declared inhabitants of each interface bounded parameter
        let predicates = self.template_predicates(template);
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

            // take the inhabitants of the first bound that names an interface
            let mut witnesses = None;
            for bound in bounds {
                let mut visited = FxIndexSet::default();
                witnesses = self.interface_inhabitants(origin, module, bound, &mut visited)?;
                if witnesses.is_some() {
                    break;
                }
            }

            // stop once a bound turns out uninhabited
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

    /// Decide one overlap under a witness assignment, returning the witnessed receiver.
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
        let mut receiver_witness = None;
        let verdict = self.decide_candidate(|state| {
            // open this declaration's parameters and fix the witnessed ones
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
                state.commit_solution(variable, *witness)?;
            }

            // apply the witnessed substitution to the target and the interface
            let receiver = state.substitute_type(target_type, &substitution)?;
            let interface = state.substitute_type(interface_type, &substitution)?;
            let (interface_module, interface) = state.nominal_application(interface)?;

            // bind the witnessed instantiation through the other implementation
            let matched = state.match_extension_implementation(
                origin,
                Relation::Storable,
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
                    state.constrain_type(
                        constraint.origin,
                        constraint.cause,
                        constraint.relation,
                        constraint.source,
                        constraint.target,
                    )?;
                    let variables = state.type_variables(constraint.source)?;
                    state.settle_variables(&variables, Settle::All)?;
                    let verdict = state.decide_relation(
                        constraint.origin,
                        constraint.relation,
                        constraint.source,
                        constraint.target,
                    )?;
                    if verdict != Verdict::Holds {
                        return Ok(CandidateOutcome::Rejected(()));
                    }
                }
            }

            // record the resolved receiver as this assignment's witness
            receiver_witness = Some(state.deeply_resolve(origin, receiver)?);

            Ok(CandidateOutcome::Accepted(()))
        });

        Ok(match verdict? {
            Verdict::Holds => receiver_witness,
            _ => None,
        })
    }

    /// Return the declared types implementing one interface bound, or `None` for every other bound.
    fn interface_inhabitants(
        &mut self,
        origin: Origin,
        module: ModuleId,
        constraint: dir::GlobalTypeId,
        visited: &mut FxIndexSet<dir::GlobalSymbolId>,
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        // read the bound's interface, other bounds leave the parameter open
        let Some((_, instance)) = self.nominal_application_maybe(constraint)? else {
            return Ok(None);
        };
        if !self.symbol_kind(instance.symbol)?.is_interface() {
            return Ok(None);
        }
        if !visited.insert(instance.symbol) {
            return Ok(Some(Vec::new()));
        }

        // take the target of each rooted implementation, descending through the blankets
        let mut inhabitants = Vec::new();
        for (implementor, _) in self.implementations_over(origin, module, None, instance.symbol)? {
            let Some(dir::Definition::Extension(extension)) = self.definition(implementor)? else {
                continue;
            };
            match extension.target {
                // a rooted implementation inhabits the interface with its own target
                dir::ExtensionTarget::Rooted { ty, .. } => {
                    if !inhabitants.contains(&ty) {
                        inhabitants.push(ty);
                    }
                }
                // a blanket implementation inhabits through its own bound
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
                    let Some(bounded) =
                        self.interface_inhabitants(origin, module, bound, visited)?
                    else {
                        continue;
                    };
                    for ty in bounded {
                        if !inhabitants.contains(&ty) {
                            inhabitants.push(ty);
                        }
                    }
                }
            }
        }

        Ok(Some(inhabitants))
    }

    /// Instantiate one matched extension's declared and conformance members, paired with keys.
    pub(in crate::sema) fn extension_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        source: &ExtensionSource,
        space: dir::MemberSpace,
        key: Option<dir::StaticKey>,
    ) -> CompilerResult<Vec<(dir::StaticKey, MemberCandidate)>> {
        // read the extension's target and the interfaces it implements
        let extension_symbol = source.extension;
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
            if !self.is_loaded_module(interface.symbol.module_id) {
                self.import_external_module(interface.symbol.module_id)?;
            }
            let Some(dir::Definition::Interface(definition)) =
                self.definition_maybe(interface.symbol)
            else {
                continue;
            };

            // take each default member whose key the extension left open
            let first_default = members.len();
            for member in &definition.members {
                if member.space() != space || !self.definition_member_has_default(member) {
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

        // stop once the extension exposes no member under the key
        if members.is_empty() {
            return Ok(Vec::new());
        }

        // bind each template parameter to its decided argument, keeping erased slots
        let arguments = &source.arguments;
        let template = self.symbol_template(extension_symbol)?;
        let mut substitution = TypeSubstitution::default();
        let mut site_parameters = SmallVec::<[GenericParameterId; 2]>::new();
        if let Some(template) = template {
            let parameters = self.generic_template_parameters(template)?;
            for (parameter, argument) in parameters.iter().zip(arguments.iter()) {
                // keep one slot per erased parameter across the arguments
                for erased in self.erased_parameters(*argument)? {
                    if !site_parameters.contains(&erased) {
                        site_parameters.push(erased);
                    }
                }
                substitution.bind(*parameter, *argument)?;
            }
        }

        // bind this to the matched extension target
        let this = self.substitute_type(target_type, &substitution)?;
        let mut substitution = substitution.with_receiver(this);

        // carry the declared bounds and the target match to each use site while operands stay open
        let mut is_open = !site_parameters.is_empty()
            || (self.type_flags(receiver)? | self.type_flags(subject)?).has_variable();
        for argument in arguments {
            is_open |= self.type_flags(*argument)?.has_variable();
        }
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
            let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
            target = Some(RelationCheck {
                origin,
                relation: Relation::Storable,
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
            if let Some(declared) = candidate.declaration_mut() {
                declared.bounds = bounds.clone();
                declared.target = target;
                declared.site_parameters = site_parameters.clone();
            }
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
            return Ok(Vec::new());
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
            None => return Ok(Vec::new()),
        };

        // require the extension to be visible from the asking module
        if !extension.is_visible_from(module) {
            return Ok(Vec::new());
        }

        // require the extension to be rooted at this declaration
        if extension.target.root() != Some(dir::TypeRoot::Declaration(symbol)) {
            return Ok(Vec::new());
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
            return Ok(Vec::new());
        }

        // bind extension parameters through the written receiver arguments
        let substitution = match arguments.is_empty() {
            // a bare declaration name reads the statics of its default form
            true => {
                let chain = self.form_chain(origin, target_type)?;
                let written = chain
                    .ownership_form()
                    .and_then(|form| form.form.ownership());
                if written.is_some() && written != self.default_ownership(origin, chain.base())? {
                    return Ok(Vec::new());
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
                let verdict = self.decide_candidate(|state| {
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
                let matched = match verdict {
                    Verdict::Fails => None,
                    Verdict::Holds | Verdict::Ambiguous => self.match_extension_subject(
                        origin,
                        applied,
                        applied,
                        template,
                        target_type,
                        UnboundParameters::Open,
                    )?,
                };
                let Some(substitution) = matched else {
                    return Ok(Vec::new());
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

        Ok(candidates)
    }

    /// Return the substituted member candidates one matched extension exposes per key.
    pub(in crate::sema) fn extension_member_candidates(
        &mut self,
        origin: Origin,
        extension_symbol: dir::GlobalSymbolId,
        substitution: &TypeSubstitution,
        members: &[DeclaredMember],
    ) -> CompilerResult<Vec<(dir::StaticKey, MemberCandidate)>> {
        // rank blanket declarations behind rooted extensions
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
            let access = self.projected_member_access(origin, substitution.receiver, member, ty)?;

            // substitute static projections through the same extension instance
            let written = match self.static_value(member.symbol) {
                Some(written) => Some(self.substitute_type(written, substitution)?),
                None => None,
            };

            // carry the solved extension arguments onto the candidate
            let generic_arguments = self.resolved_argument_bindings(&substitution.bindings)?;
            let requirement = self.requirement_interface(extension_symbol, member.key)?;
            let mut declared = DeclaredCandidate::new(
                member.symbol,
                extension_symbol,
                member_origin,
                generic_arguments,
            );
            declared.requirement = requirement;
            declared.value_type = written;
            let candidate = MemberCandidate::declared(declared, member, access, callable);

            candidates.push((member.key, candidate));
        }

        Ok(candidates)
    }

    /// Collect the applied interfaces that extensions conform one owner to under one key.
    pub(in crate::sema) fn conformed_interfaces(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        // collect the extensions over the owner's heads
        let module = self.module_id;
        let heads = self.receiver_heads(origin, owner)?;
        let symbols = self.extensions_over(module, &heads)?;

        // walk each extension declaration visible from this module
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

            // keep conformances whose interface declares the projected member
            let mut declaring = SmallVec::<[dir::GlobalTypeId; 2]>::new();
            for conformance in conformances {
                if self.has_associated_type(conformance, key)? {
                    declaring.push(conformance);
                }
            }
            if declaring.is_empty() {
                continue;
            }

            // require the declared target to bind every extension parameter
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

            // apply the matched substitution to each declaring interface
            for interface in declaring {
                let applied = self.substitute_type(interface, &substitution)?;
                if !interfaces.contains(&applied) {
                    interfaces.push(applied);
                }
            }
        }

        Ok(interfaces)
    }

    /// Match one extension target against a receiver, probing its dereference steps.
    pub(in crate::sema) fn match_extension(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        // match each dereference step of the receiver, then the widened subject, against the target
        let value = Value {
            ty: receiver,
            node: None,
            place: None,
            is_fresh: false,
        };
        let mut subjects = self
            .builtin_steps(origin, value)?
            .into_iter()
            .map(|step| step.ty)
            .collect::<SmallVec<[_; 4]>>();
        if !subjects.contains(&subject) {
            subjects.push(subject);
        }
        for subject in subjects {
            let substitution = self.match_extension_subject(
                origin,
                receiver,
                subject,
                template,
                target_type,
                UnboundParameters::Open,
            )?;
            if substitution.is_some() {
                return Ok(substitution);
            }
        }

        Ok(None)
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
        if !self.is_matching_root(subject, target_type)? {
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

        // attach the use-site receiver to the substitution
        let substitution = substitution.with_receiver(receiver);

        // require the subject to satisfy the bound target
        let target = self.substitute_type(target_type, &substitution)?;
        let target = self.normalize(origin, target)?;
        if self.decide_relation(origin, Relation::Subtype, subject, target)? == Verdict::Fails {
            return Ok(None);
        }

        // filter on the declared constraints, keeping their obligations pending
        if let Some(template) = template
            && !self.is_substitution_viable(origin, template, &substitution)?
        {
            return Ok(None);
        }

        Ok(Some(substitution))
    }
}
