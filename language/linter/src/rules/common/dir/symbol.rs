use destack_artifact::WellKnownSymbols;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Repository, Revision};

/// Symbol type id tied to the module that owns its type table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolValueTypeId {
    /// The module id that owns the type table.
    pub module_id: ModuleId,
    /// The local type id in that module type table.
    pub type_id: dir::LocalTypeId,
}

/// Return canonical candidate symbols for an expression usage site.
pub fn expression_candidate_symbols(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
) -> Vec<dir::GlobalSymbolId> {
    let mut symbols = Vec::new();

    // include direct expression target symbols
    if let Some(symbol_id) = expression.target_symbol() {
        push_unique_symbol(&mut symbols, symbol_id);
    }

    // include resolver candidates for dynamic/static lookups
    let global_expression_id = expression_id.into_global_any(local_module_id);
    if let Some(resolution_id) = local_types.get_resolution_for_node(global_expression_id) {
        let resolution = local_types.get_resolution(resolution_id);
        for symbol_id in resolution_target_symbols(resolution) {
            push_unique_symbol(&mut symbols, symbol_id);
        }
    }

    // canonicalize symbols and keep deterministic unique order
    let mut canonical_symbols = Vec::new();
    for symbol_id in symbols {
        let canonical_symbol_id = canonical_symbol_for(
            repository,
            revision,
            profile_id,
            local_module_id,
            local_symbols,
            symbol_id,
        )
        .unwrap_or(symbol_id);
        push_unique_symbol(&mut canonical_symbols, canonical_symbol_id);
    }

    canonical_symbols
}

/// Map decorators found on expression candidate symbols.
#[allow(clippy::too_many_arguments)]
pub fn expression_decorator_map<T>(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
    mut map: impl FnMut(&dir::SymbolDecorators) -> Option<T>,
) -> Option<T> {
    let symbols = expression_candidate_symbols(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        local_types,
        expression_id,
        expression,
    );

    for symbol_id in symbols {
        let Some(decorators) = symbol_decorators_for(
            repository,
            revision,
            profile_id,
            local_module_id,
            local_symbols,
            symbol_id,
        ) else {
            continue;
        };

        if let Some(value) = map(&decorators) {
            return Some(value);
        }
    }

    None
}

/// Return true when an expression candidate symbol matches one decorator predicate.
#[allow(clippy::too_many_arguments)]
pub fn expression_has_decorator(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
    mut predicate: impl FnMut(&dir::SymbolDecorators) -> bool,
) -> bool {
    expression_decorator_map(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        local_types,
        expression_id,
        expression,
        |decorators| predicate(decorators).then_some(()),
    )
    .is_some()
}

/// Collect target symbols from a resolution.
pub fn resolution_target_symbols(resolution: &dir::Resolution) -> Vec<dir::GlobalSymbolId> {
    match resolution {
        dir::Resolution::Static { candidate, .. } => vec![candidate.target_symbol],
        dir::Resolution::Dynamic { candidates, .. } => candidates
            .iter()
            .map(|candidate| candidate.target_symbol)
            .collect(),
        dir::Resolution::Unresolved { .. } | dir::Resolution::Builtin { .. } => Vec::new(),
    }
}

/// Insert a symbol if it is not already present.
fn push_unique_symbol(symbols: &mut Vec<dir::GlobalSymbolId>, symbol: dir::GlobalSymbolId) {
    if symbols.contains(&symbol) {
        return;
    }
    symbols.push(symbol);
}

/// Return all symbol candidates for one well known symbol id.
pub fn well_known_symbol_candidates(
    well_known_symbols: &WellKnownSymbols,
    symbol: dir::WellKnownSymbol,
) -> Vec<dir::GlobalSymbolId> {
    let Some(group) = well_known_symbols.get_group(symbol) else {
        return Vec::new();
    };

    let mut symbols = Vec::new();
    if let Some(type_symbol) = group.ty {
        push_unique_symbol(&mut symbols, type_symbol);
    }
    if let Some(value_symbol) = group.value {
        push_unique_symbol(&mut symbols, value_symbol);
    }

    symbols
}

