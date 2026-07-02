use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, DeclaredMember, Dependency, GenericTemplateId, MemberCandidate,
    MemberLookup, Origin, Relation, TypeSubstitution, answer,
};

impl CheckState<'_> {
    /// Look up one extension member on a declaration reference.
    pub(in crate::check) fn lookup_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let extensions = self.visible_extensions(module, instance.symbol);

        // visit extension declarations in resolution order
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();
        for extension_symbol in extensions {
            let lookup = answer!(self.lookup_extension_symbol_member(
                origin,
                module,
                receiver,
                extension_symbol,
                space,
                key,
            )?);

            match lookup {
                MemberLookup::Found(found) => {
                    // collect first declarations of each member symbol
                    for candidate in found {
                        if candidate.symbol.is_some_and(|symbol| !seen.insert(symbol)) {
                            continue;
                        }
                        candidates.push(candidate);
                    }
                }
                MemberLookup::Missing | MemberLookup::Field(_) => {}
            }
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
    }

    /// Look up one static extension member on a declaration reference.
    pub(in crate::check) fn lookup_static_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        let extensions = self.visible_extensions(module, symbol);
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();

        // visit extension declarations in resolution order
        for extension_symbol in extensions {
            let lookup = answer!(self.lookup_one_static_extension(
                origin,
                module,
                symbol,
                extension_symbol,
                key
            )?);

            match lookup {
                MemberLookup::Found(found) => {
                    for candidate in found {
                        if candidate.symbol.is_some_and(|symbol| !seen.insert(symbol)) {
                            continue;
                        }
                        candidates.push(candidate);
                    }
                }
                MemberLookup::Missing | MemberLookup::Field(_) => {}
            }
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
    }

    /// Collect extension symbols visible from one module for one target.
    pub(in crate::check) fn visible_extensions(
        &self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        let mut symbols = SmallVec::new();

        // collect extensions declared beside the looking module
        if let Some(state) = self.modules.get(&module) {
            let working = &state.definitions;
            symbols.extend(working.target_extensions(target).iter().copied());
            symbols.extend(working.blanket_extensions().iter().copied());
        }

        // collect inherent extensions beside the target declaration
        if target.module_id != module {
            if let Some(state) = self.modules.get(&target.module_id) {
                let working = &state.definitions;
                symbols.extend(working.target_extensions(target).iter().copied());
                symbols.extend(working.blanket_extensions().iter().copied());
            }
            if let Some(external) = self.external_modules.get(&target.module_id) {
                symbols.extend(external.definitions.target_extensions(target));
                symbols.extend(external.definitions.blanket_extensions());
            }
        }

        // collect explicitly imported extension symbols
        for (_, symbol) in self.module(module).resolved.imports.symbol_targets() {
            if self
                .definition(symbol)
                .is_some_and(|definition| matches!(definition, dir::Definition::Extension(_)))
                && !symbols.contains(&symbol)
            {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Decide whether a visible extension implements one interface for a receiver.
    pub(in crate::check) fn decide_extension_implementation(
        &mut self,
        origin: Origin,
        module: ModuleId,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let extensions = self.visible_receiver_extensions(module, receiver)?;

        // try each visible implementation declaration
        for extension_symbol in extensions {
            if self.is_absent_symbol(extension_symbol) {
                continue;
            }
            let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)
            else {
                continue;
            };
            if !extension.is_visible_from(module) {
                continue;
            }

            // skip extensions without interface declarations
            let implements = extension.implements.clone();
            if implements.is_empty() {
                continue;
            }

            // take declaration inputs before entering the candidate probe
            let target_type = extension.target.r#type();
            let template = self.symbol_template(extension_symbol);

            // match the extension target under a probe
            let probe = self.begin_probe();
            let result = self.match_extension_target(origin, receiver, template, target_type);

            let matched = match result {
                Ok(Answer::Ready(Some(substitution))) => self.extension_implements_interface(
                    origin,
                    module,
                    interface_module,
                    &substitution,
                    &implements,
                    interface,
                ),
                Ok(Answer::Ready(None)) => Ok(Answer::Ready(false)),
                Ok(Answer::Pending(pending)) => Ok(Answer::Pending(pending)),
                Err(error) => Err(error),
            };

            let matched = matched?;
            match matched {
                Answer::Ready(true) => {
                    self.commit_probe(probe);

                    return Ok(Answer::Ready(true));
                }
                Answer::Ready(false) => {
                    self.reject_probe(probe);
                }
                Answer::Pending(pending) => {
                    self.reject_probe(probe);
                    blockers.extend(self.live_blockers(pending));
                }
            }
        }

        Ok(Answer::ready_unless_blocked(false, blockers))
    }

    /// Collect extension symbols visible for one receiver type.
    pub(in crate::check) fn visible_receiver_extensions(
        &mut self,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        let Some((_, instance)) = self.apparent_instance(receiver)? else {
            return Ok(self.visible_blanket_extensions(module));
        };
        let scope = instance.symbol;

        if !self.is_component_module(scope.module_id) {
            self.import_external_module(scope.module_id)?;
        }

        Ok(self.visible_extensions(module, scope))
    }

    /// Collect blanket extension symbols visible from one module.
    fn visible_blanket_extensions(&self, module: ModuleId) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        let mut symbols = SmallVec::new();

        // collect local blanket extensions
        if let Some(state) = self.modules.get(&module) {
            symbols.extend(state.definitions.blanket_extensions().iter().copied());
        }

        // collect imported extension symbols
        for (_, symbol) in self.module(module).resolved.imports.symbol_targets() {
            let is_extension = self
                .definition(symbol)
                .is_some_and(|definition| matches!(definition, dir::Definition::Extension(_)));
            if is_extension && !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Look up matching members from one extension declaration.
    pub(in crate::check) fn lookup_extension_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        // skip extensions removed by statically false gates
        if self.is_absent_symbol(extension_symbol) {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }

        // read the extension members
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(Answer::Ready(MemberLookup::Missing));
        };
        if !extension.is_visible_from(module) {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }
        let target_type = extension.target.r#type();
        let definition_members = extension.members.clone();
        let mut matched = SmallVec::<[_; 2]>::new();
        for member in &definition_members {
            let Some(member) = answer!(self.declared_member(member)?) else {
                continue;
            };
            if member.matches(space, key) {
                matched.push(member);
            }
        }
        if matched.is_empty() {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }
        let members = matched;

        // open extension generics and match the receiver
        let template = self.symbol_template(extension_symbol);
        let probe = self.begin_probe();
        let result = self.match_extension(
            origin,
            receiver,
            extension_symbol,
            template,
            target_type,
            &members,
        );
        match result {
            Ok(Answer::Ready(Some(candidates))) => {
                self.commit_probe(probe);

                Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
            }
            Ok(Answer::Ready(None)) => {
                self.reject_probe(probe);

                Ok(Answer::Ready(MemberLookup::Missing))
            }
            // blockers that died with the probe cannot wake this extension
            Ok(Answer::Pending(blockers)) => {
                self.reject_probe(probe);

                let blockers = self.live_blockers(blockers);
                if blockers.is_empty() {
                    Ok(Answer::Ready(MemberLookup::Missing))
                } else {
                    Ok(Answer::Pending(blockers))
                }
            }
            Err(error) => {
                self.reject_probe(probe);

                Err(error)
            }
        }
    }

    /// Look up matching static members from one extension declaration.
    fn lookup_one_static_extension(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        extension_symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<MemberLookup>> {
        // skip extensions removed by statically false gates
        if self.is_absent_symbol(extension_symbol) {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }

        // read matching static members from extensions of this declaration
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(Answer::Ready(MemberLookup::Missing));
        };
        if !extension.is_visible_from(module) {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }
        if extension.target.root() != Some(symbol) {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }
        let definition_members = extension.members.clone();
        let mut members = SmallVec::<[_; 2]>::new();
        for member in &definition_members {
            let Some(member) = answer!(self.declared_member(member)?) else {
                continue;
            };
            if member.matches(dir::MemberSpace::Static, key) {
                members.push(member);
            }
        }
        if members.is_empty() {
            return Ok(Answer::Ready(MemberLookup::Missing));
        }

        self.lookup_parameterized_static_extension(origin, extension_symbol, &members)
    }

    /// Look up static extension members with unspecialized extension parameters.
    fn lookup_parameterized_static_extension(
        &mut self,
        origin: Origin,
        extension_symbol: dir::GlobalSymbolId,
        members: &[DeclaredMember],
    ) -> CompilerResult<Answer<MemberLookup>> {
        let arguments = self.extension_parameter_arguments(origin, extension_symbol)?;

        // expose matching static members for later call inference
        let mut candidates = Vec::new();
        for member in members {
            let Some(ty) = member.ty else {
                continue;
            };
            let ty = member.value_type(self, ty)?;
            let written = member.symbol.and_then(|symbol| self.static_value(symbol));

            let generic_arguments =
                self.symbol_generic_argument_bindings(extension_symbol, &arguments)?;
            let ty = self.resolve_type_variables(origin.module(), ty)?;
            let ty = answer!(self.projected_member_type(origin, None, member.role, ty)?);

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                role: member.role,
                ty,
                generic_arguments,
                value: member.value,
                value_type: written,
            });
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
    }

    /// Return generic parameter placeholder arguments for one extension head.
    fn extension_parameter_arguments(
        &mut self,
        origin: Origin,
        extension_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let Some(template) = self.symbol_template(extension_symbol) else {
            return Ok(Vec::new());
        };
        let module = origin.module();

        let mut arguments = Vec::new();
        for parameter in self.generic_template_parameters(template) {
            arguments.push(self.intern_type(module, dir::Type::Parameter(parameter))?);
        }

        Ok(arguments)
    }

    /// Match one extension target against a receiver under an active probe.
    pub(in crate::check) fn match_extension_target(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<TypeSubstitution>>> {
        // bind extension generics from the receiver target pattern
        let substitution = match template {
            Some(template) => {
                let parameters = self.generic_template_parameters(template);
                let Some(substitution) = answer!(self.match_generic_pattern(
                    origin,
                    &parameters,
                    target_type,
                    receiver
                )?) else {
                    return Ok(Answer::Ready(None));
                };

                substitution
            }
            None => Default::default(),
        };
        let substitution = substitution.with_receiver(receiver);

        // prove the receiver satisfies the completed target
        let target_type = self.substitute_type(origin.module(), target_type, &substitution)?;
        if !answer!(self.decide_relation(origin, Relation::Assignable, receiver, target_type)?) {
            return Ok(Answer::Ready(None));
        }

        // prove every declared where predicate for this receiver
        let predicates = self.template_predicates(template);
        for predicate in predicates {
            let left = self.substitute_type(origin.module(), predicate.left, &substitution)?;
            let right = self.substitute_type(origin.module(), predicate.right, &substitution)?;

            if !answer!(self.decide_relation(origin, Relation::Satisfies, left, right)?) {
                return Ok(Answer::Ready(None));
            }
        }

        Ok(Answer::Ready(Some(substitution)))
    }

    /// Return substituted member candidates for one matched extension.
    pub(in crate::check) fn extension_member_candidates(
        &mut self,
        origin: Origin,
        extension_symbol: dir::GlobalSymbolId,
        substitution: &TypeSubstitution,
        members: &[DeclaredMember],
    ) -> CompilerResult<Answer<Vec<MemberCandidate>>> {
        let mut candidates = Vec::new();

        // substitute extension parameters in each matching member
        for member in members {
            let Some(ty) = member.ty else {
                continue;
            };

            let ty = self.substitute_type(origin.module(), ty, substitution)?;
            let ty = self.resolve_type_variables(origin.module(), ty)?;
            let ty = member.value_type(self, ty)?;
            let ty =
                match self.projected_member_type(origin, substitution.receiver, member.role, ty)? {
                    Answer::Ready(ty) => ty,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };

            // substitute static projections through the same extension instance
            let written = match member.symbol.and_then(|symbol| self.static_value(symbol)) {
                Some(written) => {
                    let written = self.substitute_type(origin.module(), written, substitution)?;

                    Some(self.resolve_type_variables(origin.module(), written)?)
                }
                written => written,
            };

            let generic_arguments =
                self.generic_argument_bindings(&substitution.parameters, &substitution.arguments)?;

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                role: member.role,
                ty,
                generic_arguments,
                value: member.value,
                value_type: written,
            });
        }

        Ok(Answer::Ready(candidates))
    }

    /// Match one extension member declaration against a receiver.
    pub(in crate::check) fn match_extension(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
        members: &[DeclaredMember],
    ) -> CompilerResult<Answer<Option<Vec<MemberCandidate>>>> {
        let Some(substitution) =
            answer!(self.match_extension_target(origin, receiver, template, target_type,)?)
        else {
            return Ok(Answer::Ready(None));
        };

        let candidates = answer!(self.extension_member_candidates(
            origin,
            extension_symbol,
            &substitution,
            members,
        )?);
        if candidates.is_empty() {
            return Ok(Answer::Ready(None));
        }

        Ok(Answer::Ready(Some(candidates)))
    }
}
