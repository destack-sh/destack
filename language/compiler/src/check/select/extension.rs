use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, BodyState, CandidateOutcome, Cause, CauseKind, DeclaredMember, Dependency,
    GenericParameterId, GenericTemplateId, LookupReceiver, MemberCandidate, MemberLookup, Origin,
    ReceiverSteps, Relation, TypeArgumentInference, TypeSubstitution, answer,
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
    ) -> CompilerResult<Answer<MemberLookup>> {
        let extensions = self.visible_extensions(module, symbol)?;

        // visit extension declarations in resolution order
        let mut candidates = Vec::new();
        let mut seen = FxIndexSet::default();
        for extension_symbol in extensions {
            let lookup = answer!(self.lookup_extension_symbol_member(
                origin,
                module,
                receiver,
                subject,
                extension_symbol,
                space,
                key,
            )?);

            let found = match lookup {
                MemberLookup::Found(found) => found,
                MemberLookup::Missing | MemberLookup::Field(_) => continue,
                MemberLookup::Union(_) | MemberLookup::Intersection(_) => {
                    return Err(CompilerError::Internal {
                        message: "one extension declaration produced a union member lookup"
                            .to_string(),
                    });
                }
            };

            // collect first declarations of each member symbol
            for candidate in found {
                if !seen.insert(candidate.symbol) {
                    continue;
                }
                candidates.push(candidate);
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
            let definitions = &state.definitions;
            symbols.extend(definitions.target_extensions(target).iter().copied());
            symbols.extend(definitions.blanket_extensions().iter().copied());
        }

        // collect inherent extensions beside the target declaration
        if target.module_id != module {
            if let Some(state) = self.modules.get(&target.module_id) {
                let definitions = &state.definitions;
                symbols.extend(definitions.target_extensions(target).iter().copied());
                symbols.extend(definitions.blanket_extensions().iter().copied());
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
        relation: Relation,
        module: ModuleId,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericApplication,
        excluded: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Answer<bool>> {
        // break inductive applicability cycles: goals reached from themselves fail
        let arguments = self
            .type_ids(interface_module, interface.arguments)?
            .to_vec();
        let arguments = self.intern_type_ids(origin.module(), &arguments)?;
        let interface_type = self.intern_type(
            origin.module(),
            dir::Type::Application(dir::GenericApplication {
                symbol: interface.symbol,
                arguments,
            }),
        )?;
        let goal = (relation, receiver, interface_type);
        if !self.check.deciding_extensions.insert(goal) {
            return Ok(Answer::Ready(false));
        }

        let decision = self.decide_visible_extensions(
            origin,
            relation,
            module,
            interface_module,
            receiver,
            interface,
            excluded,
        );
        self.check.deciding_extensions.swap_remove(&goal);

        decision
    }

    /// Judge every visible extension against one applicability goal.
    fn decide_visible_extensions(
        &mut self,
        origin: Origin,
        relation: Relation,
        module: ModuleId,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericApplication,
        excluded: Option<dir::GlobalSymbolId>,
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let extensions = answer!(self.visible_implementation_extensions(origin, module, receiver)?);

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
                return Err(CompilerError::Internal {
                    message: format!(
                        "indexed implementation symbol {extension_symbol:?} has no extension definition"
                    ),
                });
            };
            if !extension.is_visible_from(module) {
                continue;
            }

            // skip extensions without interface declarations
            let implements = extension.implements.clone();
            if implements.is_empty() {
                continue;
            }

            // match the extension header and rows in one candidate probe
            let target_type = extension.target.r#type();
            let template = self.symbol_template(extension_symbol)?;
            let matched = self.confirm_candidate(|state| {
                let matched = state.match_extension_applicability(
                    origin,
                    relation,
                    module,
                    interface_module,
                    receiver,
                    interface,
                    template,
                    target_type,
                    &implements,
                )?;
                Ok(match matched {
                    Answer::Ready(true) => Answer::Ready(CandidateOutcome::Accepted(())),
                    Answer::Ready(false) => Answer::Ready(CandidateOutcome::Rejected(())),
                    Answer::Pending(pending) => Answer::Pending(pending),
                })
            })?;
            match matched {
                Answer::Ready(Some(())) => return Ok(Answer::Ready(true)),
                Answer::Ready(None) => {}
                Answer::Pending(pending) => blockers.extend(pending),
            }
        }

        Ok(Answer::ready_unless_blocked(false, blockers))
    }

    /// Match one extension's target and implements rows against a goal.
    fn match_extension_applicability(
        &mut self,
        origin: Origin,
        relation: Relation,
        module: ModuleId,
        interface_module: ModuleId,
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericApplication,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
        implements: &[dir::InterfaceImplementation],
    ) -> CompilerResult<Answer<bool>> {
        // matching binds the extension parameters through the target header
        let parameters = match template {
            Some(template) => self.generic_template_parameters(template)?,
            None => SmallVec::new(),
        };
        let mut substitution = TypeSubstitution::default().with_receiver(receiver);
        if !answer!(self.bind_extension_target(
            origin,
            &parameters,
            &mut substitution,
            target_type,
            receiver,
        )?) {
            return Ok(Answer::Ready(false));
        }

        // require the receiver to satisfy the bound target
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        let target = self.substitute_type(origin.module(), target_type, &substitution)?;
        if !answer!(self.constrain_type(origin, cause, Relation::Assignable, receiver, target)?) {
            return Ok(Answer::Ready(false));
        }

        // one declared row must implement the requested interface
        let implementation = answer!(self.match_implemented_interface(
            origin,
            relation,
            module,
            interface_module,
            &parameters,
            &mut substitution,
            implements,
            interface,
        )?);
        if implementation.is_none() {
            return Ok(Answer::Ready(false));
        }

        // register the declared bounds and predicates with the candidate
        if let Some(template) = template {
            let constraints =
                self.substitute_application_constraints(origin, template, &substitution)?;
            for constraint in constraints {
                self.check.push_constraint(constraint);
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Bind extension parameters by matching the target against the receiver.
    fn bind_extension_target(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        target: dir::GlobalTypeId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // a direct structural match binds most extension targets
        let mut scratch = substitution.clone();
        if answer!(self.extend_generic_substitution(
            origin,
            parameters,
            &mut scratch,
            &[(target, receiver)],
        )?) {
            *substitution = scratch;

            return Ok(Answer::Ready(true));
        }

        // nominal receivers bind through their substituted heritage
        let receiver_value = answer!(self.strip_form(origin, receiver)?);
        let Some((receiver_module, instance)) = self.nominal_application_maybe(receiver_value)?
        else {
            return Ok(Answer::Ready(false));
        };
        let closure = answer!(self.heritage_closure(origin, receiver_module, &instance)?);
        for ancestor in &closure.applications {
            let mut scratch = substitution.clone();
            if answer!(self.extend_generic_substitution(
                origin,
                parameters,
                &mut scratch,
                &[(target, ancestor.ty)],
            )?) {
                *substitution = scratch;

                return Ok(Answer::Ready(true));
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Instantiate one extension target and register its declared constraints.
    pub(in crate::check) fn instantiate_extension(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<TypeSubstitution>>> {
        // open every extension parameter before collecting applicability constraints
        let substitution = match template {
            Some(template) => {
                let parameters = self.generic_template_parameters(template)?;
                let Some(substitution) = answer!(self.instantiate_parameters(
                    origin,
                    &parameters,
                    &[],
                    TypeSubstitution::default().with_receiver(receiver),
                    TypeArgumentInference::Exact,
                )?) else {
                    return Ok(Answer::Ready(None));
                };

                substitution
            }
            None => TypeSubstitution::default().with_receiver(receiver),
        };
        // constrain the receiver against the applied extension target
        let target_type = self.substitute_type(origin.module(), target_type, &substitution)?;
        let cause = self.intern_cause(Cause::root(origin, CauseKind::Expression));
        if !answer!(self.constrain_type(
            origin,
            cause,
            Relation::Assignable,
            receiver,
            target_type,
        )?) {
            return Ok(Answer::Ready(None));
        }

        // register parameter bounds and where predicates with the candidate
        if let Some(template) = template {
            let constraints =
                self.substitute_application_constraints(origin, template, &substitution)?;
            for constraint in constraints {
                self.check.push_constraint(constraint);
            }
        }

        Ok(Answer::Ready(Some(substitution)))
    }

    /// Collect visible extensions that may implement an interface for one receiver.
    pub(in crate::check) fn visible_implementation_extensions(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[dir::GlobalSymbolId; 4]>>> {
        let receiver = answer!(self.strip_form(origin, receiver)?);
        let scope = match self.nominal_application_maybe(receiver)? {
            Some((_, instance)) => Some(instance.symbol),
            None => self
                .ty(receiver)?
                .representation_item()
                .map(|item| self.language_symbol(item))
                .transpose()?,
        };
        let Some(scope) = scope else {
            let extensions = self.visible_blanket_extensions(module)?;

            return Ok(Answer::Ready(extensions));
        };

        if !self.is_component_module(scope.module_id) {
            self.import_external_module(scope.module_id)?;
        }

        let extensions = self.visible_extensions(module, scope)?;

        Ok(Answer::Ready(extensions))
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
        subject: dir::GlobalTypeId,
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
            return Err(CompilerError::Internal {
                message: format!(
                    "indexed extension symbol {extension_symbol:?} has no extension definition"
                ),
            });
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
        let result = self.confirm_candidate(|state| {
            match state.match_extension(
                origin,
                receiver,
                subject,
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
            return Err(CompilerError::Internal {
                message: format!(
                    "indexed extension symbol {extension_symbol:?} has no extension definition"
                ),
            });
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
            let callable = member.callable_type(origin.module(), extension_symbol, ty, self)?;
            let access_type = member.access_type(self, ty)?;
            let written = self.static_value(member.symbol);
            let access_type =
                answer!(self.projected_member_type(origin, None, member.role, access_type)?);

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                space: dir::MemberSpace::Static,
                role: member.role,
                access_type,
                callable,
                is_optional: member.is_optional,
                generic_arguments: Vec::new(),
                value: member.value,
                value_type: written,
                receiver: LookupReceiver::Direct(ReceiverSteps::new()),
            });
        }

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
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
            let callable = member.callable_type(origin.module(), extension_symbol, ty, self)?;
            let access_type = member.access_type(self, ty)?;
            let access_type = match self.projected_member_type(
                origin,
                substitution.receiver,
                member.role,
                access_type,
            )? {
                Answer::Ready(ty) => ty,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };

            // substitute static projections through the same extension instance
            let written = match self.static_value(member.symbol) {
                Some(written) => {
                    let written = self.substitute_type(origin.module(), written, substitution)?;

                    Some(written)
                }
                written => written,
            };

            let generic_arguments = self.settled_argument_bindings(&substitution.bindings)?;

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                space: member.space,
                role: member.role,
                access_type,
                callable,
                is_optional: member.is_optional,
                generic_arguments,
                value: member.value,
                value_type: written,
                receiver: LookupReceiver::Direct(ReceiverSteps::new()),
            });
        }

        Ok(Answer::Ready(candidates))
    }

    /// Match one extension member declaration against a receiver.
    pub(in crate::check) fn match_extension(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        subject: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
        members: &[DeclaredMember],
    ) -> CompilerResult<Answer<Option<Vec<MemberCandidate>>>> {
        // specialize parameters determined by the receiver target
        let substitution = match template {
            Some(template) => {
                let parameters = self.generic_template_parameters(template)?;
                let mut substitution = TypeSubstitution::default();
                let pairs = [(target_type, subject)];
                if !answer!(self.extend_generic_substitution(
                    origin,
                    &parameters,
                    &mut substitution,
                    &pairs,
                )?) {
                    return Ok(Answer::Ready(None));
                }

                substitution
            }
            None => TypeSubstitution::default(),
        };
        let substitution = substitution.with_receiver(receiver);

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