/// Read one symbol entry from local or remote module tables.
pub fn symbol_for(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::Symbol> {
    if symbol_id.module_id == local_module_id {
        return Some(local_symbols.get_symbol(symbol_id.local_id).clone());
    }

    let dir = repository.dir_analyzed(revision, symbol_id.module_id, profile_id)?;
    Some(dir.symbols.get_symbol(symbol_id.local_id).clone())
}

/// Resolve the canonical target symbol when available.
pub fn canonical_symbol_for(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::GlobalSymbolId> {
    let symbol = symbol_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;

    Some(
        symbol
            .canonical_symbol
            .or(symbol.target_symbol)
            .unwrap_or(symbol_id),
    )
}

/// Return true when one symbol matches an expected symbol directly or canonically.
pub fn symbol_matches_or_canonical(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
    expected_symbol: dir::GlobalSymbolId,
) -> bool {
    if symbol_id == expected_symbol {
        return true;
    }

    canonical_symbol_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )
    .is_some_and(|canonical_symbol_id| canonical_symbol_id == expected_symbol)
}

/// Return true when one symbol matches any candidate symbol directly or canonically.
pub fn symbol_matches_any_or_canonical(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
    expected_symbols: &[dir::GlobalSymbolId],
) -> bool {
    if expected_symbols.contains(&symbol_id) {
        return true;
    }

    canonical_symbol_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )
    .is_some_and(|canonical_symbol_id| expected_symbols.contains(&canonical_symbol_id))
}

