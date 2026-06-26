use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, DeclaredMember, Dependency, GenericArgumentMode, GenericTemplateId,
    MemberCandidate, MemberLookup, Origin, Relation, TypeRewrite, TypeSubstitution, answer,
};

impl CheckState<'_> {
    /// Return whether a selected extension satisfies its where-clauses.
    pub(in crate::check) fn extension_clauses_hold(
        &mut self,
        origin: Origin,
        owner: Option<dir::GlobalSymbolId>,
        arguments: &[dir::GenericArgumentBinding],
    ) -> CompilerResult<Answer<bool>> {
        let Some(owner) = owner else {
            return Ok(Answer::Ready(true));
        };
        let Some(dir::Definition::Extension(extension)) = self.definition(owner) else {
            return Ok(Answer::Ready(true));
        };
        let where_clauses = extension.where_clauses.clone();
        if where_clauses.is_empty() {
            return Ok(Answer::Ready(true));
        }

        // substitute selected arguments into the extension clauses
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let substitution = match self.symbol_template(owner) {
            Some(template) => {
                let parameters = self.generic_template_parameters(template);
                let mut selected = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                for parameter in parameters.iter().copied() {
                    let Some(argument) = arguments
                        .iter()
                        .find(|argument| argument.parameter == parameter)
                        .map(|argument| argument.argument)
                    else {
                        return Ok(Answer::Ready(false));
                    };

                    selected.push(argument);
                }
                if selected.len() != parameters.len() {
                    return Ok(Answer::Ready(false));
                }

                TypeSubstitution {
                    parameters,
                    arguments: selected,
                    receiver: None,
                }
            }
            None => TypeSubstitution::default(),
        };

        // require every extension clause to hold
        for clause in where_clauses {
            let left = if substitution.is_empty() {
                clause.left
            } else {
                self.fold_type(module, source, clause.left, substitution.rewrite())?
            };
            let right = if substitution.is_empty() {
                clause.right
            } else {
                self.fold_type(module, source, clause.right, substitution.rewrite())?
            };
            if !answer!(self.decide_relation(origin, Relation::Satisfies, left, right)?) {
                return Ok(Answer::Ready(false));
            }
        }

        Ok(Answer::Ready(true))
    }

    /// Look up one extension member on a declaration reference.
    pub(in crate::check) fn lookup_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        instance: &dir::GenericInstance,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let extensions = self.visible_extensions(module, instance.symbol);

        // visit extension declarations in resolution order
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();
        for extension_symbol in extensions {
            let lookup = self.lookup_extension_symbol_member(
                origin,
                module,
                receiver,
                extension_symbol,
                space,
                key,
            )?;

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
                MemberLookup::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                MemberLookup::Missing | MemberLookup::Field(_) => {}
            }
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Look up extension members on visible implementations for one receiver.
    pub(in crate::check) fn lookup_receiver_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        space: dir::MemberSpace,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let extensions = self.visible_receiver_extensions(module, receiver)?;
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();

        // collect extension members without ordinary member shadowing
        for extension_symbol in extensions {
            let lookup = self.lookup_extension_symbol_member(
                origin,
                module,
                receiver,
                extension_symbol,
                space,
                key,
            )?;

            match lookup {
                MemberLookup::Found(found) => {
                    for candidate in found {
                        if candidate.symbol.is_some_and(|symbol| !seen.insert(symbol)) {
                            continue;
                        }
                        candidates.push(candidate);
                    }
                }
                MemberLookup::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                MemberLookup::Missing | MemberLookup::Field(_) => {}
            }
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Look up one static extension member on a declaration reference.
    pub(in crate::check) fn lookup_static_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let extensions = self.visible_extensions(module, symbol);
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();

        // visit extension declarations in resolution order
        for extension_symbol in extensions {
            let lookup =
                self.lookup_one_static_extension(origin, module, symbol, extension_symbol, key)?;

            match lookup {
                MemberLookup::Found(found) => {
                    for candidate in found {
                        if candidate.symbol.is_some_and(|symbol| !seen.insert(symbol)) {
                            continue;
                        }
                        candidates.push(candidate);
                    }
                }
                MemberLookup::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                MemberLookup::Missing | MemberLookup::Field(_) => {}
            }
        }

        Ok(MemberLookup::from_candidates(candidates))
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
        receiver: dir::GlobalTypeId,
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let extensions = self.visible_receiver_extensions(module, receiver)?;

        // try each visible implementation declaration
        for extension_symbol in extensions {
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

            // copy extension fields before matching mutates solver state
            let target_type = extension.target.r#type();
            let where_clauses = extension.where_clauses.clone();
            let template = self.symbol_template(extension_symbol);

            // require extension availability before matching
            match self.decide_availability(extension_symbol)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(pending) => {
                    blockers.extend(pending);
                    continue;
                }
            }

            // match the extension target under a probe
            let probe = self.begin_probe();
            let result = self.match_extension_target(
                origin,
                module,
                receiver,
                template,
                target_type,
                &where_clauses,
            );

            let matched = match result {
                Ok(Answer::Ready(Some(substitution))) => self.extension_implements_interface(
                    origin,
                    module,
                    &substitution,
                    &implements,
                    interface,
                ),
                Ok(Answer::Ready(None)) => Ok(Answer::Ready(false)),
                Ok(Answer::Pending(pending)) => Ok(Answer::Pending(pending)),
                Err(error) => Err(error),
            };

            // reject solver state created by this candidate
            self.reject_probe(probe);

            let matched = matched?;
            match matched {
                Answer::Ready(true) => return Ok(Answer::Ready(true)),
                Answer::Ready(false) => {}
                Answer::Pending(pending) => blockers.extend(self.live_blockers(pending)),
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
        let scope = self.extension_root(receiver)?;

        match scope {
            Some(scope) => {
                if !self.is_component_module(scope.module_id) {
                    self.import_external_module(scope.module_id)?;
                }

                Ok(self.visible_extensions(module, scope))
            }
            None => Ok(self.visible_blanket_extensions(module)),
        }
    }

    /// Return the extension lookup root for one receiver type.
    pub(in crate::check) fn extension_root(
        &mut self,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let root = match self.ty(receiver)? {
            dir::Type::Form(form) => return self.extension_root(form.value),
            dir::Type::EnumMember(member) => return self.extension_root(member.owner),
            dir::Type::Instance(instance) => Some(self.resolve_symbol_alias(instance.symbol)?),
            dir::Type::Literal(literal) => {
                literal.owner_item().map(|item| self.language_symbol(item))
            }
            dir::Type::Primitive(primitive) => primitive
                .owner_item()
                .map(|item| self.language_symbol(item)),
            dir::Type::Array(_) => Some(self.language_symbol(dir::LanguageItem::Array)),
            dir::Type::Slice(_) => Some(self.language_symbol(dir::LanguageItem::Slice)),
            dir::Type::FixedArray(_) => Some(self.language_symbol(dir::LanguageItem::FixedArray)),
            _ => None,
        };

        Ok(root)
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

    /// Decide whether one selected extension implements one interface.
    pub(in crate::check) fn selected_extension_implements_interface(
        &mut self,
        origin: Origin,
        module: ModuleId,
        extension_symbol: dir::GlobalSymbolId,
        extension_arguments: &[dir::GlobalTypeId],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(Answer::Ready(false));
        };
        let implements = extension.implements.clone();
        if implements.is_empty() {
            return Ok(Answer::Ready(false));
        }
        let extension = dir::GenericInstance {
            symbol: extension_symbol,
            arguments: extension_arguments.to_vec(),
        };
        let substitution = self.instance_substitution(&extension)?;

        self.extension_implements_interface(origin, module, &substitution, &implements, interface)
    }

    /// Decide whether one selected member owner satisfies one interface.
    pub(in crate::check) fn selected_member_owner_implements_interface(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        owner: dir::GlobalSymbolId,
        owner_arguments: &[dir::GenericArgumentBinding],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        if matches!(self.definition(owner), Some(dir::Definition::Extension(_))) {
            let owner_arguments = owner_arguments
                .iter()
                .map(|argument| argument.argument)
                .collect::<Vec<_>>();

            return self.selected_extension_implements_interface(
                origin,
                module,
                owner,
                &owner_arguments,
                interface,
            );
        }

        let source = self.origin_source_node(origin)?;
        let interface = self.push_type(module, dir::Type::Instance(interface.clone()), source)?;

        self.decide_relation(origin, Relation::Implements, receiver, interface)
    }

    /// Decide whether one selected member owner names or inherits one protocol.
    pub(in crate::check) fn selected_member_owner_has_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        owner: dir::GlobalSymbolId,
        owner_arguments: &[dir::GenericArgumentBinding],
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        if matches!(self.definition(owner), Some(dir::Definition::Extension(_))) {
            let owner_arguments = owner_arguments
                .iter()
                .map(|argument| argument.argument)
                .collect::<Vec<_>>();

            return self.selected_extension_has_protocol(
                origin,
                module,
                owner,
                &owner_arguments,
                protocol,
            );
        }

        let receiver = answer!(self.reduce_type_root(origin, receiver)?);
        let instance = match self.ty(receiver)? {
            dir::Type::Form(form) => match self.ty(form.value)? {
                dir::Type::Instance(instance) => Some(instance.clone()),
                _ => None,
            },
            dir::Type::Instance(instance) => Some(instance.clone()),
            _ => None,
        };
        let Some(instance) = instance else {
            return Ok(Answer::Ready(false));
        };
        if instance.symbol == protocol {
            return Ok(Answer::Ready(true));
        }

        let inherited = answer!(self.heritage_instance(origin, &instance, protocol)?);

        Ok(Answer::Ready(inherited.is_some()))
    }

    /// Decide whether one selected extension names or inherits one protocol.
    fn selected_extension_has_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        extension_symbol: dir::GlobalSymbolId,
        extension_arguments: &[dir::GlobalTypeId],
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(Answer::Ready(false));
        };
        let implements = extension.implements.clone();
        if implements.is_empty() {
            return Ok(Answer::Ready(false));
        }
        let extension = dir::GenericInstance {
            symbol: extension_symbol,
            arguments: extension_arguments.to_vec(),
        };
        let substitution = self.instance_substitution(&extension)?;

        self.extension_has_protocol(origin, module, &substitution, &implements, protocol)
    }

    /// Decide whether implemented heritage names or inherits one protocol.
    fn extension_has_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &TypeSubstitution,
        implements: &[dir::NominalHeritage],
        protocol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<bool>> {
        let source = self.origin_source_node(origin)?;

        // compare each declared interface by protocol symbol
        for heritage in implements {
            let implemented = self.substituted_heritage(module, source, substitution, heritage)?;
            if implemented.symbol == protocol {
                return Ok(Answer::Ready(true));
            }
            if answer!(self.heritage_instance(origin, &implemented, protocol)?).is_some() {
                return Ok(Answer::Ready(true));
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Decide whether implemented heritage covers one requested interface.
    fn extension_implements_interface(
        &mut self,
        origin: Origin,
        module: ModuleId,
        substitution: &TypeSubstitution,
        implements: &[dir::NominalHeritage],
        interface: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let source = self.origin_source_node(origin)?;

        // compare each declared interface
        for heritage in implements {
            let implemented = self.substituted_heritage(module, source, substitution, heritage)?;
            let matches = if implemented.symbol == interface.symbol {
                self.decide_each_argument(origin, &implemented, interface)?
            } else if let Some(inherited) =
                answer!(self.heritage_instance(origin, &implemented, interface.symbol)?)
            {
                self.decide_each_argument(origin, &inherited, interface)?
            } else {
                Answer::Ready(false)
            };

            if !matches!(matches, Answer::Ready(false)) {
                return Ok(matches);
            }
        }

        Ok(Answer::Ready(false))
    }

    /// Return implemented heritage after extension generic substitution.
    fn substituted_heritage(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        substitution: &TypeSubstitution,
        heritage: &dir::NominalHeritage,
    ) -> CompilerResult<dir::GenericInstance> {
        let arguments = if substitution.is_empty() {
            heritage.arguments.clone()
        } else {
            heritage
                .arguments
                .iter()
                .map(|argument| self.fold_type(module, source, *argument, substitution.rewrite()))
                .collect::<CompilerResult<Vec<_>>>()?
        };

        Ok(dir::GenericInstance {
            symbol: heritage.symbol,
            arguments,
        })
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
    ) -> CompilerResult<MemberLookup> {
        // require extension availability before member lookup
        match self.decide_availability(extension_symbol)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) => return Ok(MemberLookup::Missing),
            Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
        }

        // read the extension members
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(MemberLookup::Missing);
        };
        if !extension.is_visible_from(module) {
            return Ok(MemberLookup::Missing);
        }
        let target_type = extension.target.r#type();
        let matched = extension
            .members
            .iter()
            .filter_map(DeclaredMember::from_definition)
            .filter(|member| member.matches(space, key))
            .collect::<SmallVec<[_; 2]>>();
        if matched.is_empty() {
            return Ok(MemberLookup::Missing);
        }
        let members = matched;
        let where_clauses = match self.definition(extension_symbol) {
            Some(dir::Definition::Extension(extension)) => extension.where_clauses.clone(),
            _ => Vec::new(),
        };

        // open extension generics and match the receiver
        let template = self.symbol_template(extension_symbol);
        let probe = self.begin_probe();
        let result = self.match_extension(
            origin,
            module,
            receiver,
            extension_symbol,
            template,
            target_type,
            &where_clauses,
            &members,
        );
        match result {
            Ok(Answer::Ready(lookup @ (MemberLookup::Field(_) | MemberLookup::Found(_)))) => {
                self.commit_probe(probe);

                Ok(lookup)
            }
            Ok(Answer::Ready(MemberLookup::Missing)) => {
                self.reject_probe(probe);

                Ok(MemberLookup::Missing)
            }
            // blockers that died with the probe cannot wake this extension
            Ok(Answer::Pending(blockers) | Answer::Ready(MemberLookup::Pending(blockers))) => {
                self.reject_probe(probe);

                let blockers = self.live_blockers(blockers);
                if blockers.is_empty() {
                    Ok(MemberLookup::Missing)
                } else {
                    Ok(MemberLookup::Pending(blockers))
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
    ) -> CompilerResult<MemberLookup> {
        // require extension availability before member lookup
        match self.decide_availability(extension_symbol)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) => return Ok(MemberLookup::Missing),
            Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
        }

        // read matching static members from extensions of this declaration
        let Some(dir::Definition::Extension(extension)) = self.definition(extension_symbol) else {
            return Ok(MemberLookup::Missing);
        };
        if !extension.is_visible_from(module) {
            return Ok(MemberLookup::Missing);
        }
        if extension.target.root() != Some(symbol) {
            return Ok(MemberLookup::Missing);
        }
        let members = extension
            .members
            .iter()
            .filter_map(DeclaredMember::from_definition)
            .filter(|member| member.matches(dir::MemberSpace::Static, key))
            .collect::<SmallVec<[_; 2]>>();
        if members.is_empty() {
            return Ok(MemberLookup::Missing);
        }
        let where_clauses = extension.where_clauses.clone();

        self.lookup_open_static_extension(
            origin,
            module,
            extension_symbol,
            &where_clauses,
            &members,
        )
    }

    /// Look up static extension members without solving extension generics.
    fn lookup_open_static_extension(
        &mut self,
        origin: Origin,
        module: ModuleId,
        extension_symbol: dir::GlobalSymbolId,
        where_clauses: &[dir::ExtensionWhereClause],
        members: &[DeclaredMember],
    ) -> CompilerResult<MemberLookup> {
        let source = self.origin_source_node(origin)?;
        let substitution = TypeSubstitution::default();
        let arguments = self.open_extension_arguments(origin, extension_symbol)?;

        // require clauses that do not depend on call inference now
        if !where_clauses.is_empty() && self.symbol_template(extension_symbol).is_none() {
            for clause in where_clauses {
                match self.decide_relation(
                    origin,
                    Relation::Satisfies,
                    clause.left,
                    clause.right,
                )? {
                    Answer::Ready(true) => {}
                    Answer::Ready(false) => return Ok(MemberLookup::Missing),
                    Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                }
            }
        }

        // expose matching static members as open callable candidates
        let mut candidates = Vec::new();
        for member in members {
            let Some(ty) = member.ty else {
                continue;
            };
            match self.decide_member_availability(
                origin,
                module,
                member.condition,
                &substitution,
            )? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }

            let ty = member.read_type(self, ty)?;
            let written = member.symbol.and_then(|symbol| self.static_value(symbol));

            let generic_arguments =
                self.symbol_generic_argument_bindings(extension_symbol, &arguments)?;

            candidates.push(MemberCandidate {
                symbol: member.symbol,
                owner: extension_symbol,
                role: member.role,
                ty: self.fold_type(module, source, ty, TypeRewrite::Resolve)?,
                generic_arguments,
                value: member.value,
                value_type: written,
            });
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Return generic parameter types for one open extension head.
    fn open_extension_arguments(
        &mut self,
        origin: Origin,
        extension_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let Some(template) = self.symbol_template(extension_symbol) else {
            return Ok(Vec::new());
        };
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        let mut arguments = Vec::new();
        for parameter in self.generic_template_parameters(template) {
            arguments.push(self.push_type(module, dir::Type::Parameter(parameter), source)?);
        }

        Ok(arguments)
    }

    /// Match one extension target against a receiver under an active probe.
    fn match_extension_target(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
        where_clauses: &[dir::ExtensionWhereClause],
    ) -> CompilerResult<Answer<Option<TypeSubstitution>>> {
        let source = self.origin_source_node(origin)?;

        // instantiate the extension's generic parameters
        let substitution = match template {
            Some(template) => {
                match self.instantiate_template(
                    origin,
                    template,
                    &[],
                    GenericArgumentMode::Match,
                )? {
                    Some(substitution) => substitution,
                    None => return Ok(Answer::Ready(None)),
                }
            }
            None => Default::default(),
        };
        let substitution = substitution.with_receiver(receiver);
        let target_type = if substitution.is_empty() {
            target_type
        } else {
            self.fold_type(module, source, target_type, substitution.rewrite())?
        };
        // match the receiver against the extension target
        if !answer!(self.constrain_extension_target(origin, receiver, target_type)?) {
            return Ok(Answer::Ready(None));
        }

        // reject extensions whose inferred arguments violate their constraints
        let variables = self.substitution_variables(&substitution)?;
        if !answer!(self.solve_probe_variables(variables)?) {
            return Ok(Answer::Ready(None));
        }

        // require every where clause to hold
        for clause in where_clauses {
            let left = if substitution.is_empty() {
                clause.left
            } else {
                self.fold_type(module, source, clause.left, substitution.rewrite())?
            };
            let right = if substitution.is_empty() {
                clause.right
            } else {
                self.fold_type(module, source, clause.right, substitution.rewrite())?
            };

            if !answer!(self.decide_relation(origin, Relation::Satisfies, left, right)?) {
                return Ok(Answer::Ready(None));
            }
        }

        Ok(Answer::Ready(Some(substitution)))
    }

    /// Constrain one receiver against one extension target.
    fn constrain_extension_target(
        &mut self,
        origin: Origin,
        receiver: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let receiver = answer!(self.reduce_type_root(origin, receiver)?);
        let target = answer!(self.reduce_type_root(origin, target)?);

        // same-root targets bind extension arguments as a pattern
        let matching_reference = match (self.ty(receiver)?, self.ty(target)?) {
            (dir::Type::Instance(receiver), dir::Type::Instance(target))
                if receiver.symbol == target.symbol
                    && receiver.arguments.len() == target.arguments.len() =>
            {
                let receiver_arguments = receiver
                    .arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();
                let target_arguments = target
                    .arguments
                    .iter()
                    .copied()
                    .collect::<SmallVec<[_; 4]>>();

                Some((receiver_arguments, target_arguments))
            }
            _ => None,
        };
        if let Some((receiver_arguments, target_arguments)) = matching_reference {
            let mut matched = Answer::Ready(true);
            for (receiver, target) in receiver_arguments.iter().zip(&target_arguments) {
                let constraint = self.constrain(origin, Relation::Equal, *receiver, *target)?;
                matched = matched.and(constraint);
                if matched.is_ready_false() {
                    return Ok(matched);
                }
            }

            return Ok(matched);
        }

        // broader targets use regular assignability
        self.constrain(origin, Relation::Assignable, receiver, target)
    }

    /// Match one extension member declaration against a receiver.
    fn match_extension(
        &mut self,
        origin: Origin,
        module: ModuleId,
        receiver: dir::GlobalTypeId,
        extension_symbol: dir::GlobalSymbolId,
        template: Option<GenericTemplateId>,
        target_type: dir::GlobalTypeId,
        where_clauses: &[dir::ExtensionWhereClause],
        members: &[DeclaredMember],
    ) -> CompilerResult<Answer<MemberLookup>> {
        let source = self.origin_source_node(origin)?;
        let Some(substitution) = answer!(self.match_extension_target(
            origin,
            module,
            receiver,
            template,
            target_type,
            where_clauses,
        )?) else {
            return Ok(Answer::Ready(MemberLookup::Missing));
        };

        // resolve matched members through solved inference variables
        let mut candidates = Vec::new();
        for member in members {
            let Some(ty) = member.ty else {
                continue;
            };

            // gate members on their substituted @if availability
            match self.decide_member_availability(
                origin,
                module,
                member.condition,
                &substitution,
            )? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
            let ty = if substitution.is_empty() {
                ty
            } else {
                self.fold_type(module, source, ty, substitution.rewrite())?
            };
            let ty = self.fold_type(module, source, ty, TypeRewrite::Resolve)?;
            let ty = member.read_type(self, ty)?;

            // carry substituted static value types for projections
            let written = match member.symbol.and_then(|symbol| self.static_value(symbol)) {
                Some(written) if !substitution.is_empty() => {
                    let folded = self.fold_type(module, source, written, substitution.rewrite())?;

                    Some(self.fold_type(module, source, folded, TypeRewrite::Resolve)?)
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

        Ok(Answer::Ready(MemberLookup::from_candidates(candidates)))
    }
}
