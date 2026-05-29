use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{
    CheckDependencyState, CheckState, Decision, GenericArgument, GenericInstance, GenericSlotId,
    GenericSubstitution, MemberProtocol, Origin, ShapeMember, TypeRelation, TypeTerm, VariableId,
    VariableOutput,
};

/// A symbol-backed member candidate found by lookup.
pub(in crate::check) struct MemberCandidate {
    /// The resolved member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The resolved member type.
    pub(in crate::check) ty: TypeTerm,
    /// The resolved generic instance, when lookup instantiated an owner.
    pub(in crate::check) instance: Option<GenericInstance>,
}

/// Extension member candidate visible to component checking.
#[derive(Debug, Clone, PartialEq)]
struct ExtensionCandidate {
    /// The extension declaration symbol.
    symbol: dir::GlobalSymbolId,
    /// The extension target type.
    target: VariableId,
    /// The extension where clauses.
    where_clauses: Vec<ExtensionWhereClause>,
    /// The resolved member symbol.
    member: dir::GlobalSymbolId,
}

/// Where clause attached to an extension candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExtensionWhereClause {
    /// The source where clause node.
    source: dir::GlobalNodeIdAny,
    /// The constrained type.
    left: VariableId,
    /// The required constraint type.
    right: VariableId,
}