/// Read decorators for a symbol after canonicalization.
pub fn symbol_decorators_for(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::SymbolDecorators> {
    let symbol_id = canonical_symbol_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    let symbol = symbol_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    Some(symbol.decorators)
}

/// Read the primary declaration id for a symbol after canonicalization.
pub fn symbol_primary_declaration_for(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::GlobalNodeIdAny> {
    let symbol_id = canonical_symbol_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    let symbol = symbol_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    symbol.primary_declaration
}

/// Resolve one local initializer expression for a symbol when available.
pub fn symbol_initializer_expression(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    tree: &dir::Tree,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // resolve the primary declaration for this symbol
    let declaration_id = symbol_primary_declaration_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    if declaration_id.module_id != local_module_id {
        return None;
    }

    // resolve the declaration initializer in the local tree
    primary_declaration_initializer_expression(tree, declaration_id, symbol_id.local_id)
}

/// Resolve one initializer expression from a symbol primary declaration node.
pub fn primary_declaration_initializer_expression(
    tree: &dir::Tree,
    declaration_id: dir::GlobalNodeIdAny,
    symbol_id: dir::LocalSymbolId,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    match declaration_id.local_id.ty {
        // read direct declarator initializers
        dir::NodeType::Declarator => {
            let declarator = tree.get(declaration_id.into_local_typed::<dir::Declarator>());
            declarator.value
        }

        // resolve pattern and pattern-field declarations through their declarator
        dir::NodeType::Pattern | dir::NodeType::PatternField => {
            let declarator_id = enclosing_declarator(tree, declaration_id.local_id.id)?;
            let declarator = tree.get(declarator_id);
            declarator.value
        }

        // map property declarations to value or default initializers
        dir::NodeType::Property => {
            let property = tree.get(declaration_id.into_local_typed::<dir::Property>());
            match property {
                dir::Property::Field { value, .. } => Some(*value),
                dir::Property::Method { .. } => None,
                dir::Property::Spread { value, .. } => Some(*value),
                dir::Property::Error { .. } => None,
            }
        }

        // map member declarations to value-like initializers
        dir::NodeType::Member => {
            let member = tree.get(declaration_id.into_local_typed::<dir::Member>());
            match member {
                dir::Member::Field { default, .. } => *default,
                dir::Member::AssociatedConst { value, .. } => *value,
                dir::Member::Method { .. }
                | dir::Member::AssociatedType { .. }
                | dir::Member::Embed { .. }
                | dir::Member::StaticBlock { .. }
                | dir::Member::ComptimeBlock { .. }
                | dir::Member::Error { .. } => None,
            }
        }

        // map parameters to default value expressions
        dir::NodeType::Parameter => {
            let parameter = tree.get(declaration_id.into_local_typed::<dir::Parameter>());
            match parameter {
                dir::Parameter::Named { default, .. } | dir::Parameter::Pattern { default, .. } => {
                    *default
                }
                dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => {
                    None
                }
                dir::Parameter::Error { .. } => None,
            }
        }

        // handle let and using declarations that point at the root expression node
        dir::NodeType::Expression => {
            let expression = tree.get(declaration_id.into_local_typed::<dir::Expression>());
            match expression {
                dir::Expression::Let { declarators, .. }
                | dir::Expression::Using { declarators, .. } => declarators.iter().find_map(|id| {
                    let declarator = tree.get(*id);
                    let pattern = tree.get(declarator.pattern);
                    (pattern.symbol() == Some(symbol_id))
                        .then_some(declarator.value)
                        .flatten()
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Find the nearest declarator parent for one node id.
fn enclosing_declarator(
    tree: &dir::Tree,
    mut node_id: u32,
) -> Option<dir::LocalNodeId<dir::Declarator>> {
    loop {
        let parent = tree.get_parent(node_id)?;
        if parent.ty == dir::NodeType::Declarator {
            return Some(parent.into_typed());
        }

        node_id = parent.id;
    }
}

/// Read the value type id for a symbol after canonicalization.
pub fn symbol_value_type_id_for(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<SymbolValueTypeId> {
    let symbol_id = canonical_symbol_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;

    if symbol_id.module_id == local_module_id {
        let type_id = local_types.get_value_type_id(symbol_id)?;
        return Some(SymbolValueTypeId {
            module_id: local_module_id,
            type_id,
        });
    }

    let dir = repository.dir_analyzed(revision, symbol_id.module_id, profile_id)?;
    let type_id = dir.types.get_value_type_id(symbol_id)?;
    Some(SymbolValueTypeId {
        module_id: symbol_id.module_id,
        type_id,
    })
}

/// Map one symbol value type from local or remote type tables.
pub fn symbol_value_type_map_for<T>(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    symbol_id: dir::GlobalSymbolId,
    map: impl FnOnce(&dir::TypeTable, dir::LocalTypeId) -> T,
) -> Option<T> {
    let symbol_type_id = symbol_value_type_id_for(
        repository,
        revision,
        profile_id,
        local_module_id,
        local_symbols,
        local_types,
        symbol_id,
    )?;

    if symbol_type_id.module_id == local_module_id {
        return Some(map(local_types, symbol_type_id.type_id));
    }

    let dir = repository.dir_analyzed(revision, symbol_type_id.module_id, profile_id)?;
    Some(map(&dir.types, symbol_type_id.type_id))
}

/// Return true when one local symbol is merged with a class declaration in this module.
pub fn local_symbol_has_other_declarations(
    symbols: &dir::SymbolTable,
    symbol_id: dir::LocalSymbolId,
) -> bool {
    let symbol = symbols.get_symbol(symbol_id);

    if symbol
        .secondary_declarations
        .as_deref()
        .is_some_and(|declarations| !declarations.is_empty())
    {
        return true;
    }

    let Some(merge_group_id) = symbol.merge_group else {
        return false;
    };

    symbols
        .merge_group_symbols(merge_group_id)
        .iter()
        .any(|merged_symbol_id| *merged_symbol_id != symbol_id)
}

/// Return true when one local symbol is merged with a class declaration in this module.
pub fn local_symbol_has_class_merge(
    tree: &dir::Tree,
    symbols: &dir::SymbolTable,
    symbol_id: dir::LocalSymbolId,
) -> bool {
    let symbol = symbols.get_symbol(symbol_id);
    let Some(merge_group_id) = symbol.merge_group else {
        return false;
    };

    for merged_symbol_id in symbols.merge_group_symbols(merge_group_id) {
        if *merged_symbol_id == symbol_id {
            continue;
        }

        if local_symbol_has_class_declaration(tree, symbols, *merged_symbol_id) {
            return true;
        }
    }

    false
}

/// Return true when one local symbol declares a class in this module.
fn local_symbol_has_class_declaration(
    tree: &dir::Tree,
    symbols: &dir::SymbolTable,
    symbol_id: dir::LocalSymbolId,
) -> bool {
    let symbol = symbols.get_symbol(symbol_id);

    if local_node_is_class_declaration(tree, symbol.primary_declaration.map(|id| id.local_id)) {
        return true;
    }

    let Some(secondary_declarations) = symbol.secondary_declarations.as_deref() else {
        return false;
    };

    secondary_declarations
        .iter()
        .any(|declaration_id| local_node_is_class_declaration(tree, Some(declaration_id.local_id)))
}

/// Return true when one local node id points at a class declaration.
fn local_node_is_class_declaration(
    tree: &dir::Tree,
    declaration_id: Option<dir::LocalNodeIdAny>,
) -> bool {
    let Some(declaration_id) = declaration_id else {
        return false;
    };
    if declaration_id.ty != dir::NodeType::Declaration {
        return false;
    }

    matches!(
        tree.get(declaration_id.into_typed::<dir::Declaration>()),
        dir::Declaration::Class(_)
    )
}
