use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, BodyState, CandidateOutcome, Cause, CauseKind, DeclaredMember, Dependency,
    GenericTemplateId, MemberCandidate, MemberLookup, Origin, ProbeReason, ReceiverSteps, Relation,
    TypeSubstitution, answer,
};

impl BodyState<'_, '_> {
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
        let extensions = self.visible_extensions(module, instance.symbol)?;

        // visit extension declarations in resolution order
        let mut candidates = Vec::new();
        let mut seen = FxIndexSet::default();
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
        let extensions = self.visible_extensions(module, symbol)?;
        let mut candidates = Vec::new();
        let mut seen = FxIndexSet::default();

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
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
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

    /// Decide whether a visible extension implements one interface for a receiver.
    pub(in crate::check) fn decide_extension_implementation(
        &mut self,
        origin: Origin,
        module: ModuleId,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericInstance,
        excluded: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let extensions = self.visible_receiver_extensions(module, receiver)?;

        // try each visible implementation declaration
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

            // skip extensions without interface declarations
            let implements = extension.implements.clone();
            if implements.is_empty() {
                continue;
            }
            // take declaration inputs before entering the candidate probe
            let target_type = extension.target.r#type();
            let template = self.symbol_template(extension_symbol)?;

            // match the extension target speculatively
            let matched = self.confirm_candidate(ProbeReason::Implements, |state| {
                let matched =
                    state.match_extension_target(origin, receiver, template, target_type)?;
                match matched {
                    Answer::Ready(Some(substitution)) => {
                        let holds = answer!(state.extension_implements_interface(
                            origin,
                            module,
                            interface_module,
                            &substitution,
                            &implements,
                            interface,
                        )?);
                        if holds {
                            Ok(Answer::Ready(CandidateOutcome::Accepted(())))
                        } else {
                            Ok(Answer::Ready(CandidateOutcome::Rejected(())))
                        }
                    }
                    Answer::Ready(None) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                    Answer::Pending(pending) => Ok(Answer::Pending(pending)),
                }
            })?;
            match matched {
                Answer::Ready(Some(())) => {
                    return Ok(Answer::Ready(true));
                }
                Answer::Ready(None) => {}
                Answer::Pending(pending) => {
                    blockers.extend(pending);
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
            return self.visible_blanket_extensions(module);
        };
        let scope = instance.symbol;

        if !self.is_component_module(scope.module_id) {
            self.import_external_module(scope.module_id)?;
        }

        self.visible_extensions(module, scope)
    }

    /// Collect blanket extension symbols visible from one module.
    fn visible_blanket_extensions(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<SmallVec<[dir::GlobalSymbolId; 4]>> {
        let mut symbols = SmallVec::new();

        // collect local blanket extensions
        if let Some(state) = self.modules.get(&module) {
            symbols.extend(state.definitions.blanket_extensions().iter().copied());
        }

        // collect imported extension symbols
        let imported = self
            .module(module)
            .resolved
            .imports
            .symbol_targets()
            .map(|(_, symbol)| symbol)
            .collect::<SmallVec<[_; 8]>>();
        for symbol in imported {
            let is_extension = self
                .definition(symbol)?
                .is_some_and(|definition| matches!(definition, dir::Definition::Extension(_)));
            if is_extension && !symbols.contains(&symbol) {
                symbols.push(symbol);
            }
        }

        Ok(symbols)
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
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)? else {
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
        let template = self.symbol_template(extension_symbol)?;
        let result = self.confirm_candidate(ProbeReason::Extension, |state| {
            match state.match_extension(
                origin,
                receiver,
                extension_symbol,
                template,
                target_type,
                &members,
            )? {
                Answer::Ready(Some(candidates)) => {
                    Ok(Answer::Ready(CandidateOutcome::Accepted(candidates)))
                }
                Answer::Ready(None) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
            }
        })?;

        match result {
            Answer::Ready(Some(candidates)) => {
                Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
            }
            Answer::Ready(None) => Ok(Answer::Ready(MemberLookup::Missing)),
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
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
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol)? else {
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
        // expose matching static members for later call inference
        let mut candidates = Vec::new();
        for member in members {
            let Some(ty) = member.ty else {
                continue;
            };
            let ty = member.value_type(self, ty)?;
            let written = member.symbol.and_then(|symbol| self.static_value(symbol));
            let ty = answer!(self.projected_member_type(origin, None, member.role, ty)?);

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                space: dir::MemberSpace::Static,
                role: member.role,
                ty,
                generic_arguments: Vec::new(),
                value: member.value,
                value_type: written,
                steps: ReceiverSteps::new(),
                receiver: None,
            });
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
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
            None => TypeSubstitution::default(),
        };
        let substitution = substitution.with_receiver(receiver);

        // prove the receiver satisfies the completed target; open receivers
        //  constrain under the active probe so bounds flow into their holes
        let target_type = self.substitute_type(origin.module(), target_type, &substitution)?;
        let proven = match self.type_variables(receiver)?.is_empty() {
            true => self.decide_relation(origin, Relation::Assignable, receiver, target_type)?,
            false => {
                let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));

                self.constrain_type(cause, Relation::Assignable, receiver, target_type)?
            }
        };
        if !answer!(proven) {
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

                    Some(written)
                }
                written => written,
            };

            let generic_arguments =
                self.generic_argument_bindings(&substitution.parameters, &substitution.arguments)?;

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                space: member.space,
                role: member.role,
                ty,
                generic_arguments,
                value: member.value,
                value_type: written,
                steps: ReceiverSteps::new(),
                receiver: None,
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
