use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    ArgumentTerm, CheckComponentState, FunctionTerm, GenericInstance, MemberFailure, MemberOutcome,
    MemberProtocol, MemberResolution, MemberResolutionTarget, ShapeMemberTerm, StaticRelation,
    StaticTerm, TupleElementTerm, TypeRelation, TypeTerm, VariableId, VariableOrigin,
};
use crate::{CompilerError, CompilerResult};

use super::Decision;
use super::substitute::{GenericSubstitution, GenericSubstitutionEntry};

/// Extension member candidate visible to component checking.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ExtensionCandidate {
    /// The module that owns the extension declaration.
    pub(super) module: ModuleId,
    /// The extension declaration symbol.
    pub(super) symbol: dir::GlobalSymbolId,
    /// The extension target type expression.
    pub(super) target: dir::LocalNodeId<dir::TypeExpression>,
    /// The extension where clauses.
    pub(super) where_clauses: Vec<dir::LocalNodeId<dir::WhereClause>>,
    /// The resolved member symbol.
    pub(super) member: dir::GlobalSymbolId,
}

/// A symbol-backed member candidate found by lookup.
pub(super) struct MemberCandidate {
    /// The resolved member symbol.
    pub(super) symbol: dir::GlobalSymbolId,
    /// The resolved member type.
    pub(super) ty: VariableId,
    /// The resolved generic instance, when lookup instantiated an owner.
    pub(super) instance: Option<GenericInstance>,
}

