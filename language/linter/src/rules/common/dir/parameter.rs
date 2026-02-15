use std::collections::HashSet;

use destack_dir as dir;

/// Collect value-space binding symbols declared by one parameter.
pub fn collect_parameter_value_binding_symbols(
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    let parameter = tree.get(parameter_id);

    // collect the parameter root symbol when it lives in value space
    let parameter_symbol = parameter.symbol();
    collect_symbol_when_value_space(symbols, parameter_symbol, bindings);

    // collect nested pattern symbols for pattern parameters
    match parameter {
        dir::Parameter::Pattern { pattern, .. }
        | dir::Parameter::VariadicPattern { pattern, .. } => {
            collect_pattern_value_binding_symbols(tree, symbols, *pattern, bindings);
        }
        dir::Parameter::Named { .. } | dir::Parameter::VariadicNamed { .. } => {}
    }
}

/// Collect value-space binding symbols declared by a pattern subtree.
pub fn collect_pattern_value_binding_symbols(
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    let pattern = tree.get(pattern_id);

    // collect the pattern binding symbol when present
    if let Some(symbol_id) = pattern.symbol() {
        collect_symbol_when_value_space(symbols, symbol_id, bindings);
    }

    // recurse through child patterns
    match pattern {
        dir::Pattern::Wildcard | dir::Pattern::Expression { .. } => {}
        dir::Pattern::Must(inner)
        | dir::Pattern::ReferenceOf { right: inner, .. }
        | dir::Pattern::ValueOf { right: inner, .. } => {
            collect_pattern_value_binding_symbols(tree, symbols, *inner, bindings);
        }
        dir::Pattern::Tuple { fields }
        | dir::Pattern::TaggedTuple { fields, .. }
        | dir::Pattern::Array { fields }
        | dir::Pattern::Object { fields }
        | dir::Pattern::TaggedObject { fields, .. } => {
            for field_id in fields {
                collect_pattern_field_value_binding_symbols(tree, symbols, *field_id, bindings);
            }
        }
        dir::Pattern::Union { patterns } => {
            for inner_pattern_id in patterns {
                collect_pattern_value_binding_symbols(tree, symbols, *inner_pattern_id, bindings);
            }
        }
        dir::Pattern::Binding { pattern, .. } => {
            if let Some(inner_pattern_id) = pattern {
                collect_pattern_value_binding_symbols(tree, symbols, *inner_pattern_id, bindings);
            }
        }
    }
}

/// Collect value-space binding symbols declared by one pattern field.
pub fn collect_pattern_field_value_binding_symbols(
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    field_id: dir::LocalNodeId<dir::PatternField>,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    let field = tree.get(field_id);

    // collect the field binding symbol when present
    if let Some(symbol_id) = field.symbol() {
        collect_symbol_when_value_space(symbols, symbol_id, bindings);
    }

    // recurse into nested field patterns
    match field {
        dir::PatternField::Named { pattern, .. }
        | dir::PatternField::Computed { pattern, .. }
        | dir::PatternField::Spread { pattern, .. } => {
            if let Some(pattern_id) = pattern {
                collect_pattern_value_binding_symbols(tree, symbols, *pattern_id, bindings);
            }
        }
        dir::PatternField::Positional { pattern, .. } => {
            collect_pattern_value_binding_symbols(tree, symbols, *pattern, bindings);
        }
        dir::PatternField::Alias { .. } | dir::PatternField::Elision => {}
    }
}

/// Collect one symbol when it belongs to value space.
fn collect_symbol_when_value_space(
    symbols: &dir::SymbolTable,
    symbol_id: dir::LocalSymbolId,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    let symbol = symbols.get_symbol(symbol_id);
    if matches!(
        symbol.space,
        dir::SymbolSpace::Value | dir::SymbolSpace::TypeValue
    ) {
        bindings.insert(symbol_id);
    }
}
