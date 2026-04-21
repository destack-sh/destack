use std::collections::HashSet;

use destack_dir as dir;
use destack_source::{ModuleId, Span};

use crate::LintModuleDirContext;

/// Resolve one direct binding name and symbol from a simple pattern binding.
pub fn pattern_binding_name_and_symbol(
    tree: &dir::NodeTree,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
) -> Option<(dir::StringId, dir::LocalSymbolId)> {
    let pattern = tree.get(pattern_id);
    let dir::Pattern::Binding { name, symbol, .. } = pattern else {
        return None;
    };

    Some((*name, *symbol))
}

/// Resolve one binding name and symbol pair from a parameter.
pub fn parameter_binding_name_and_symbol(
    tree: &dir::NodeTree,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
) -> Option<(dir::StringId, dir::LocalSymbolId)> {
    let parameter = tree.get(parameter_id);
    match parameter {
        dir::Parameter::Named { name, symbol, .. }
        | dir::Parameter::VariadicNamed { name, symbol, .. } => Some((*name, *symbol)),
        dir::Parameter::Pattern { pattern, .. }
        | dir::Parameter::VariadicPattern { pattern, .. } => {
            pattern_binding_name_and_symbol(tree, *pattern)
        }
        dir::Parameter::Error { .. } => None,
    }
}

/// Return true when one signature declares a value binding with the target name.
pub fn signature_declares_value_name(
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    signature: &dir::FunctionSignature,
    name: dir::StringId,
) -> bool {
    // check the `this` parameter first when present
    if signature.this_parameter.is_some_and(|parameter_id| {
        parameter_declares_value_name(tree, symbols, parameter_id, name)
    }) {
        return true;
    }

    // then check regular dynamic parameters
    signature
        .parameters
        .iter()
        .any(|parameter_id| parameter_declares_value_name(tree, symbols, *parameter_id, name))
}

/// Return true when one parameter declares a value binding with the target name.
fn parameter_declares_value_name(
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    name: dir::StringId,
) -> bool {
    let mut bindings = HashSet::new();
    collect_parameter_value_binding_symbols(tree, symbols, parameter_id, &mut bindings);

    bindings.into_iter().any(|symbol_id| {
        let symbol = symbols.get_symbol(symbol_id);
        symbol.name() == Some(name)
    })
}

/// Collect value-space binding symbols declared by one parameter.
pub fn collect_parameter_value_binding_symbols(
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    bindings: &mut HashSet<dir::LocalSymbolId>,
) {
    // resolve the parameter node
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
        dir::Parameter::Named { .. }
        | dir::Parameter::VariadicNamed { .. }
        | dir::Parameter::Error { .. } => {}
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
        dir::Pattern::Wildcard
        | dir::Pattern::Expression { .. }
        | dir::Pattern::TypeExpression { .. } => {}
        dir::Pattern::Assign { pattern, .. } => {
            collect_pattern_value_binding_symbols(tree, symbols, *pattern, bindings);
        }
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
        dir::PatternField::Named { pattern, .. } | dir::PatternField::Spread { pattern, .. } => {
            if let Some(pattern_id) = pattern {
                collect_pattern_value_binding_symbols(tree, symbols, *pattern_id, bindings);
            }
        }
        dir::PatternField::Computed { pattern, .. } => {
            collect_pattern_value_binding_symbols(tree, symbols, *pattern, bindings);
        }
        dir::PatternField::Positional { pattern, .. } => {
            collect_pattern_value_binding_symbols(tree, symbols, *pattern, bindings);
        }
        dir::PatternField::Elision => {}
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

/// Collect callable parameter value bindings as global symbols for one module.
///
/// This includes function declarations and member methods that have bodies.
pub fn collect_callable_parameter_value_binding_symbols(
    module_id: ModuleId,
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
) -> HashSet<dir::GlobalSymbolId> {
    let mut global_symbols = HashSet::new();

    // inspect function declarations with bodies
    for declaration_id in tree.iter_node_ids_of_type::<dir::Declaration>() {
        let declaration = tree.get(declaration_id);
        let dir::Declaration::Function(declaration) = declaration else {
            continue;
        };
        if declaration.body.is_none() {
            continue;
        }

        collect_signature_parameter_value_binding_symbols(
            module_id,
            tree,
            symbols,
            &declaration.signature,
            &mut global_symbols,
        );
    }

    // inspect member methods with bodies
    for member_id in tree.iter_node_ids_of_type::<dir::Member>() {
        let member = tree.get(member_id);
        let dir::Member::Method {
            signature,
            body: Some(_),
            ..
        } = member
        else {
            continue;
        };

        collect_signature_parameter_value_binding_symbols(
            module_id,
            tree,
            symbols,
            signature,
            &mut global_symbols,
        );
    }

    global_symbols
}

/// Collect value-space parameter bindings for one callable signature as globals.
fn collect_signature_parameter_value_binding_symbols(
    module_id: ModuleId,
    tree: &dir::NodeTree,
    symbols: &dir::SymbolTable,
    signature: &dir::FunctionSignature,
    global_symbols: &mut HashSet<dir::GlobalSymbolId>,
) {
    for parameter_id in &signature.parameters {
        let mut local_symbols = HashSet::new();
        collect_parameter_value_binding_symbols(tree, symbols, *parameter_id, &mut local_symbols);

        for local_symbol in local_symbols {
            global_symbols.insert(local_symbol.into_global(module_id));
        }
    }
}

/// Resolve a precise report span for one unused binding inside a parameter pattern.
pub fn parameter_binding_span(
    ctx: &LintModuleDirContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    local_node_id: dir::LocalNodeIdAny,
) -> Span {
    // prefer pattern node spans when available
    if local_node_id.ty == dir::NodeType::Pattern {
        return ctx.get_span(local_node_id.into_typed::<dir::Pattern>());
    }

    // then prefer pattern field spans
    if local_node_id.ty == dir::NodeType::PatternField {
        return ctx.get_span(local_node_id.into_typed::<dir::PatternField>());
    }

    // otherwise report at parameter span
    ctx.get_span(parameter_id)
}
