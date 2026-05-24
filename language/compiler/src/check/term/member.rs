use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    ArgumentTerm, CheckComponentState, FunctionTerm, GenericInstance, GenericSubstitution,
    MemberFailure, MemberOutcome, MemberProtocol, MemberResolution, MemberResolutionTarget,
    ShapeMemberTerm, StaticTerm, TupleElementTerm, TypeRelation, TypeTerm, VariableId,
    VariableOrigin,
};
use crate::{CompilerError, CompilerResult};

use crate::check::Decision;

/// A symbol-backed member candidate found by lookup.
pub(in crate::check) struct MemberCandidate {
    /// The resolved member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The resolved member type.
    pub(in crate::check) ty: VariableId,
    /// The resolved generic instance, when lookup instantiated an owner.
    pub(in crate::check) instance: Option<GenericInstance>,
}

impl CheckComponentState<'_> {
    /// Reduce one member projection to its type.
    pub(in crate::check) fn reduce_member_type(
        &mut self,
        module: ModuleId,
        source: Option<dir::GlobalNodeIdAny>,
        owner: VariableId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let receiver = owner;
        let Some(owner) = self.solved_type_term(owner)? else {
            return Ok(None);
        };
        self.record_member_outcome(source, receiver, &key, &owner)?;

        self.member_type_term(module, &owner, &key)
    }

    /// Record one member outcome for commit and diagnostics.
    fn record_member_outcome(
        &mut self,
        source: Option<dir::GlobalNodeIdAny>,
        receiver: VariableId,
        key: &dir::StaticKey,
        owner: &TypeTerm,
    ) -> CompilerResult<()> {
        let Some(source) = source else {
            return Ok(());
        };
        let outcome =
            if let Some(member) = self.member_type_candidate(receiver.module, owner, key)? {
                let target = MemberResolutionTarget::Symbol {
                    symbol: member.symbol,
                    instance: member.instance,
                };
                let member = MemberResolution {
                    source,
                    receiver,
                    target,
                };

                MemberOutcome::Resolved(member)
            } else if self.type_term_has_field(receiver.module, owner, key)? {
                let member = MemberResolution {
                    source,
                    receiver,
                    target: MemberResolutionTarget::Field(*key),
                };

                MemberOutcome::Resolved(member)
            } else {
                MemberOutcome::Rejected(MemberFailure::Missing)
            };

        self.module_mut(source.module_id)?
            .record_member_outcome(source, outcome);

        Ok(())
    }

    /// Return a member type from one reduced type term.
    pub(in crate::check) fn member_type_term(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.member_type_term(module, &term, key)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.solved_type_term(*payload)? else {
                    return Ok(None);
                };

                self.member_type_term(module, &term, key)
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                if self.generic_type_constraint(*symbol)?.is_some() {
                    self.parameter_member_type(*symbol, key)
                } else {
                    self.symbol_member_type(module, *symbol, *key)
                }
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.applied_symbol_member_type(module, *symbol, arguments, *key),
            TypeTerm::Tuple { elements, .. } => self.tuple_member_type(module, elements, key),
            TypeTerm::Shape { members } => Ok(self.shape_member_type(members, key)),
            TypeTerm::Parameter { symbol } => self.parameter_member_type(*symbol, key),
            _ => Ok(None),
        }
    }

    /// Return a member static from one reduced type term.
    pub(in crate::check) fn member_static_term(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<StaticTerm>> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.member_static_term(module, &term, key)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.solved_type_term(*payload)? else {
                    return Ok(None);
                };

                self.member_static_term(module, &term, key)
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_member_static(module, *symbol, *key),
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.applied_symbol_member_static(module, *symbol, arguments, *key),
            _ => Ok(None),
        }
    }

    /// Return a symbol-backed member candidate from one reduced type term.
    pub(in crate::check) fn member_type_candidate(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<MemberCandidate>> {
        self.member_type_candidate_matching(module, term, key, None)
    }

    /// Return a symbol-backed member candidate that satisfies one protocol.
    pub(in crate::check) fn member_type_candidate_for_protocol(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Option<MemberCandidate>> {
        self.member_type_candidate_matching(module, term, key, Some(protocol))
    }

    /// Return a symbol-backed member candidate from one reduced type term.
    fn member_type_candidate_matching(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.member_type_candidate_matching(variable.module, &term, key, protocol)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.solved_type_term(*payload)? else {
                    return Ok(None);
                };

                self.member_type_candidate_matching(module, &term, key, protocol)
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                if self.generic_type_constraint(*symbol)?.is_some() {
                    self.parameter_member_candidate(*symbol, key, protocol)
                } else {
                    self.applied_symbol_member_candidate(module, *symbol, arguments, *key, protocol)
                }
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.applied_symbol_member_candidate(module, *symbol, arguments, *key, protocol),
            TypeTerm::Parameter { symbol } => {
                self.parameter_member_candidate(*symbol, key, protocol)
            }
            _ => Ok(None),
        }
    }

    /// Return a member type through a generic parameter constraint.
    fn parameter_member_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(constraint) = self.generic_type_constraint(symbol)? else {
            return Ok(None);
        };

        let term = TypeTerm::Variable(constraint);

        self.member_type_term(constraint.module, &term, key)
    }

    /// Return a member candidate through a generic parameter constraint.
    fn parameter_member_candidate(
        &mut self,
        symbol: dir::GlobalSymbolId,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        let Some(constraint) = self.generic_type_constraint(symbol)? else {
            return Ok(None);
        };

        let term = TypeTerm::Variable(constraint);

        self.member_type_candidate_matching(constraint.module, &term, key, protocol)
    }

    /// Return a member type from a nominal declaration.
    fn symbol_member_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let member = if let Some(member) = self.visible_member_symbol(symbol, key)? {
            member
        } else {
            let Some(member) = self.extension_member_symbol(module, symbol, key)? else {
                return Ok(None);
            };

            member
        };
        let variable = self.member_type_variable(module, member)?;

        Ok(Some(TypeTerm::Variable(variable)))
    }

    /// Return a member from a component module or checked dependency.
    fn visible_member_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let symbols = self.visible_member_symbols(symbol, dir::MemberSlot::Key(key))?;
        let symbol = symbols.first().copied();

        Ok(symbol)
    }

    /// Return role members from a component module or checked dependency.
    pub(in crate::check) fn visible_role_member_symbols(
        &mut self,
        symbol: dir::GlobalSymbolId,
        slots: &[dir::MemberSlot],
    ) -> CompilerResult<Vec<dir::GlobalSymbolId>> {
        let mut symbols = Vec::new();

        // collect each role slot in caller-specified priority order
        for slot in slots {
            symbols.extend(self.visible_member_symbols(symbol, *slot)?);
        }

        Ok(symbols)
    }

    /// Return members from a component module or checked dependency.
    fn visible_member_symbols(
        &mut self,
        symbol: dir::GlobalSymbolId,
        slot: dir::MemberSlot,
    ) -> CompilerResult<Vec<dir::GlobalSymbolId>> {
        if let Some(module) = self.modules.get(&symbol.module_id) {
            let bindings = module.binding_table();
            let view = module.input.view();
            let symbols = Self::binding_member_symbols(&bindings, Some(&view), symbol, slot);

            return Ok(symbols);
        }

        let dependency = self.dependency_input(symbol.module_id)?;
        let view = dependency.view();
        let symbols = Self::binding_member_symbols(&dependency.bindings, Some(&view), symbol, slot);

        Ok(symbols)
    }

    /// Return member symbols from one binding table.
    pub(in crate::check) fn binding_member_symbols(
        bindings: &dir::BindingTable<'_>,
        view: Option<&dir::View<'_>>,
        owner: dir::GlobalSymbolId,
        slot: dir::MemberSlot,
    ) -> Vec<dir::GlobalSymbolId> {
        for scope_id in bindings.scope_ids() {
            let scope = bindings.get_scope_by_id(scope_id);
            if scope.owner != Some(owner.local_id) {
                continue;
            }

            return match slot {
                dir::MemberSlot::Key(key) => scope
                    .find_symbol(key)
                    .map(|symbol| vec![symbol.into_global(owner.module_id)])
                    .unwrap_or_default(),
                role_slot => Self::binding_role_member_symbols(
                    bindings,
                    view,
                    owner.module_id,
                    scope,
                    role_slot,
                ),
            };
        }

        Vec::new()
    }

    /// Return role member symbols from one owner scope.
    fn binding_role_member_symbols(
        bindings: &dir::BindingTable<'_>,
        view: Option<&dir::View<'_>>,
        module: ModuleId,
        scope: &dir::Scope,
        role_slot: dir::MemberSlot,
    ) -> Vec<dir::GlobalSymbolId> {
        let Some(view) = view else {
            return Vec::new();
        };
        let mut symbols = Vec::new();

        // scan anonymous owner members in source order
        for symbol in scope.anonymous_symbols() {
            let entry = bindings.get_symbol(symbol);
            let Some(member_slot) = Self::symbol_member_slot(view, entry) else {
                continue;
            };
            if member_slot == role_slot {
                symbols.push(symbol.into_global(module));
            }
        }

        symbols
    }

    /// Return the slot carried by one anonymous member symbol.
    fn symbol_member_slot(view: &dir::View<'_>, symbol: &dir::Symbol) -> Option<dir::MemberSlot> {
        let declaration = symbol.declaration?;
        if declaration.local_id.ty != dir::NodeType::Member {
            return None;
        }
        let member_id = dir::LocalNodeId::<dir::Member>::new(declaration.local_id.id);

        view.get(member_id).slot()
    }

    /// Return the local solver variable for one selected member symbol.
    pub(in crate::check) fn member_type_variable(
        &mut self,
        module: ModuleId,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<VariableId> {
        if self.modules.contains_key(&member.module_id) {
            let variable = self
                .module_mut(member.module_id)?
                .symbol_type_variable(member);

            return Ok(variable);
        }

        self.define_dependency_symbol(module, member)?;

        Ok(self.module_mut(module)?.symbol_type_variable(member))
    }

    /// Return the local solver variable for one selected static member symbol.
    pub(in crate::check) fn member_static_variable(
        &mut self,
        module: ModuleId,
        member: dir::GlobalSymbolId,
    ) -> CompilerResult<VariableId> {
        if self.modules.contains_key(&member.module_id) {
            let variable = self
                .module_mut(member.module_id)?
                .symbol_static_variable(member);

            return Ok(variable);
        }

        self.define_dependency_symbol(module, member)?;

        Ok(self.module_mut(module)?.symbol_static_variable(member))
    }

    /// Return a member static from one nominal declaration.
    fn symbol_member_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticTerm>> {
        let Some(member) = self.visible_member_symbol(symbol, key)? else {
            return Ok(None);
        };
        let variable = self.member_static_variable(module, member)?;

        Ok(Some(StaticTerm::Variable(variable)))
    }

    /// Return a member type from one applied nominal declaration.
    fn applied_symbol_member_type(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
        key: dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        if let Some(member) = self.visible_member_symbol(symbol, key)? {
            let variable = self.member_type_variable(module, member)?;
            let substitution = self.generic_substitution(symbol, arguments)?;
            let term = TypeTerm::Variable(variable);
            if substitution.is_empty() {
                return Ok(Some(term));
            }

            return term.substitute(module, &substitution, self);
        }

        let Some((_, member, substitution)) =
            self.extension_member(module, symbol, arguments, key, None)?
        else {
            return Ok(None);
        };
        let variable = self.member_type_variable(module, member)?;
        let term = TypeTerm::Variable(variable);
        if substitution.is_empty() {
            return Ok(Some(term));
        }

        term.substitute(module, &substitution, self)
    }

    /// Return a member static from one applied nominal declaration.
    fn applied_symbol_member_static(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
        key: dir::StaticKey,
    ) -> CompilerResult<Option<StaticTerm>> {
        let Some(member) = self.visible_member_symbol(symbol, key)? else {
            return Ok(None);
        };
        let variable = self.member_static_variable(module, member)?;
        let substitution = self.generic_substitution(symbol, arguments)?;
        let term = StaticTerm::Variable(variable);
        if substitution.is_empty() {
            return Ok(Some(term));
        }

        term.substitute(module, &substitution, self).map(Some)
    }

    /// Return a member candidate from one applied nominal declaration.
    fn applied_symbol_member_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
        key: dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        if let Some(member) = self.visible_member_symbol(symbol, key)? {
            let variable = self.member_type_variable(module, member)?;
            let mut substitution = self.generic_substitution(symbol, arguments)?;
            if let Some(protocol) = protocol
                && !self.symbol_matches_protocol(symbol, protocol, &mut substitution)?
            {
                return Ok(None);
            }
            if substitution.is_empty() {
                return Ok(Some(MemberCandidate {
                    symbol: member,
                    ty: variable,
                    instance: None,
                }));
            }
            let term = TypeTerm::Variable(variable);
            let Some(term) = term.substitute(module, &substitution, self)? else {
                return Ok(None);
            };
            let variable = self.push_solved_type_variable(module, term)?;
            let instance = Some(GenericInstance {
                symbol,
                arguments: arguments.to_vec(),
            });

            return Ok(Some(MemberCandidate {
                symbol: member,
                ty: variable,
                instance,
            }));
        }

        let Some((extension, member, substitution)) =
            self.extension_member(module, symbol, arguments, key, protocol)?
        else {
            return Ok(None);
        };
        let variable = self
            .module_mut(member.module_id)?
            .symbol_type_variable(member);
        if substitution.is_empty() {
            return Ok(Some(MemberCandidate {
                symbol: member,
                ty: variable,
                instance: None,
            }));
        }
        let term = TypeTerm::Variable(variable);
        let Some(term) = term.substitute(module, &substitution, self)? else {
            return Ok(None);
        };
        let variable = self.push_solved_type_variable(module, term)?;
        let instance = self.substitution_instance(extension, &substitution)?;

        Ok(Some(MemberCandidate {
            symbol: member,
            ty: variable,
            instance,
        }))
    }

    /// Return the type constraint for one generic type symbol.
    fn generic_type_constraint(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<VariableId>> {
        let Some(module) = self.modules.get_mut(&symbol.module_id) else {
            return Ok(None);
        };
        let variable = module.symbol_type_variable(symbol);
        let VariableOrigin::Generic(generic) = &module.variable(variable).origin else {
            return Ok(None);
        };
        Ok(generic.type_constraint())
    }

    /// Return whether one resolved member belongs to an owner implementing a language item.
    pub(in crate::check) fn member_implements_language_item(
        &mut self,
        member: dir::GlobalSymbolId,
        item: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        let Some(owner) = self.member_owner_symbol(member) else {
            return Ok(false);
        };

        self.symbol_implements_language_item(owner, item)
    }

    /// Return whether one language item satisfies a required protocol item.
    fn language_item_satisfies(
        &self,
        actual: dir::LanguageItem,
        required: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        let Some(actual) = self.environment.language.symbol(actual) else {
            return Err(CompilerError::Internal {
                message: format!("language item {actual} has no symbol"),
            });
        };
        let Some(required) = self.environment.language.symbol(required) else {
            return Err(CompilerError::Internal {
                message: format!("language item {required} has no symbol"),
            });
        };
        let actual = TypeTerm::Reference {
            source: None,
            symbol: actual,
            arguments: Vec::new(),
        };
        let required = TypeTerm::Reference {
            source: None,
            symbol: required,
            arguments: Vec::new(),
        };

        Ok(
            self.decide_type_term_relation(TypeRelation::Extends, &actual, &required)?
                == Decision::Yes,
        )
    }

    /// Return the owning declaration symbol for one resolved member.
    fn member_owner_symbol(&self, member: dir::GlobalSymbolId) -> Option<dir::GlobalSymbolId> {
        let module = self.modules.get(&member.module_id)?;
        let bindings = module.binding_table();
        let member = bindings.get_symbol(member.local_id);
        let scope = bindings.get_scope(member.scope);

        scope.owner.map(|owner| owner.into_global(module.module()))
    }

    /// Return whether one declaration implements a language item.
    fn symbol_implements_language_item(
        &mut self,
        symbol: dir::GlobalSymbolId,
        item: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        let implemented = self.implemented_type_expressions(symbol);

        for implemented in implemented {
            let variable = self
                .module_mut(symbol.module_id)?
                .type_expression_variable(implemented);
            let Some(term) = self.solved_type_term(variable)? else {
                continue;
            };
            if self.type_term_is_language_item(symbol.module_id, &term, item)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return whether one declaration implements the required protocol.
    pub(in crate::check) fn symbol_matches_protocol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        protocol: &MemberProtocol,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let implemented = self.implemented_type_expressions(symbol);

        for implemented in implemented {
            let variable = self
                .module_mut(symbol.module_id)?
                .type_expression_variable(implemented);
            let Some(term) = self.solved_type_term(variable)? else {
                continue;
            };
            let mut candidate = substitution.clone();
            if self.match_protocol_term(
                symbol.module_id,
                symbol,
                &term,
                protocol,
                &mut candidate,
            )? {
                *substitution = candidate;

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Match one implemented protocol type against a required protocol.
    fn match_protocol_term(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
        protocol: &MemberProtocol,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let is_match = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                return self.match_protocol_term(
                    variable.module,
                    owner,
                    &term,
                    protocol,
                    substitution,
                );
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.solved_type_term(*payload)? else {
                    return Ok(false);
                };

                return self.match_protocol_term(module, owner, &term, protocol, substitution);
            }
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => {
                let Some(actual) = self.environment.language.item(*symbol) else {
                    return Ok(false);
                };

                self.language_item_satisfies(actual, protocol.item)?
                    && self.match_protocol_arguments(
                        module,
                        owner,
                        arguments,
                        protocol,
                        substitution,
                    )?
            }
            _ => false,
        };

        Ok(is_match)
    }

    /// Match required protocol generic arguments.
    fn match_protocol_arguments(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        actual: &[ArgumentTerm],
        protocol: &MemberProtocol,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if protocol.arguments.is_empty() {
            return Ok(true);
        }
        if actual.len() != protocol.arguments.len() {
            return Ok(false);
        }

        for (actual, required) in actual.iter().zip(&protocol.arguments) {
            if !self.match_argument_pattern(module, owner, actual, required, substitution)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return implemented type expressions for one declaration symbol.
    fn implemented_type_expressions(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::LocalNodeId<dir::TypeExpression>> {
        let Some(module) = self.modules.get(&symbol.module_id) else {
            return Vec::new();
        };
        let bindings = module.binding_table();
        let symbol = bindings.get_symbol(symbol.local_id);
        let Some(declaration) = symbol.declaration else {
            return Vec::new();
        };
        if declaration.module_id != module.module()
            || declaration.local_id.ty != dir::NodeType::Declaration
        {
            return Vec::new();
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(declaration.local_id.id);

        match module.input.view().get(declaration) {
            dir::Declaration::Class(declaration) => declaration.implements_types.clone(),
            dir::Declaration::Struct(declaration) => declaration.implements_types.clone(),
            dir::Declaration::Enum(declaration) => declaration.implements_types.clone(),
            dir::Declaration::Extension(declaration) => declaration.implements_types.clone(),
            _ => Vec::new(),
        }
    }

    /// Return whether one type term names a language item.
    fn type_term_is_language_item(
        &self,
        module: ModuleId,
        term: &TypeTerm,
        item: dir::LanguageItem,
    ) -> CompilerResult<bool> {
        let is_item = match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                return self.type_term_is_language_item(variable.module, &term, item);
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.solved_type_term(*payload)? else {
                    return Ok(false);
                };

                return self.type_term_is_language_item(module, &term, item);
            }
            TypeTerm::Reference { symbol, .. } => self
                .environment
                .language
                .item(*symbol)
                .map(|actual| self.language_item_satisfies(actual, item))
                .transpose()?
                .unwrap_or(false),
            _ => false,
        };

        Ok(is_item)
    }

    /// Decide structural satisfaction after ordinary relation checks are inconclusive.
    pub(in crate::check) fn decide_structural_satisfies(
        &mut self,
        module: ModuleId,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let target = self.reduce_pattern_type(target)?;
        let decision = match target {
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } => self.decide_named_members_satisfied(module, source, symbol, &arguments)?,
            _ => Decision::Undecidable,
        };

        Ok(decision)
    }

    /// Decide whether a source type has every member required by a nominal target.
    fn decide_named_members_satisfied(
        &mut self,
        module: ModuleId,
        source: &TypeTerm,
        target: dir::GlobalSymbolId,
        target_arguments: &[ArgumentTerm],
    ) -> CompilerResult<Decision> {
        let members = self.nominal_member_declarations(target)?;
        let substitution = self.generic_substitution(target, target_arguments)?;
        let mut decision = Decision::Yes;

        for (key, target_member) in members {
            let Some(source_member) = self.member_type_term(module, source, &key)? else {
                return Ok(Decision::No);
            };
            let target_member = self
                .module_mut(target_member.module_id)?
                .symbol_type_variable(target_member);
            let mut target_member = TypeTerm::Variable(target_member);
            if !substitution.is_empty() {
                let Some(term) = target_member.substitute(module, &substitution, self)? else {
                    return Ok(Decision::Undecidable);
                };

                target_member = term;
            }
            decision = decision.and(self.decide_member_satisfied(&source_member, &target_member)?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return nominal member declarations in source order.
    fn nominal_member_declarations(
        &self,
        owner: dir::GlobalSymbolId,
    ) -> CompilerResult<Vec<(dir::StaticKey, dir::GlobalSymbolId)>> {
        let module = self.module(owner.module_id)?;
        let binding_table = module.binding_table();
        let mut members = Vec::new();

        for scope_id in binding_table.scope_ids() {
            let scope = binding_table.get_scope_by_id(scope_id);
            if scope.owner != Some(owner.local_id) {
                continue;
            }

            members.extend(
                scope
                    .named_symbols()
                    .map(|(key, symbol)| (key, symbol.into_global(owner.module_id))),
            );

            break;
        }

        Ok(members)
    }

    /// Decide whether one member can satisfy another.
    fn decide_member_satisfied(
        &mut self,
        source: &TypeTerm,
        target: &TypeTerm,
    ) -> CompilerResult<Decision> {
        let source = self.reduce_pattern_type(source)?;
        let target = self.reduce_pattern_type(target)?;
        let decision = match (&source, &target) {
            (TypeTerm::Function(source), TypeTerm::Function(target)) => {
                self.decide_member_function_satisfied(source, target)?
            }
            _ => self.decide_type_term_relation(TypeRelation::Assignable, &source, &target)?,
        };

        Ok(decision)
    }

    /// Decide member function satisfaction, ignoring expected `this`.
    fn decide_member_function_satisfied(
        &self,
        source: &FunctionTerm,
        target: &FunctionTerm,
    ) -> CompilerResult<Decision> {
        if source.asynchrony != target.asynchrony
            || source.is_generator != target.is_generator
            || source.parameters.len() != target.parameters.len()
        {
            return Ok(Decision::No);
        }
        let mut decision = Decision::Yes;

        for (source, target) in source.parameters.iter().zip(&target.parameters) {
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                *target,
                *source,
            )?);
            if decision == Decision::No {
                return Ok(decision);
            }
        }

        if let (Some(source), Some(target)) = (source.return_type, target.return_type) {
            decision = decision.and(self.decide_type_relation(
                TypeRelation::Assignable,
                source,
                target,
            )?);
        } else if source.return_type != target.return_type {
            decision = Decision::No;
        }

        Ok(decision)
    }

    /// Return a member type from one check shape term.
    fn shape_member_type(
        &self,
        members: &[ShapeMemberTerm],
        key: &dir::StaticKey,
    ) -> Option<TypeTerm> {
        for member in members {
            let ShapeMemberTerm::Field {
                key: member_key,
                ty,
                ..
            } = member
            else {
                continue;
            };
            if member_key.matches(key) {
                return Some(TypeTerm::Variable(*ty));
            }
        }

        None
    }

    /// Return a member type from one tuple term.
    fn tuple_member_type(
        &self,
        module: ModuleId,
        elements: &[TupleElementTerm],
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(index) = self.tuple_member_index(module, key)? else {
            return Ok(None);
        };
        let Some(element) = elements.get(index) else {
            return Ok(None);
        };

        Ok(Some(TypeTerm::Variable(element.ty)))
    }

    /// Return the tuple index selected by one member key.
    fn tuple_member_index(
        &self,
        module: ModuleId,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<usize>> {
        let dir::StaticKey::Number(number) = key else {
            return Ok(None);
        };
        let strings = &self.module(module)?.input.strings;
        let index = strings.get(*number).parse().ok();

        Ok(index)
    }

    /// Return whether one reduced type has a structural field.
    fn type_term_has_field(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<bool> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(false);
                };

                self.type_term_has_field(variable.module, &term, key)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.solved_type_term(*payload)? else {
                    return Ok(false);
                };

                self.type_term_has_field(module, &term, key)
            }
            TypeTerm::Shape { members } => Ok(self.shape_member_type(members, key).is_some()),
            _ => Ok(false),
        }
    }
}