impl CheckComponentState<'_> {
    /// Reduce one member projection to its type.
    pub(super) fn reduce_member_type(
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
    pub(super) fn member_type_term(
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
            TypeTerm::Literal(ty) => self.committed_member_type(module, ty, key),
            _ => Ok(None),
        }
    }

    /// Return a member static from one reduced type term.
    pub(super) fn member_static_term(
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
            TypeTerm::Literal(dir::Type::Named(named)) => {
                let arguments = self.argument_terms_from_static_arguments(
                    module,
                    named.symbol,
                    &named.arguments,
                )?;

                self.applied_symbol_member_static(module, named.symbol, &arguments, *key)
            }
            TypeTerm::Literal(dir::Type::Form(form)) => {
                let ty = self.module(module)?.get_type(form.value);
                let term = TypeTerm::Literal(ty);

                self.member_static_term(module, &term, key)
            }
            _ => Ok(None),
        }
    }

    /// Return a symbol-backed member candidate from one reduced type term.
    pub(super) fn member_type_candidate(
        &mut self,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<MemberCandidate>> {
        self.member_type_candidate_matching(module, term, key, None)
    }

    /// Return a symbol-backed member candidate that satisfies one protocol.
    pub(super) fn member_type_candidate_for_protocol(
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
            TypeTerm::Literal(dir::Type::Form(form)) => {
                let ty = self.module(module)?.get_type(form.value);

                self.committed_member_candidate(module, &ty, key, protocol)
            }
            TypeTerm::Literal(ty) => self.committed_member_candidate(module, ty, key, protocol),
            _ => Ok(None),
        }
    }

    /// Return a symbol-backed member candidate from one committed DIR type.
    fn committed_member_candidate(
        &mut self,
        module: ModuleId,
        ty: &dir::Type,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        match ty {
            dir::Type::Form(form) => {
                let ty = self.module(module)?.get_type(form.value);

                self.committed_member_candidate(module, &ty, key, protocol)
            }
            dir::Type::Named(named) => {
                self.committed_named_member_candidate(module, named, *key, protocol)
            }
            dir::Type::Parameter(parameter) => {
                self.parameter_member_candidate(parameter.symbol, key, protocol)
            }
            _ => Ok(None),
        }
    }

    /// Return a member type from one committed DIR type.
    fn committed_member_type(
        &mut self,
        module: ModuleId,
        ty: &dir::Type,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        match ty {
            dir::Type::Form(form) => {
                let ty = self.module(module)?.get_type(form.value);

                self.committed_member_type(module, &ty, key)
            }
            dir::Type::Named(named) => self.committed_named_member_type(module, named, *key),
            dir::Type::Parameter(parameter) => self.parameter_member_type(parameter.symbol, key),
            dir::Type::Tuple(tuple) => self.committed_tuple_member_type(module, tuple, key),
            dir::Type::Shape(shape) => self.committed_shape_member_type(module, shape, key),
            _ => Ok(None),
        }
    }

    /// Return a member type from one applied named DIR type.
    fn committed_named_member_type(
        &mut self,
        module: ModuleId,
        named: &dir::NamedType,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let arguments =
            self.argument_terms_from_static_arguments(module, named.symbol, &named.arguments)?;

        self.applied_symbol_member_type(module, named.symbol, &arguments, key)
    }

    /// Return a member candidate from one applied named DIR type.
    fn committed_named_member_candidate(
        &mut self,
        module: ModuleId,
        named: &dir::NamedType,
        key: dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        let arguments =
            self.argument_terms_from_static_arguments(module, named.symbol, &named.arguments)?;

        self.applied_symbol_member_candidate(module, named.symbol, &arguments, key, protocol)
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
            let Some(member) = self.extension_member_symbol(symbol, key)? else {
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
    pub(super) fn visible_role_member_symbols(
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
            let symbols = Self::binding_member_symbols(
                &bindings,
                Some(&module.input.parsed.tree),
                symbol,
                slot,
            );

            return Ok(symbols);
        }

        let dependency = self.dependency_input(symbol.module_id)?;
        let symbols = Self::binding_member_symbols(
            &dependency.bindings,
            Some(&dependency.parsed.tree),
            symbol,
            slot,
        );

        Ok(symbols)
    }

    /// Return member symbols from one binding table.
    fn binding_member_symbols(
        bindings: &dir::BindingTable<'_>,
        tree: Option<&dir::Tree>,
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
                    tree,
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
        tree: Option<&dir::Tree>,
        module: ModuleId,
        scope: &dir::Scope,
        role_slot: dir::MemberSlot,
    ) -> Vec<dir::GlobalSymbolId> {
        let Some(tree) = tree else {
            return Vec::new();
        };
        let mut symbols = Vec::new();

        // scan anonymous owner members in source order
        for symbol in scope.anonymous_symbols() {
            let entry = bindings.get_symbol(symbol);
            let Some(member_slot) = Self::symbol_member_slot(tree, entry) else {
                continue;
            };
            if member_slot == role_slot {
                symbols.push(symbol.into_global(module));
            }
        }

        symbols
    }

    /// Return the slot carried by one anonymous member symbol.
    fn symbol_member_slot(tree: &dir::Tree, symbol: &dir::Symbol) -> Option<dir::MemberSlot> {
        let declaration = symbol.declaration?;
        if declaration.local_id.ty != dir::NodeType::Member {
            return None;
        }
        let member_id = dir::LocalNodeId::<dir::Member>::new(declaration.local_id.id);

        tree.get(member_id).slot()
    }

    /// Return the local solver variable for one selected member symbol.
    pub(super) fn member_type_variable(
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
    pub(super) fn member_static_variable(
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

            return self.substitute_type_term(module, &substitution, &term);
        }

        let Some((_, member, substitution)) =
            self.extension_member(symbol, arguments, key, None)?
        else {
            return Ok(None);
        };
        let variable = self.member_type_variable(module, member)?;
        let term = TypeTerm::Variable(variable);
        if substitution.is_empty() {
            return Ok(Some(term));
        }

        self.substitute_type_term(module, &substitution, &term)
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

        self.substitute_static_term(module, &substitution, &term)
            .map(Some)
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
            let Some(term) = self.substitute_type_term(module, &substitution, &term)? else {
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
            self.extension_member(symbol, arguments, key, protocol)?
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
        let Some(term) = self.substitute_type_term(module, &substitution, &term)? else {
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

    /// Return the generic substitution for one applied symbol.
    pub(super) fn generic_substitution(
        &self,
        owner: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
    ) -> CompilerResult<GenericSubstitution> {
        if arguments.is_empty() {
            return Ok(GenericSubstitution::empty());
        }
        let Some(module) = self.modules.get(&owner.module_id) else {
            return Ok(GenericSubstitution::empty());
        };
        let mut slots = module
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner == owner)
            .map(|(variable, generic)| {
                let symbol = match generic.slot().key {
                    dir::GenericSlotKey::Symbol(symbol) => Some(symbol),
                    dir::GenericSlotKey::Generated(_) => None,
                };

                (generic.slot().index, variable, symbol)
            })
            .collect::<Vec<_>>();

        slots.sort_by_key(|(index, _, _)| *index);

        let entries = slots
            .into_iter()
            .zip(arguments.iter().cloned())
            .map(
                |((_, variable, symbol), argument)| GenericSubstitutionEntry {
                    variable,
                    symbol,
                    argument,
                },
            )
            .collect();

        Ok(GenericSubstitution { entries })
    }

    /// Return the concrete instance described by one substitution.
    fn substitution_instance(
        &self,
        owner: dir::GlobalSymbolId,
        substitution: &GenericSubstitution,
    ) -> CompilerResult<Option<GenericInstance>> {
        let Some(module) = self.modules.get(&owner.module_id) else {
            return Ok(None);
        };
        let mut arguments = module
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner == owner)
            .filter_map(|(variable, generic)| {
                let argument = if generic.is_type() {
                    substitution.type_variable(variable).map(ArgumentTerm::Type)
                } else {
                    substitution
                        .static_variable(variable)
                        .map(ArgumentTerm::Static)
                }?;

                Some((generic.slot().index, argument))
            })
            .collect::<Vec<_>>();

        if arguments.is_empty() {
            return Ok(None);
        }
        arguments.sort_by_key(|(index, _)| *index);
        let arguments = arguments
            .into_iter()
            .map(|(_, argument)| argument)
            .collect::<Vec<_>>();

        Ok(Some(GenericInstance {
            symbol: owner,
            arguments,
        }))
    }

    /// Lower committed static arguments back into check arguments.
    pub(super) fn argument_terms_from_static_arguments(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        arguments: &[dir::StaticArgument],
    ) -> CompilerResult<Vec<ArgumentTerm>> {
        if arguments.is_empty() {
            return Ok(Vec::new());
        }
        let Some(owner_module) = self.modules.get(&owner.module_id) else {
            return Ok(Vec::new());
        };
        let mut slots = owner_module
            .generic_parameters()
            .filter(|(_, generic)| generic.slot().owner == owner)
            .map(|(variable, generic)| (generic.slot().index, variable, generic.is_type()))
            .collect::<Vec<_>>();

        slots.sort_by_key(|(index, _, _)| *index);

        let mut terms = Vec::with_capacity(arguments.len());
        for ((_, _, is_type), argument) in slots.into_iter().zip(arguments) {
            let value = self.module(module)?.get_static(argument.value);
            if is_type {
                let dir::StaticTerm::Type { ty } = value else {
                    continue;
                };
                let ty = self.module(module)?.get_type(ty);
                let variable = self.push_solved_type_variable(module, TypeTerm::Literal(ty))?;

                terms.push(ArgumentTerm::Type(variable));
            } else {
                let variable = self.push_solved_static_value_variable(module, value)?;

                terms.push(ArgumentTerm::Static(variable));
            }
        }

        Ok(terms)
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

    /// Return an applicable extension member for one applied nominal receiver.
    fn extension_member(
        &mut self,
        target: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
        key: dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<
        Option<(
            dir::GlobalSymbolId,
            dir::GlobalSymbolId,
            GenericSubstitution,
        )>,
    > {
        let mut extensions = Vec::new();

        // collect candidates before solving target expressions
        for (module_id, module) in &self.modules {
            for (id, declaration) in module
                .input
                .parsed
                .tree
                .iter_nodes_of_type::<dir::Declaration>()
            {
                let dir::Declaration::Extension(extension) = declaration else {
                    continue;
                };
                let Some(symbol) = module.declaration_symbol(id.into_any()) else {
                    continue;
                };
                let Some(member) = module.member_symbol(symbol, key) else {
                    continue;
                };

                extensions.push(ExtensionCandidate {
                    module: *module_id,
                    symbol,
                    target: extension.target_type,
                    where_clauses: extension.where_clauses.clone(),
                    member,
                });
            }
        }

        // accept the first extension whose target pattern and constraints hold
        for extension in extensions {
            let Some(mut substitution) = self.extension_substitution(
                extension.module,
                extension.symbol,
                extension.target,
                target,
                arguments,
            )?
            else {
                continue;
            };
            if let Some(protocol) = protocol
                && !self.symbol_matches_protocol(extension.symbol, protocol, &mut substitution)?
            {
                continue;
            }

            if self.extension_where_clauses_hold(
                extension.module,
                &extension.where_clauses,
                &substitution,
            )? {
                return Ok(Some((extension.symbol, extension.member, substitution)));
            }
        }

        Ok(None)
    }

    /// Return a visible extension member for one unapplied nominal receiver.
    fn extension_member_symbol(
        &mut self,
        target: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        let member = self
            .extension_member(target, &[], key, None)?
            .map(|(_, member, _)| member);

        Ok(member)
    }

    /// Return whether one resolved member belongs to an owner implementing a language item.
    pub(super) fn member_implements_language_item(
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
    fn symbol_matches_protocol(
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
            TypeTerm::Literal(dir::Type::Named(named)) => {
                let Some(actual) = self.environment.language.item(named.symbol) else {
                    return Ok(false);
                };
                let arguments = self.argument_terms_from_static_arguments(
                    module,
                    named.symbol,
                    &named.arguments,
                )?;

                self.language_item_satisfies(actual, protocol.item)?
                    && self.match_protocol_arguments(
                        module,
                        owner,
                        &arguments,
                        protocol,
                        substitution,
                    )?
            }
            TypeTerm::Literal(dir::Type::Form(form)) => {
                let ty = self.module(module)?.get_type(form.value);

                return self.match_protocol_term(
                    module,
                    owner,
                    &TypeTerm::Literal(ty),
                    protocol,
                    substitution,
                );
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

        match module.input.parsed.tree.get(declaration) {
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
            TypeTerm::Literal(dir::Type::Named(named)) => self
                .environment
                .language
                .item(named.symbol)
                .map(|actual| self.language_item_satisfies(actual, item))
                .transpose()?
                .unwrap_or(false),
            TypeTerm::Literal(dir::Type::Form(form)) => {
                let ty = self.module(module)?.get_type(form.value);

                return self.type_term_is_language_item(module, &TypeTerm::Literal(ty), item);
            }
            _ => false,
        };

        Ok(is_item)
    }

    /// Match an extension target pattern against an applied receiver.
    fn extension_substitution(
        &mut self,
        module: ModuleId,
        extension: dir::GlobalSymbolId,
        target_type: dir::LocalNodeId<dir::TypeExpression>,
        receiver: dir::GlobalSymbolId,
        arguments: &[ArgumentTerm],
    ) -> CompilerResult<Option<GenericSubstitution>> {
        let target_variable = self
            .module_mut(module)?
            .type_expression_variable(target_type);
        let Some(pattern) = self.solved_type_term(target_variable)? else {
            return Ok(None);
        };
        let actual = TypeTerm::Reference {
            source: None,
            symbol: receiver,
            arguments: arguments.to_vec(),
        };
        let mut substitution = GenericSubstitution::empty();
        let is_match =
            self.match_type_pattern(module, extension, &pattern, &actual, &mut substitution)?;

        Ok(is_match.then_some(substitution))
    }

    /// Return whether substituted extension where clauses hold.
    fn extension_where_clauses_hold(
        &mut self,
        module: ModuleId,
        where_clauses: &[dir::LocalNodeId<dir::WhereClause>],
        substitution: &GenericSubstitution,
    ) -> CompilerResult<bool> {
        for where_clause in where_clauses {
            let where_clause = self
                .module(module)?
                .input
                .parsed
                .tree
                .get(*where_clause)
                .clone();
            let left = self
                .module_mut(module)?
                .type_expression_variable(where_clause.left);
            let right = self
                .module_mut(module)?
                .type_expression_variable(where_clause.right);

            let Some(left) = self.solved_type_term(left)? else {
                return Ok(false);
            };
            let Some(right) = self.solved_type_term(right)? else {
                return Ok(false);
            };
            let Some(left) = self.substitute_type_term(module, substitution, &left)? else {
                return Ok(false);
            };
            let Some(right) = self.substitute_type_term(module, substitution, &right)? else {
                return Ok(false);
            };

            let decision =
                self.decide_type_term_relation(TypeRelation::Satisfies, &left, &right)?;
            let decision = if decision == Decision::Undecidable {
                self.decide_structural_satisfies(module, &left, &right)?
            } else {
                decision
            };
            if decision != Decision::Yes {
                return Ok(false);
            };
        }

        Ok(true)
    }

    /// Match a type pattern and collect generic substitutions.
    fn match_type_pattern(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        pattern: &TypeTerm,
        actual: &TypeTerm,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some((variable, symbol)) = self.type_pattern_generic(owner, pattern)? {
            return self.match_type_generic(module, variable, symbol, actual, substitution);
        }

        let pattern = self.reduce_pattern_type(pattern)?;
        let actual = self.reduce_pattern_type(actual)?;
        if let Some((variable, symbol)) = self.type_pattern_generic(owner, &pattern)? {
            return self.match_type_generic(module, variable, symbol, &actual, substitution);
        }

        let is_match = match (&pattern, &actual) {
            (
                TypeTerm::Reference {
                    source: _,
                    symbol: left,
                    arguments: left_arguments,
                },
                TypeTerm::Reference {
                    source: _,
                    symbol: right,
                    arguments: right_arguments,
                },
            ) => {
                if left != right || left_arguments.len() != right_arguments.len() {
                    false
                } else {
                    let mut is_match = true;
                    for (left, right) in left_arguments.iter().zip(right_arguments) {
                        is_match =
                            self.match_argument_pattern(module, owner, left, right, substitution)?;
                        if !is_match {
                            break;
                        }
                    }

                    is_match
                }
            }
            (left, right) => {
                self.decide_type_term_relation(TypeRelation::Equal, left, right)? == Decision::Yes
            }
        };

        Ok(is_match)
    }

    /// Match one generic argument pattern.
    fn match_argument_pattern(
        &mut self,
        module: ModuleId,
        owner: dir::GlobalSymbolId,
        pattern: &ArgumentTerm,
        actual: &ArgumentTerm,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let is_match = match (pattern, actual) {
            (ArgumentTerm::Type(pattern), ArgumentTerm::Type(actual))
            | (ArgumentTerm::SpreadType(pattern), ArgumentTerm::SpreadType(actual)) => {
                let pattern = TypeTerm::Variable(*pattern);
                let actual = TypeTerm::Variable(*actual);

                self.match_type_pattern(module, owner, &pattern, &actual, substitution)?
            }
            (ArgumentTerm::Static(pattern), ArgumentTerm::Static(actual))
            | (ArgumentTerm::SpreadStatic(pattern), ArgumentTerm::SpreadStatic(actual)) => {
                if let Some((variable, symbol)) = self.variable_static_generic(owner, *pattern)? {
                    self.match_static_generic(variable, symbol, *actual, substitution)?
                } else {
                    self.decide_static_relation(StaticRelation::Equal, *pattern, *actual)?
                        == Decision::Yes
                }
            }
            _ => false,
        };

        Ok(is_match)
    }

    /// Match one generic type slot against an actual type.
    fn match_type_generic(
        &mut self,
        module: ModuleId,
        variable: VariableId,
        symbol: Option<dir::GlobalSymbolId>,
        actual: &TypeTerm,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        let actual = match actual {
            TypeTerm::Variable(variable) => *variable,
            term => self.push_solved_type_variable(module, term.clone())?,
        };
        if let Some(existing) = substitution.type_variable(variable) {
            let decision = self.decide_type_relation(TypeRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            variable,
            symbol,
            argument: ArgumentTerm::Type(actual),
        });

        Ok(true)
    }

    /// Match one generic static slot against an actual static value.
    fn match_static_generic(
        &mut self,
        variable: VariableId,
        symbol: Option<dir::GlobalSymbolId>,
        actual: VariableId,
        substitution: &mut GenericSubstitution,
    ) -> CompilerResult<bool> {
        if let Some(existing) = substitution.static_variable(variable) {
            let decision = self.decide_static_relation(StaticRelation::Equal, existing, actual)?;

            return Ok(decision != Decision::No);
        }

        substitution.entries.push(GenericSubstitutionEntry {
            variable,
            symbol,
            argument: ArgumentTerm::Static(actual),
        });

        Ok(true)
    }

    /// Return a generic type slot represented by a pattern term.
    fn type_pattern_generic(
        &self,
        owner: dir::GlobalSymbolId,
        term: &TypeTerm,
    ) -> CompilerResult<Option<(VariableId, Option<dir::GlobalSymbolId>)>> {
        let slot = match term {
            TypeTerm::Variable(variable) => self.variable_type_generic(owner, *variable)?,
            TypeTerm::Reference {
                source: _,
                symbol,
                arguments,
            } if arguments.is_empty() => self.symbol_type_generic(owner, *symbol)?,
            TypeTerm::Literal(dir::Type::Parameter(parameter)) => {
                self.symbol_type_generic(owner, parameter.symbol)?
            }
            _ => None,
        };

        Ok(slot)
    }

    /// Return a generic type slot represented by a variable.
    fn variable_type_generic(
        &self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
    ) -> CompilerResult<Option<(VariableId, Option<dir::GlobalSymbolId>)>> {
        let module = self.module(variable.module)?;
        let VariableOrigin::Generic(generic) = &module.variable(variable).origin else {
            return Ok(None);
        };
        if generic.slot().owner != owner || !generic.is_type() {
            return Ok(None);
        }
        let symbol = match generic.slot().key {
            dir::GenericSlotKey::Symbol(symbol) => Some(symbol),
            dir::GenericSlotKey::Generated(_) => None,
        };

        Ok(Some((variable, symbol)))
    }

    /// Return a generic static slot represented by a variable.
    fn variable_static_generic(
        &self,
        owner: dir::GlobalSymbolId,
        variable: VariableId,
    ) -> CompilerResult<Option<(VariableId, Option<dir::GlobalSymbolId>)>> {
        let module = self.module(variable.module)?;
        let VariableOrigin::Generic(generic) = &module.variable(variable).origin else {
            return Ok(None);
        };
        if generic.slot().owner != owner || !generic.is_static() {
            return Ok(None);
        }
        let symbol = match generic.slot().key {
            dir::GenericSlotKey::Symbol(symbol) => Some(symbol),
            dir::GenericSlotKey::Generated(_) => None,
        };

        Ok(Some((variable, symbol)))
    }

    /// Return a generic type slot represented by a symbol.
    fn symbol_type_generic(
        &self,
        owner: dir::GlobalSymbolId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(VariableId, Option<dir::GlobalSymbolId>)>> {
        let module = self.module(symbol.module_id)?;
        let slot = module.generic_parameters().find_map(|(variable, generic)| {
            if generic.slot().owner == owner
                && generic.slot().key == dir::GenericSlotKey::Symbol(symbol)
                && generic.is_type()
            {
                Some((variable, Some(symbol)))
            } else {
                None
            }
        });

        Ok(slot)
    }

    /// Reduce variables and forms while matching type patterns.
    fn reduce_pattern_type(&self, term: &TypeTerm) -> CompilerResult<TypeTerm> {
        let term = match term {
            TypeTerm::Variable(variable) => {
                if let Some(term) = self.solved_type_term(*variable)? {
                    term
                } else {
                    term.clone()
                }
            }
            TypeTerm::Form { payload, .. } => {
                if let Some(term) = self.solved_type_term(*payload)? {
                    term
                } else {
                    term.clone()
                }
            }
            _ => term.clone(),
        };

        Ok(term)
    }

    /// Decide structural satisfaction after ordinary relation checks are inconclusive.
    fn decide_structural_satisfies(
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
            TypeTerm::Literal(dir::Type::Named(named)) => {
                let arguments = self.argument_terms_from_static_arguments(
                    module,
                    named.symbol,
                    &named.arguments,
                )?;

                self.decide_named_members_satisfied(module, source, named.symbol, &arguments)?
            }
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
                let Some(term) =
                    self.substitute_type_term(module, &substitution, &target_member)?
                else {
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

    /// Return a member type from one committed DIR tuple.
    fn committed_tuple_member_type(
        &self,
        module: ModuleId,
        tuple: &dir::TupleType,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        let Some(index) = self.tuple_member_index(module, key)? else {
            return Ok(None);
        };
        let Some(element) = tuple.elements.get(index) else {
            return Ok(None);
        };
        let ty = self.module(module)?.get_type(element.ty);

        Ok(Some(TypeTerm::Literal(ty)))
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
            TypeTerm::Literal(dir::Type::Form(form)) => {
                let ty = self.module(module)?.get_type(form.value);

                self.type_term_has_field(module, &TypeTerm::Literal(ty), key)
            }
            TypeTerm::Literal(dir::Type::Shape(shape)) => Ok(self
                .committed_shape_member_type(module, shape, key)?
                .is_some()),
            _ => Ok(false),
        }
    }

    /// Return a member type from one committed DIR shape.
    fn committed_shape_member_type(
        &self,
        module: ModuleId,
        shape: &dir::ShapeType,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<TypeTerm>> {
        for field in &shape.fields {
            if field.key.matches(key) {
                let ty = self.module(module)?.get_type(field.ty);

                return Ok(Some(TypeTerm::Literal(ty)));
            }
        }

        Ok(None)
    }
}
