use std::sync::Arc;

use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    BodyState, CandidateOutcome, Cause, CauseKind, DeclaredMember, GenericParameterId,
    GenericTemplateId, LookupReceiver, MemberCandidate, MemberLookup, MemberSubject, MemberTable,
    Origin, ReceiverSteps, Relation, TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult};

impl BodyState<'_, '_> {
    /// Look up one extension member on a declaration reference.
    pub(in crate::check) fn lookup_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // read the grouped table, built once per closed subject
        let members =
            self.subject_extension_members(origin, module, receiver, subject, symbol, space)?;
        let candidates = members.get(&key).cloned().unwrap_or_default();

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Return every extension member of one subject, grouped by key.
    ///
    /// Extension applicability is key independent, so the table matches each
    /// visible extension once per subject.
    pub(in crate::check) fn subject_extension_members(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> CompilerResult<MemberTable> {
        // decide the table once per closed subject: candidates are pure
        //  functions of the interned subject and the module's scope
        let flags = self.check.type_flags(receiver)? | self.check.type_flags(subject)?;
        let is_closed = !flags.has_variable();
        let key = MemberSubject {
            module,
            receiver,
            subject,
            symbol,
            space,
        };

        // serve the memo
        if is_closed && let Some(members) = self.check.members.get(&key) {
            return Ok(members.clone());
        }

        // collect the extension declarations this module can see
        let mut extensions = self.visible_extensions(module, symbol)?;

        // primitive receivers also reach extensions declared over their format
        for ty in [receiver, subject] {
            let value = self.strip_form(origin, ty)?;
            let dir::Type::Primitive(primitive) = self.ty(value)? else {
                continue;
            };

            for symbol in self.primitive_extensions(primitive)? {
                if !extensions.contains(&symbol) {
                    extensions.push(symbol);
                }
            }
        }

        // match each extension once, collecting first declarations per symbol
        let mut members = FxIndexMap::<dir::StaticKey, Vec<MemberCandidate>>::default();
        let mut seen = FxIndexSet::default();
        for extension_symbol in extensions {
            let candidates = self.extension_subject_candidates(
                origin,
                module,
                receiver,
                subject,
                extension_symbol,
                space,
            )?;
            for (key, candidate) in candidates {
                if !seen.insert(candidate.symbol) {
                    continue;
                }

                members.entry(key).or_default().push(candidate);
            }
        }

        let members = Arc::new(members);

        // decide the closed table
        if is_closed {
            self.check.members.insert(key, members.clone());
        }

        Ok(members)
    }

    /// Return the implicit extensions declared for one primitive type.
    pub(in crate::check) fn primitive_extensions(
        &self,
        primitive: dir::PrimitiveType,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // primitive extensions live only in the declared environment
        let environment =
            self.check
                .environment_declared
                .as_ref()
                .ok_or(CompilerError::Internal {
                    message: "primitive member lookup has no declared environment".to_string(),
                })?;
        let extensions = match environment.extensions_by_primitive.get(&primitive) {
            Some(extensions) => extensions.iter().copied().collect(),
            None => SmallVec::new(),
        };

        Ok(extensions)
    }

    /// Look up one static extension member on a declaration reference.
    pub(in crate::check) fn lookup_static_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        // collect the extension declarations this module can see
        let extensions = self.visible_extensions(module, symbol)?;
        let mut candidates = Vec::new();
        let mut seen = FxIndexSet::default();

        // visit extension declarations in resolution order
        for extension_symbol in extensions {
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

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Collect extension symbols visible from one module for one target.
    pub(in crate::check) fn visible_extensions(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        let mut symbols = SmallVec::new();

        // collect extensions declared beside the looking module
        if let Some(state) = self.module_maybe(module) {
            symbols.extend(state.target_extensions(target));
            symbols.extend(state.blanket_extensions());
        }

        // collect inherent extensions beside the target declaration
        if target.module_id != module {
            if let Some(state) = self.module_maybe(target.module_id) {
                symbols.extend(state.target_extensions(target));
                symbols.extend(state.blanket_extensions());
            }

            if let Some(external) = self.external_modules.get(&target.module_id) {
                symbols.extend(external.definitions.target_extensions(target));
                symbols.extend(external.definitions.blanket_extensions());
            }
        }

        // collect the implicit extensions rooted at the target
        if let Some(environment) = &self.check.environment_declared {
            if let Some(implicit) = environment.extensions_by_root.get(&target) {
                for symbol in implicit {
                    if !symbols.contains(symbol) {
                        symbols.push(*symbol);
                    }
                }
            }

            // collect the implicit open blanket extensions
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

        Ok(symbols)
    }

    /// Collect the declared members of one extension matching a selection key.
    pub(in crate::check) fn matching_extension_members(
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
    pub(in crate::check) fn decide_extension_implementation(
        &mut self,
        origin: Origin,
        relation: Relation,
        _module: ModuleId,
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

        // fill elided interface arguments before matching declared rows
        if let Some(filled) = self.fill_elided_application(interface_type.module_id, &instance)? {
            interface_type = filled;
            let (filled_module, filled_instance) = self.nominal_application(filled)?;
            instance_module = filled_module;
            instance = filled_instance;
        }

        // serve repeated goals from the selection memo, replaying the
        //  winning implementation so its inference binds this goal's variables
        let flags = self.type_flags(receiver)? | self.type_flags(interface_type)?;
        let scope = if flags.has_parameter() || flags.has_this() {
            self.assuming_scope(origin)?
        } else {
            None
        };

        let is_settled = !flags.has_variable();
        let key = (relation, receiver, interface_type, scope);
        if excluded.is_none()
            && is_settled
            && let Some((verdict, winner)) = self.check.extensions.get(&key).copied()
        {
            match winner {
                Some(winner) => {
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
                None => return Ok(verdict),
            }
        }

        // break inductive applicability cycles: goals reached from themselves fail
        let goal = (relation, receiver, interface_type);
        if !self.check.deciding_extensions.insert(goal) {
            return Ok(Verdict::Fails);
        }

        // try each visible implementation declaration
        let decision = self.decide_visible_extensions(
            origin,
            relation,
            module,
            instance_module,
            receiver,
            &instance,
            excluded,
        );
        self.check.deciding_extensions.swap_remove(&goal);

        // remember settled decided goals and their winner for replay
        let (decision, winner) = decision?;
        if excluded.is_none() && is_settled && decision != Verdict::Ambiguous {
            self.check.extensions.insert(key, (decision, winner));
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
            )?;
            Ok(match matched {
                Some(_) => CandidateOutcome::Accepted(()),
                None => CandidateOutcome::Rejected(()),
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

        // try each visible implementation declaration
        let mut speculative = Vec::new();
        for extension_symbol in extensions {
            if Some(extension_symbol) == excluded {
                continue;
            }

            if self.is_absent_symbol(extension_symbol) {
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
            if !self.reaches_requested_interface(&interfaces, interface.symbol)? {
                continue;
            }

            // match in one confirming attempt
            let template = self.symbol_template(extension_symbol)?;
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
                    template,
                    target_type,
                    &interfaces,
                )?;
                solves_outer = state
                    .check
                    .infer
                    .solves_variable_before(trail_from, variables);
                Ok(match (matched, solves_outer) {
                    (Some(_), false) => CandidateOutcome::Accepted(()),
                    _ => CandidateOutcome::Rejected(()),
                })
            })?;

            if matched.is_some() {
                return Ok((Verdict::Holds, Some(extension_symbol)));
            }

            // hold a match that solves outer variables for uniqueness:
            //  only a sole applicable candidate may commit their solutions
            if solves_outer {
                speculative.push((extension_symbol, template, target_type, interfaces));
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
                )?;
                Ok(match matched {
                    Some(_) => CandidateOutcome::Accepted(()),
                    None => CandidateOutcome::Rejected(()),
                })
            })?;

            if matched.is_some() {
                return Ok((Verdict::Holds, Some(*extension_symbol)));
            }
        }

        Ok(match speculative.is_empty() {
            false => (Verdict::Ambiguous, None),
            true => (Verdict::Fails, None),
        })
    }

    /// Return whether one declaration's conformances reach a requested interface.
    fn reaches_requested_interface(
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
    pub(in crate::check) fn match_extension_implementation(
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
    ) -> CompilerResult<Option<(TypeSubstitution, dir::GlobalTypeId)>> {
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
            return Ok(None);
        }

        // require the lookup receiver to satisfy the applied target
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let target = self.substitute_type(target_type, &substitution)?;
        if !self.constrain_type(origin, cause, Relation::Assignable, lookup_receiver, target)? {
            return Ok(None);
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
            return Ok(None);
        };

        // register the declared bounds and predicates with the candidate
        // NOTE: coherence forbids overlapping implementations, so the solver
        //  can defer the sole candidate's constraints to settle
        if let Some(template) = template {
            let constraints =
                self.substitute_application_constraints(origin, template, &substitution)?;
            for constraint in constraints {
                self.check.register_constraint(constraint);
            }
        }

        Ok(Some((substitution, implementation)))
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

    /// Collect visible extensions that may implement an interface for one receiver.
    pub(in crate::check) fn visible_implementation_extensions(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        // collect the extensions in scope for the receiver's own head
        let mut parameters = SmallVec::new();
        let mut symbols = SmallVec::new();
        self.collect_implementation_extensions(
            origin,
            module,
            receiver,
            &mut parameters,
            &mut symbols,
        )?;

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

        Ok(symbols)
    }

    /// Collect the implementation extension candidates for one receiver head.
    fn collect_implementation_extensions(
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
                    let collected = self.collect_implementation_extensions(
                        origin, module, bound, parameters, symbols,
                    );
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
                    self.collect_implementation_extensions(
                        origin, module, element, parameters, symbols,
                    )?;
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
                for symbol in self.visible_extensions(module, scope)? {
                    if !symbols.contains(&symbol) {
                        symbols.push(symbol);
                    }
                }

                Ok(())
            }
        }
    }

    /// Collect blanket extension symbols visible from one module.
    pub(in crate::check) fn visible_blanket_extensions(
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
    fn extension_subject_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> CompilerResult<Vec<(dir::StaticKey, MemberCandidate)>> {
        // skip extensions removed by statically false gates
        if self.is_absent_symbol(extension_symbol) {
            return Ok(Vec::new());
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
            None => return Ok(Vec::new()),
        };

        if !extension.is_visible_from(module) {
            return Ok(Vec::new());
        }

        // close recursive lookups of this extension coinductively
        if !self.check.extending.insert((extension_symbol, subject)) {
            return Ok(Vec::new());
        }

        // collect the candidates, then release the re-entry mark
        let result =
            self.collect_extension_candidates(origin, receiver, subject, extension_symbol, space);
        self.check
            .extending
            .swap_remove(&(extension_symbol, subject));

        result
    }

    /// Collect one extension's declared and conformance member candidates.
    fn collect_extension_candidates(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
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
            let Some(dir::Definition::Interface(definition)) = self.definition(interface.symbol)?
            else {
                continue;
            };

            // take each default member whose key the extension left open
            let first_default = members.len();
            for member in &definition.members {
                if member.space() != space || !member.is_default() {
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

        // open extension generics and match the receiver once
        let template = self.symbol_template(extension_symbol)?;
        let result = self.confirm_candidate(|state| {
            let Some(mut substitution) =
                state.match_extension(origin, receiver, subject, template, target_type)?
            else {
                return Ok(CandidateOutcome::Rejected(()));
            };

            // read the checked type of each member the matched extension exposes
            state.resolve_declared_types(&mut members)?;

            // rewrite conformance interface parameters into subject terms
            for (interface_module, interface) in &conformances {
                let interface_substitution =
                    state.instance_substitution(*interface_module, interface)?;
                for binding in interface_substitution.bindings {
                    let argument = state.substitute_type(binding.argument, &substitution)?;
                    substitution.bind(binding.parameter, argument)?;
                }
            }

            // substitute the matched arguments into every declared member
            let candidates = state.extension_member_candidates(
                origin,
                extension_symbol,
                &substitution,
                &members,
            )?;
            if candidates.is_empty() {
                return Ok(CandidateOutcome::Rejected(()));
            }

            Ok(CandidateOutcome::Accepted(candidates))
        })?;

        Ok(result.unwrap_or_default())
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

        if extension.target.root() != Some(symbol) {
            return Ok(MemberLookup::Missing);
        }

        // read the extension target and clone only the members matching the requested key
        let target_type = extension.target.r#type();
        let keyed = keyed_members(&extension.members, dir::MemberSpace::Static, key);
        if keyed.is_empty() {
            return Ok(MemberLookup::Missing);
        }

        let members = self.matching_extension_members(&keyed, dir::MemberSpace::Static, key)?;
        if members.is_empty() {
            return Ok(MemberLookup::Missing);
        }

        // bind extension parameters through the written receiver arguments
        let substitution = match arguments.is_empty() {
            // a bare declaration name leaves every extension parameter open for the call
            true => TypeSubstitution::default(),
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
    pub(in crate::check) fn extension_member_candidates(
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
            let callable = member.callable_type(origin.module(), extension_symbol, ty, self)?;
            let access_type = member.access_type(self, ty)?;
            let access_type = self.projected_member_type(
                origin,
                substitution.receiver,
                member.role,
                access_type,
            )?;

            // substitute static projections through the same extension instance
            let written = match self.static_value(member.symbol) {
                Some(written) => {
                    let written = self.substitute_type(written, substitution)?;

                    Some(written)
                }
                written => written,
            };

            // carry the solved extension arguments onto the candidate
            let generic_arguments = self.settled_argument_bindings(&substitution.bindings)?;
            let candidate = MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                origin: member_origin,
                space: member.space,
                role: member.role,
                kind: member.kind,
                is_writable: member.is_writable,
                access_type,
                callable,
                is_optional: member.is_optional,
                generic_arguments,
                value: member.value,
                value_type: written,
                receiver: LookupReceiver::Direct(ReceiverSteps::new()),
            };

            candidates.push((member.key, candidate));
        }

        Ok(candidates)
    }

    /// Match one extension target against a receiver and its widened lookup subject.
    pub(in crate::check) fn match_extension(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        // match the exact receiver first, then its widened lookup subject
        let substitution =
            self.match_extension_subject(origin, receiver, receiver, template, target_type)?;
        if substitution.is_none() && subject != receiver {
            return self.match_extension_subject(origin, receiver, subject, template, target_type);
        }

        Ok(substitution)
    }

    /// Match one lookup subject against an extension target and its constraints.
    pub(in crate::check) fn match_extension_subject(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        self.match_extension_subject_uncached(origin, receiver, subject, template, target_type)
    }

    /// Match one lookup subject against an extension target without the memo.
    fn match_extension_subject_uncached(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<Option<TypeSubstitution>> {
        // written class names match as their canonical declaration instance
        let subject = match self.ty(subject)? {
            dir::Type::Reference(reference) => {
                let instance = self.declaration_instance(reference.symbol)?;

                self.intern_type(dir::Type::Application(instance))?
            }
            _ => subject,
        };

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
        }

        let substitution = substitution.with_receiver(receiver);

        // require the subject to satisfy the bound target
        let target = self.substitute_type(target_type, &substitution)?;
        if !self.evaluate_relation(origin, Relation::Assignable, subject, target)? {
            return Ok(None);
        }

        // require the extension declaration's substituted constraints
        if let Some(template) = template
            && !self.relate_substitution_constraints(origin, template, &substitution)?
        {
            return Ok(None);
        }

        Ok(Some(substitution))
    }
}

/// Clone the members declared under one space and key.
fn keyed_members(
    members: &[dir::DefinitionMember],
    space: dir::MemberSpace,
    key: dir::StaticKey,
) -> SmallVec<[dir::DefinitionMember; 2]> {
    members
        .iter()
        .filter(|member| member.space() == space && member.key() == Some(key))
        .cloned()
        .collect()
}