impl CheckState<'_> {
    /// Return a symbol-backed member candidate from one reduced type term.
    pub(in crate::check) fn member_type_candidate(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<MemberCandidate>> {
        let Some(mut candidates) = self.member_type_candidates(origin, module, term, key)? else {
            return Ok(None);
        };
        if candidates.len() != 1 {
            return Ok(None);
        }

        Ok(candidates.pop())
    }

    /// Return symbol-backed member candidates from one reduced type term.
    pub(in crate::check) fn member_type_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let reduction = self.reduce_type_term(origin, term)?;
        let term = reduction.value.as_ref().unwrap_or(term);

        self.member_type_candidates_matching(origin, module, term, key, None)
    }

    /// Return a symbol-backed member candidate that satisfies one protocol.
    pub(in crate::check) fn member_type_candidate_for_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Option<MemberCandidate>> {
        let Some(mut candidates) =
            self.member_type_candidates_for_protocol(origin, module, term, key, protocol)?
        else {
            return Ok(None);
        };
        if candidates.len() != 1 {
            return Ok(None);
        }

        Ok(candidates.pop())
    }

    /// Return symbol-backed member candidates that satisfy one protocol.
    pub(in crate::check) fn member_type_candidates_for_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let reduction = self.reduce_type_term(origin, term)?;
        let term = reduction.value.as_ref().unwrap_or(term);

        self.member_type_candidates_matching(origin, module, term, key, Some(protocol))
    }

    /// Return symbol-backed member candidates from one reduced type term.
    pub(in crate::check) fn member_type_candidates_matching(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        match term {
            TypeTerm::Variable(variable) => {
                let Some(term) = self.solved_type_term(*variable)? else {
                    return Ok(None);
                };

                self.member_type_candidates_matching(origin, module, &term, key, protocol)
            }
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(None);
                };

                self.member_type_candidates_matching(origin, module, &term, key, protocol)
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                if let Some(parameter) = self.generic_type_reference(module, *symbol)? {
                    self.parameter_member_candidates(module, origin, parameter, key, protocol)
                } else {
                    self.instantiate_symbol_member_candidate(
                        module, *symbol, arguments, *key, protocol,
                    )
                    .map(|candidate| candidate.map(|candidate| vec![candidate]))
                }
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self
                .instantiate_symbol_member_candidate(module, *symbol, arguments, *key, protocol)
                .map(|candidate| candidate.map(|candidate| vec![candidate])),
            TypeTerm::Parameter(parameter) => {
                self.parameter_member_candidates(module, origin, *parameter, key, protocol)
            }
            TypeTerm::Shape { members } => self
                .shape_member_candidate(members, key, protocol)
                .map(|candidate| candidate.map(|candidate| vec![candidate])),
            TypeTerm::Union { elements } => {
                self.union_member_candidates(origin, module, elements, key, protocol)
            }
            TypeTerm::Operation(operation) => {
                self.operation_member_candidates(origin, module, *operation, key, protocol)
            }
            _ => Ok(None),
        }
    }

    /// Return member candidates through a generic parameter constraint.
    fn parameter_member_candidates(
        &mut self,
        module: ModuleId,
        origin: Origin,
        parameter: GenericSlotId,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let Some(constraint) = self.generic_parameter_type_constraint(module, parameter)? else {
            return Ok(None);
        };
        let term = self.reduced_generic_type_constraint(origin, constraint)?;

        self.member_type_candidates_matching(origin, module, &term, key, protocol)
    }

    /// Return a symbol-backed candidate from one shape method field.
    fn shape_member_candidate(
        &self,
        members: &[ShapeMember],
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        if protocol.is_some() {
            return Ok(None);
        }

        for member in members {
            let ShapeMember::Field {
                key: member_key,
                ty,
                ..
            } = member
            else {
                continue;
            };
            if !member_key.matches(key) {
                continue;
            };
            let Some(variable) = ty.variable() else {
                return Ok(None);
            };
            let Some(VariableOutput::Symbol(symbol)) = self.variable(variable).output else {
                return Ok(None);
            };

            return Ok(Some(MemberCandidate {
                symbol,
                ty: TypeTerm::Variable(variable),
                instance: None,
            }));
        }

        Ok(None)
    }

    /// Instantiate a member candidate from one applied nominal declaration.
    fn instantiate_symbol_member_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        if let Some(member) = self.visible_member_symbol(symbol, key)? {
            let mut substitution = self.generic_substitution(module, symbol, arguments)?;
            if self.reduce_symbol_availability(module, member, &substitution)? != Decision::Yes {
                return Ok(None);
            }
            if let Some(protocol) = protocol
                && !self.symbol_matches_protocol(symbol, protocol, &mut substitution)?
            {
                return Ok(None);
            }
            let variable = self.member_type_variable(module, member);
            if substitution.is_empty() {
                return Ok(Some(MemberCandidate {
                    symbol: member,
                    ty: TypeTerm::Variable(variable),
                    instance: None,
                }));
            }
            let term = TypeTerm::Variable(variable);
            let Some(term) = term.substitute(module, &substitution, self)? else {
                return Ok(None);
            };
            let instance = Some(GenericInstance {
                symbol,
                arguments: arguments.to_vec().into(),
            });

            return Ok(Some(MemberCandidate {
                symbol: member,
                ty: term,
                instance,
            }));
        }

        let Some((extension, member, substitution)) =
            self.extension_member(module, symbol, arguments, key, protocol)?
        else {
            return Ok(None);
        };
        if self.reduce_symbol_availability(module, member, &substitution)? != Decision::Yes {
            return Ok(None);
        }
        let variable = self.symbol_type_variable(module, member);
        if substitution.is_empty() {
            return Ok(Some(MemberCandidate {
                symbol: member,
                ty: TypeTerm::Variable(variable),
                instance: None,
            }));
        }
        let term = TypeTerm::Variable(variable);
        let Some(term) = term.substitute(module, &substitution, self)? else {
            return Ok(None);
        };
        let instance = self.substitution_instance(module, extension, &substitution)?;

        Ok(Some(MemberCandidate {
            symbol: member,
            ty: term,
            instance,
        }))
    }

    /// Return a member from a component module or checked dependency.
    pub(in crate::check) fn visible_member_symbol(
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
            let view = module.view();
            let symbols = Self::binding_member_symbols(&bindings, Some(&view), symbol, slot);

            return Ok(symbols);
        }

        let dependency = self.load_dependency(symbol.module_id)?;
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
        match slot {
            dir::MemberSlot::Key(key) => binding_lookup_symbols(
                owner.module_id,
                bindings.lookup_key_member(owner.local_id, key),
            ),
            role_slot => Self::binding_role_member_symbols(
                bindings,
                view,
                owner.module_id,
                owner.local_id,
                role_slot,
            ),
        }
    }

    /// Return role member symbols from one owner scope.
    fn binding_role_member_symbols(
        bindings: &dir::BindingTable<'_>,
        view: Option<&dir::View<'_>>,
        module: ModuleId,
        owner: dir::LocalSymbolId,
        role_slot: dir::MemberSlot,
    ) -> Vec<dir::GlobalSymbolId> {
        let Some(view) = view else {
            return Vec::new();
        };
        let Some(scope) = bindings.scope_for_owner(owner) else {
            return Vec::new();
        };
        let scope = bindings.get_scope(scope);
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

    /// Return an applicable extension member for one applied nominal receiver.
    pub(in crate::check) fn extension_member(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<
        Option<(
            dir::GlobalSymbolId,
            dir::GlobalSymbolId,
            GenericSubstitution,
        )>,
    > {
        let extensions = self.extension_candidates(module, target, key)?;

        // accept the first extension whose target pattern and constraints hold
        for extension in extensions {
            let Some(mut substitution) =
                self.extension_substitution(extension.symbol, extension.target, target, arguments)?
            else {
                continue;
            };
            if let Some(protocol) = protocol
                && !self.symbol_matches_protocol(extension.symbol, protocol, &mut substitution)?
            {
                continue;
            }

            if self.extension_where_clauses_hold(&extension.where_clauses, &substitution)? {
                return Ok(Some((extension.symbol, extension.member, substitution)));
            }
        }

        Ok(None)
    }

    /// Return extension candidates visible from one module.
    fn extension_candidates(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Vec<ExtensionCandidate>> {
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();

        self.collect_component_extension_candidates(module, key, &mut seen, &mut candidates)?;

        // add inherent extensions declared with the receiver type
        if target.module_id != module {
            self.collect_receiver_extension_candidates(
                module,
                target,
                key,
                &mut seen,
                &mut candidates,
            )?;
        }

        let imports = self
            .module(module)
            .resolved
            .imports
            .symbol_targets()
            .map(|(_, target)| target)
            .collect::<Vec<_>>();

        // add explicitly imported extension declarations
        for symbol in imports {
            self.add_extension_candidate(module, symbol, key, &mut seen, &mut candidates)?;
        }

        Ok(candidates)
    }

    /// Add component extension candidates declared in one module.
    fn collect_component_extension_candidates(
        &mut self,
        module: ModuleId,
        key: dir::StaticKey,
        seen: &mut IndexSet<dir::GlobalSymbolId>,
        candidates: &mut Vec<ExtensionCandidate>,
    ) -> CompilerResult<()> {
        let symbols = {
            let bindings = self.module(module).binding_table();
            let mut symbols = Vec::new();

            // collect extension symbols before creating variables
            for symbol in bindings.symbol_ids() {
                let entry = bindings.get_symbol(symbol);
                if entry.kind != dir::SymbolKind::Extension {
                    continue;
                };
                let symbol = symbol.into_global(module);
                if self.member_symbol(module, symbol, key).is_none() {
                    continue;
                }

                symbols.push(symbol);
            }

            symbols
        };

        for symbol in symbols {
            self.add_extension_candidate(module, symbol, key, seen, candidates)?;
        }

        Ok(())
    }

    /// Add receiver-module extension candidates.
    fn collect_receiver_extension_candidates(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: dir::StaticKey,
        seen: &mut IndexSet<dir::GlobalSymbolId>,
        candidates: &mut Vec<ExtensionCandidate>,
    ) -> CompilerResult<()> {
        if self.modules.contains_key(&target.module_id) {
            self.collect_component_extension_candidates(target.module_id, key, seen, candidates)?;

            return Ok(());
        }

        let symbols = {
            let dependency = self.load_dependency(target.module_id)?;
            let mut symbols = Vec::new();

            // collect inherent dependency extensions for this receiver
            for extension_id in dependency.extensions.target_extensions(target) {
                let extension = dependency.extensions.get_extension(extension_id);
                if extension.is_inherent() {
                    symbols.push(extension.symbol);
                }
            }

            symbols
        };

        for symbol in symbols {
            self.add_extension_candidate(module, symbol, key, seen, candidates)?;
        }

        Ok(())
    }

    /// Add one extension candidate by declaration symbol.
    fn add_extension_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
        seen: &mut IndexSet<dir::GlobalSymbolId>,
        candidates: &mut Vec<ExtensionCandidate>,
    ) -> CompilerResult<()> {
        let candidate = if self.modules.contains_key(&symbol.module_id) {
            self.component_extension_candidate(symbol, key)?
        } else {
            self.dependency_extension_candidate(module, symbol, key)?
        };
        let Some(candidate) = candidate else {
            return Ok(());
        };
        if seen.insert(candidate.symbol) {
            candidates.push(candidate);
        }

        Ok(())
    }

    /// Return one component extension candidate by declaration symbol.
    fn component_extension_candidate(
        &mut self,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<ExtensionCandidate>> {
        let Some(extension) = self.component_extension_declaration(symbol) else {
            return Ok(None);
        };
        let Some(member) = self.member_symbol(symbol.module_id, symbol, key) else {
            return Ok(None);
        };
        let target = self.intern_local_node_type_variable(symbol.module_id, extension.target_type);
        let where_clauses = extension
            .where_clauses
            .into_iter()
            .map(|where_clause| {
                let source = where_clause.into_global_any(symbol.module_id);
                let where_clause = self
                    .module(symbol.module_id)
                    .view()
                    .get(where_clause)
                    .clone();
                let left =
                    self.intern_local_node_type_variable(symbol.module_id, where_clause.left);
                let right =
                    self.intern_local_node_type_variable(symbol.module_id, where_clause.right);

                ExtensionWhereClause {
                    source,
                    left,
                    right,
                }
            })
            .collect();

        Ok(Some(ExtensionCandidate {
            symbol,
            target,
            where_clauses,
            member,
        }))
    }

    /// Return one component extension declaration by symbol.
    fn component_extension_declaration(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::ExtensionDeclaration> {
        let source = self.local_symbol_source_node(symbol)?;
        if source.ty != dir::NodeType::Declaration {
            return None;
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(source.id);

        match self.module(symbol.module_id).view().get(declaration) {
            dir::Declaration::Extension(extension) => Some(extension.clone()),
            _ => None,
        }
    }

    /// Return one checked dependency extension candidate by declaration symbol.
    fn dependency_extension_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<ExtensionCandidate>> {
        self.load_dependency(symbol.module_id)?;
        let dependency = self.dependency(symbol.module_id);
        let Some(extension_id) = dependency.extensions.symbol_extension_id(symbol) else {
            return Ok(None);
        };
        let extension = dependency.extensions.get_extension(extension_id).clone();
        let declaration = Self::dependency_extension_declaration(dependency, symbol);
        let Some(member) = Self::dependency_extension_member(dependency, symbol, key) else {
            return Ok(None);
        };
        let target =
            self.import_dependency_type_variable(module, symbol.module_id, extension.target_type);
        let mut where_clauses = Vec::with_capacity(declaration.where_clauses.len());

        for where_clause in declaration.where_clauses {
            let source = where_clause.into_global_any(symbol.module_id);
            let where_clause = self
                .dependency(symbol.module_id)
                .view()
                .get(where_clause)
                .clone();
            let left = self.import_dependency_node_type_variable(
                module,
                symbol.module_id,
                where_clause.left.into_global_any(symbol.module_id),
            );
            let right = self.import_dependency_node_type_variable(
                module,
                symbol.module_id,
                where_clause.right.into_global_any(symbol.module_id),
            );

            where_clauses.push(ExtensionWhereClause {
                source,
                left,
                right,
            });
        }

        Ok(Some(ExtensionCandidate {
            symbol,
            target,
            where_clauses,
            member,
        }))
    }

    /// Return one dependency extension declaration by symbol.
    fn dependency_extension_declaration(
        dependency: &CheckDependencyState,
        symbol: dir::GlobalSymbolId,
    ) -> dir::ExtensionDeclaration {
        let symbol_entry = dependency.bindings.get_symbol(symbol.local_id);
        let Some(source) = symbol_entry.declaration else {
            panic!("dependency extension {symbol:?} has no declaration");
        };
        if source.local_id.ty != dir::NodeType::Declaration {
            panic!("dependency extension {symbol:?} source is not a declaration");
        }
        let declaration = dir::LocalNodeId::<dir::Declaration>::new(source.local_id.id);

        match dependency.view().get(declaration) {
            dir::Declaration::Extension(extension) => extension.clone(),
            _ => panic!("dependency extension {symbol:?} source is not an extension"),
        }
    }

    /// Return one member declared by a dependency extension.
    fn dependency_extension_member(
        dependency: &CheckDependencyState,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> Option<dir::GlobalSymbolId> {
        let view = dependency.view();
        let members = Self::binding_member_symbols(
            &dependency.bindings,
            Some(&view),
            symbol,
            dir::MemberSlot::Key(key),
        );

        members.first().copied()
    }

    /// Match an extension target pattern against an applied receiver.
    fn extension_substitution(
        &mut self,
        extension: dir::GlobalSymbolId,
        target_variable: VariableId,
        receiver: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<GenericSubstitution>> {
        let Some(pattern) = self.solved_type_term(target_variable)? else {
            return Ok(None);
        };
        let actual = TypeTerm::Reference {
            origin: Origin::Symbol(receiver),
            symbol: receiver,
            arguments: arguments.to_vec().into(),
        };
        let mut substitution = GenericSubstitution::empty();
        let is_match = self.match_type_pattern(
            target_variable.module,
            extension,
            &pattern,
            &actual,
            &mut substitution,
        )?;

        Ok(is_match.then_some(substitution))
    }

    /// Return whether substituted extension where clauses hold.
    fn extension_where_clauses_hold(
        &mut self,
        where_clauses: &[ExtensionWhereClause],
        substitution: &GenericSubstitution,
    ) -> CompilerResult<bool> {
        for where_clause in where_clauses {
            let Some(left) = self.solved_type_term(where_clause.left)? else {
                return Ok(false);
            };
            let Some(right) = self.solved_type_term(where_clause.right)? else {
                return Ok(false);
            };
            let Some(left) = left.substitute(where_clause.left.module, substitution, self)? else {
                return Ok(false);
            };
            let Some(right) = right.substitute(where_clause.right.module, substitution, self)?
            else {
                return Ok(false);
            };

            let decision =
                self.decide_type_term_relation(TypeRelation::Satisfies, &left, &right)?;
            let decision = if decision == Decision::Undecidable {
                self.reduce_structural_satisfies(
                    Origin::Node(where_clause.source),
                    where_clause.source.module_id,
                    &left,
                    &right,
                )?
            } else {
                decision
            };
            if decision != Decision::Yes {
                return Ok(false);
            };
        }

        Ok(true)
    }
}

/// Convert one local binding lookup into global symbols.
fn binding_lookup_symbols(module: ModuleId, lookup: dir::SymbolLookup) -> Vec<dir::GlobalSymbolId> {
    match lookup {
        dir::SymbolLookup::Found(symbol) => vec![symbol.into_global(module)],
        dir::SymbolLookup::Ambiguous(symbols) => symbols
            .into_iter()
            .map(|symbol| symbol.into_global(module))
            .collect(),
        dir::SymbolLookup::Missing => Vec::new(),
    }
}
