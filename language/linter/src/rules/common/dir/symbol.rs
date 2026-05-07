use destack_artifact::WellKnownSymbols;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::{ArtifactCache, ProfileId};

/// Symbol type id tied to the module that owns its type table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolValueTypeId {
    /// The module id that owns the type table.
    pub module_id: ModuleId,
    /// The local type id in that module type table.
    pub type_id: dir::LocalTypeId,
}

/// Return candidate symbols for an expression usage site.
pub fn expression_candidate_symbols(
    local_module_id: ModuleId,
    local_types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Vec<dir::GlobalSymbolId> {
    let mut symbols = Vec::new();

    // include lexical reference targets
    let global_expression_id = expression_id.into_global_any(local_module_id);
    if let Some(resolution) = local_types.symbol_resolution(global_expression_id) {
        for symbol_id in symbol_resolution_target_symbols(resolution) {
            push_unique_symbol(&mut symbols, symbol_id);
        }
    }

    // include dispatch target symbols
    if let Some(resolution) = local_types.resolution(global_expression_id) {
        for symbol_id in resolution_target_symbols(resolution) {
            push_unique_symbol(&mut symbols, symbol_id);
        }
    }

    symbols
}

/// Map attributes found on expression candidate symbols.
#[allow(clippy::too_many_arguments)]
pub fn expression_attribute_map<T>(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    mut map: impl FnMut(&dir::SymbolAttributes) -> Option<T>,
) -> Option<T> {
    let symbols = expression_candidate_symbols(local_module_id, local_types, expression_id);

    for symbol_id in symbols {
        let Some(attributes) = symbol_attributes_for(
            artifacts,
            profile_id,
            local_module_id,
            local_symbols,
            symbol_id,
        ) else {
            continue;
        };

        if let Some(value) = map(&attributes) {
            return Some(value);
        }
    }

    None
}

/// Return true when an expression candidate symbol matches one attribute predicate.
#[allow(clippy::too_many_arguments)]
pub fn expression_has_attribute(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    local_types: &dir::TypeTable,
    expression_id: dir::LocalNodeId<dir::Expression>,
    mut predicate: impl FnMut(&dir::SymbolAttributes) -> bool,
) -> bool {
    expression_attribute_map(
        artifacts,
        profile_id,
        local_module_id,
        local_symbols,
        local_types,
        expression_id,
        |attributes| predicate(attributes).then_some(()),
    )
    .is_some()
}

/// Collect target symbols from a resolution.
pub fn resolution_target_symbols(resolution: &dir::Resolution) -> Vec<dir::GlobalSymbolId> {
    match resolution {
        dir::Resolution::Symbol(symbol) => symbol_resolution_target_symbols(symbol),
        dir::Resolution::Dependency(dependency) => dependency_resolution_target_symbols(dependency),
        dir::Resolution::Control(_) => Vec::new(),
        dir::Resolution::Dispatch(dispatch) => dispatch_resolution_target_symbols(dispatch),
    }
}

/// Collect target symbols from a symbol resolution.
pub fn symbol_resolution_target_symbols(
    resolution: &dir::SymbolResolution,
) -> Vec<dir::GlobalSymbolId> {
    match resolution {
        dir::SymbolResolution::Target(symbol) => vec![*symbol],
        dir::SymbolResolution::Candidates(symbols) => symbols.clone(),
    }
}

/// Collect target symbols from a dependency resolution.
pub fn dependency_resolution_target_symbols(
    resolution: &dir::DependencyResolution,
) -> Vec<dir::GlobalSymbolId> {
    match resolution {
        dir::DependencyResolution::Binding(symbol) => vec![*symbol],
        dir::DependencyResolution::Module(_) => Vec::new(),
    }
}

/// Collect target symbols from a dispatch resolution.
pub fn dispatch_resolution_target_symbols(
    resolution: &dir::DispatchResolution,
) -> Vec<dir::GlobalSymbolId> {
    match resolution {
        dir::DispatchResolution::Static { target, .. } => vec![target.symbol],
        dir::DispatchResolution::Dynamic { targets, .. } => {
            targets.iter().map(|target| target.symbol).collect()
        }
        dir::DispatchResolution::Builtin { .. } => Vec::new(),
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
    let Some(group) = well_known_symbols.get_pair(symbol) else {
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
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::Symbol> {
    if symbol_id.module_id == local_module_id {
        return Some(local_symbols.get_symbol(symbol_id.local_id).clone());
    }

    let dir = artifacts.dir_declared(symbol_id.module_id, profile_id)?;
    Some(dir.symbols.get_symbol(symbol_id.local_id).clone())
}

/// Read attributes for a symbol.
pub fn symbol_attributes_for(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::SymbolAttributes> {
    let symbol = symbol_for(
        artifacts,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    Some(symbol.attributes)
}

/// Read the declaration id for a symbol.
pub fn symbol_declaration_for(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::GlobalNodeIdAny> {
    let symbol = symbol_for(
        artifacts,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    symbol.declaration
}

/// Resolve one local initializer expression for a symbol when available.
pub fn symbol_initializer_expression(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_symbols: &dir::SymbolTable,
    tree: &dir::Tree,
    symbol_id: dir::GlobalSymbolId,
) -> Option<dir::LocalNodeId<dir::Expression>> {
    // resolve the declaration for this symbol
    let declaration_id = symbol_declaration_for(
        artifacts,
        profile_id,
        local_module_id,
        local_symbols,
        symbol_id,
    )?;
    if declaration_id.module_id != local_module_id {
        return None;
    }

    // resolve the declaration initializer in the local tree
    declaration_initializer_expression(tree, declaration_id, symbol_id.local_id)
}

/// Resolve one initializer expression from a symbol declaration node.
pub fn declaration_initializer_expression(
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

/// Read the value type id for a symbol.
pub fn symbol_value_type_id_for(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_types: &dir::TypeTable,
    symbol_id: dir::GlobalSymbolId,
) -> Option<SymbolValueTypeId> {
    if symbol_id.module_id == local_module_id {
        let type_id = local_types.get_value_type_id(symbol_id)?;
        return Some(SymbolValueTypeId {
            module_id: local_module_id,
            type_id,
        });
    }

    let dir = artifacts.dir_checked(symbol_id.module_id, profile_id)?;
    let type_id = dir.types.get_value_type_id(symbol_id)?;
    Some(SymbolValueTypeId {
        module_id: symbol_id.module_id,
        type_id,
    })
}

/// Map one symbol value type from local or remote type tables.
pub fn symbol_value_type_map_for<T>(
    artifacts: &ArtifactCache,
    profile_id: ProfileId,
    local_module_id: ModuleId,
    local_types: &dir::TypeTable,
    symbol_id: dir::GlobalSymbolId,
    map: impl FnOnce(&dir::TypeTable, dir::LocalTypeId) -> T,
) -> Option<T> {
    let value_type_id = symbol_value_type_id_for(
        artifacts,
        profile_id,
        local_module_id,
        local_types,
        symbol_id,
    )?;

    if value_type_id.module_id == local_module_id {
        return Some(map(local_types, value_type_id.type_id));
    }

    let dir = artifacts.dir_checked(value_type_id.module_id, profile_id)?;
    Some(map(&dir.types, value_type_id.type_id))
}
