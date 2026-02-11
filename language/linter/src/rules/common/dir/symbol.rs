use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program, WellKnownSymbols};

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
    program: &Program,
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
            program,
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
    program: &Program,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
    mut map: impl FnMut(&dir::SymbolDecorators) -> Option<T>,
) -> Option<T> {
    let symbols = expression_candidate_symbols(
        program,
        profile_id,
        local_module_id,
        local_symbols,
        local_types,
        expression_id,
        expression,
    );

    for symbol_id in symbols {
        let Some(decorators) = symbol_decorators_for(
            program,
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
    program: &Program,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
    mut predicate: impl FnMut(&dir::SymbolDecorators) -> bool,
) -> bool {
    expression_decorator_map(
        program,
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
    program: &Program,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::Symbol> {
    if symbol_id.module_id == local_module_id {
        return Some(local_symbols.get_symbol(symbol_id.local_id).clone());
    }

    let module_ref = program.modules.get(symbol_id.module_id);
    let module = module_ref.read();
    let dir = module.dir_maybe(profile_id)?;
    let symbols = dir.symbols.read();
    Some(symbols.get_symbol(symbol_id.local_id).clone())
}

/// Resolve the canonical target symbol when available.
pub fn canonical_symbol_for(
    program: &Program,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::GlobalSymbolId> {
    let symbol = symbol_for(
        program,
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
    program: &Program,
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
        program,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )
    .is_some_and(|canonical_symbol_id| canonical_symbol_id == expected_symbol)
}

/// Return true when one symbol matches any candidate symbol directly or canonically.
pub fn symbol_matches_any_or_canonical(
    program: &Program,
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
        program,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )
    .is_some_and(|canonical_symbol_id| expected_symbols.contains(&canonical_symbol_id))
}

/// Read decorators for a symbol after canonicalization.
pub fn symbol_decorators_for(
    program: &Program,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::SymbolDecorators> {
    let symbol_id = canonical_symbol_for(
        program,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    let symbol = symbol_for(
        program,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    Some(symbol.decorators)
}

/// Read the primary declaration id for a symbol after canonicalization.
pub fn symbol_primary_declaration_for(
    program: &Program,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::GlobalNodeIdAny> {
    let symbol_id = canonical_symbol_for(
        program,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    let symbol = symbol_for(
        program,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    symbol.primary_declaration
}

/// Read the value type id for a symbol after canonicalization.
pub fn symbol_value_type_id_for(
    program: &Program,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<SymbolValueTypeId> {
    let symbol_id = canonical_symbol_for(
        program,
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

    let module_ref = program.modules.get(symbol_id.module_id);
    let module = module_ref.read();
    let dir = module.dir_maybe(profile_id)?;
    let types = dir.types.read();
    let type_id = types.get_value_type_id(symbol_id)?;
    Some(SymbolValueTypeId {
        module_id: symbol_id.module_id,
        type_id,
    })
}

/// Map one symbol value type from local or remote type tables.
pub fn symbol_value_type_map_for<T>(
    program: &Program,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    symbol_id: dir::GlobalSymbolId,
    map: impl FnOnce(&dir::TypeTable, dir::LocalTypeId) -> T,
) -> Option<T> {
    let symbol_type_id = symbol_value_type_id_for(
        program,
        profile_id,
        local_module_id,
        local_symbols,
        local_types,
        symbol_id,
    )?;

    if symbol_type_id.module_id == local_module_id {
        return Some(map(local_types, symbol_type_id.type_id));
    }

    let module_ref = program.modules.get(symbol_type_id.module_id);
    let module = module_ref.read();
    let dir = module.dir_maybe(profile_id)?;
    let types = dir.types.read();
    Some(map(&types, symbol_type_id.type_id))
}
