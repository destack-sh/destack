use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_dir::{
    Declaration, GlobalSymbolId, LocalNodeId, LocalTypeId, Member, NodeType, Parameter, Tree,
};
use destack_workspace::{Repository, Revision};

use crate::core::{AstQueryContext, QueryContext, query_context};

use super::{doc_strings_for_node_or_enclosing, get_canonical_symbol, parse_param_docs};

/// Parameter names and documentation collected from a declaration.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParameterData {
    /// The parameter display names in declared order.
    pub names: Vec<String>,
    /// Documentation keyed by parameter name.
    pub docs: HashMap<String, String>,
}

/// One expected-parameter hint for argument completion ranking.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExpectedParameterHint {
    /// The parameter display name when available.
    pub name: Option<String>,
    /// The direct nominal type symbol when available.
    pub type_symbol: Option<GlobalSymbolId>,
    /// Related nominal type symbols reachable through the declared type.
    pub type_symbols: Vec<GlobalSymbolId>,
    /// Whether callable values are preferred.
    pub prefers_callable: bool,
    /// Whether constructable values are preferred.
    pub prefers_constructable: bool,
}

/// Format a parameter into a display name.
pub(crate) fn parameter_display_name(strings: &StringPool, parameter: &Parameter) -> String {
    // choose the display name based on the parameter shape
    match parameter {
        Parameter::Named { name, .. } => strings.get(*name).to_string(),
        Parameter::VariadicNamed { name, .. } => {
            let name_str = strings.get(*name).to_string();
            format!("...{name_str}")
        }
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => "<pattern>".to_string(),
        Parameter::Error { .. } => "<error>".to_string(),
    }
}

/// Collect parameter display names from parameter nodes.
pub(crate) fn parameter_display_names(
    strings: &StringPool,
    tree: &Tree,
    parameters: &[LocalNodeId<Parameter>],
) -> Vec<String> {
    // collect parameter display names in declared order
    parameters
        .iter()
        .map(|param_id| {
            let param = tree.get::<Parameter>(*param_id);
            parameter_display_name(strings, param)
        })
        .collect()
}

/// Get parameter names for a function symbol.
pub(crate) fn parameter_names_for_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<Vec<String>> {
    // collect parameter data for the symbol declaration
    let data = parameter_data_for_symbol(repository, revision, symbol_id)?;

    if data.names.is_empty() {
        return None;
    }

    Some(data.names)
}

/// Collect parameter names and docs for a function or method symbol.
pub(crate) fn parameter_data_for_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<ParameterData> {
    let ctx = query_context(repository, revision, symbol_id.module_id)?;
    parameter_data_for_symbol_with_context(repository, ctx, symbol_id)
}

/// Collect parameter names and docs from one ready query context.
fn parameter_data_for_symbol_with_context(
    repository: &Repository,
    ctx: QueryContext,
    symbol_id: GlobalSymbolId,
) -> Option<ParameterData> {
    // read the symbol declaration
    let global_node_id = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.declaration?
    };

    // resolve the source text for doc parsing
    let source_file = repository
        .file(ctx.revision(), ctx.file_id())
        .ok()
        .flatten()?;
    let source = source_file.text();

    // resolve parameter data based on the declaration node type
    let dir_tree = ctx.dir().tree();
    match global_node_id.local_id.ty {
        // collect parameters from function declarations
        NodeType::Declaration => {
            let declaration_id = global_node_id.local_id.try_into_typed().ok()?;
            let declaration = dir_tree.get::<Declaration>(declaration_id);
            let Declaration::Function(declaration) = declaration else {
                return None;
            };

            let ast_node_id = dir_tree.get_source(declaration_id.id);
            let docs = parameter_doc_map(ctx.ast(), source, ast_node_id);
            let names = parameter_display_names(
                ctx.dir().strings(),
                dir_tree,
                &declaration.signature.parameters,
            );

            Some(ParameterData { names, docs })
        }

        // collect parameters from method members
        NodeType::Member => {
            let member_id = global_node_id.local_id.try_into_typed().ok()?;
            let member = dir_tree.get::<Member>(member_id);
            let signature = member.signature()?;

            let ast_node_id = dir_tree.get_source(member_id.id);
            let docs = parameter_doc_map(ctx.ast(), source, ast_node_id);
            let names =
                parameter_display_names(ctx.dir().strings(), dir_tree, &signature.parameters);

            Some(ParameterData { names, docs })
        }

        _ => None,
    }
}

/// Resolve the expected-parameter hint for one active argument.
pub(crate) fn expected_parameter_hint_for_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
    parameter_index: usize,
) -> Option<ExpectedParameterHint> {
    // read the target module and build a query context
    let ctx = query_context(repository, revision, symbol_id.module_id)?;

    // read the symbol declaration
    let global_node_id = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.declaration?
    };

    // resolve the parameters for the declaration
    let dir_tree = ctx.dir().tree();
    let parameters = match global_node_id.local_id.ty {
        NodeType::Declaration => {
            let declaration_id = global_node_id.local_id.try_into_typed().ok()?;
            let declaration = dir_tree.get::<Declaration>(declaration_id);
            let Declaration::Function(declaration) = declaration else {
                return None;
            };

            declaration.signature.parameters.clone()
        }
        NodeType::Member => {
            let member_id = global_node_id.local_id.try_into_typed().ok()?;
            let member = dir_tree.get::<Member>(member_id);
            let signature = member.signature()?;

            signature.parameters.clone()
        }
        _ => return None,
    };

    // resolve the active parameter node
    let parameter_id = resolve_expected_parameter_id(dir_tree, &parameters, parameter_index)?;
    let parameter = dir_tree.get::<Parameter>(parameter_id);
    let name = Some(parameter_display_name(ctx.dir().strings(), parameter));

    // resolve the declared parameter type and classify its shape
    let global_parameter_id = parameter_id.into_global_any(ctx.module_id());
    let type_id = ctx
        .dir()
        .types()
        .get_declared_type_id(global_parameter_id)?;
    let type_symbol = ctx
        .dir()
        .types()
        .get_type(type_id)
        .symbol()
        .map(|symbol_id| get_canonical_symbol(repository, ctx.revision(), symbol_id));
    let type_symbols =
        collect_expected_type_symbols(repository, ctx.revision(), ctx.dir().types(), type_id);
    let (prefers_callable, prefers_constructable) =
        expected_value_shape(ctx.dir().types(), type_id);

    Some(ExpectedParameterHint {
        name,
        type_symbol,
        type_symbols,
        prefers_callable,
        prefers_constructable,
    })
}

/// Collect nominal type symbols that should contribute to expected-type ranking.
fn collect_expected_type_symbols(
    repository: &Repository,
    revision: Revision,
    types: &destack_dir::TypeTable,
    type_id: LocalTypeId,
) -> Vec<GlobalSymbolId> {
    let mut symbols = Vec::new();
    let mut seen_types = HashSet::new();
    let mut seen_symbols = HashSet::new();

    collect_expected_type_symbols_inner(
        repository,
        revision,
        types,
        type_id,
        &mut seen_types,
        &mut seen_symbols,
        &mut symbols,
    );

    symbols
}

/// Collect nominal symbols from one declared parameter type.
fn collect_expected_type_symbols_inner(
    repository: &Repository,
    revision: Revision,
    types: &destack_dir::TypeTable,
    type_id: LocalTypeId,
    seen_types: &mut HashSet<LocalTypeId>,
    seen_symbols: &mut HashSet<GlobalSymbolId>,
    symbols: &mut Vec<GlobalSymbolId>,
) {
    let type_id = types.unwrap_value_type_id(type_id);
    if !seen_types.insert(type_id) {
        return;
    }

    let ty = types.get_type(type_id);

    // direct nominal references
    if let destack_dir::Type::Reference(reference) = ty {
        let symbol = reference.symbol;
        let canonical_symbol = get_canonical_symbol(repository, revision, symbol);
        if seen_symbols.insert(canonical_symbol) {
            symbols.push(canonical_symbol);
        }

        if let Some(target_type_id) = types.get_alias_target_type_id(symbol) {
            collect_expected_type_symbols_inner(
                repository,
                revision,
                types,
                target_type_id,
                seen_types,
                seen_symbols,
                symbols,
            );
        }

        return;
    }

    // nominal combinations
    match ty {
        destack_dir::Type::Union(union) => {
            for &element_id in &union.elements {
                collect_expected_type_symbols_inner(
                    repository,
                    revision,
                    types,
                    element_id,
                    seen_types,
                    seen_symbols,
                    symbols,
                );
            }
        }
        destack_dir::Type::Intersection(intersection) => {
            for &element_id in &intersection.elements {
                collect_expected_type_symbols_inner(
                    repository,
                    revision,
                    types,
                    element_id,
                    seen_types,
                    seen_symbols,
                    symbols,
                );
            }
        }
        destack_dir::Type::Value(value) => {
            collect_expected_type_symbols_inner(
                repository,
                revision,
                types,
                value.value,
                seen_types,
                seen_symbols,
                symbols,
            );
        }
        _ => {}
    }
}

/// Resolve the parameter node that should guide one argument index.
fn resolve_expected_parameter_id(
    dir_tree: &Tree,
    parameters: &[LocalNodeId<Parameter>],
    parameter_index: usize,
) -> Option<LocalNodeId<Parameter>> {
    if let Some(parameter_id) = parameters.get(parameter_index) {
        return Some(*parameter_id);
    }

    let last_parameter_id = *parameters.last()?;
    let last_parameter = dir_tree.get::<Parameter>(last_parameter_id);
    match last_parameter {
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => {
            Some(last_parameter_id)
        }
        _ => None,
    }
}

/// Classify the expected value shape for one parameter type.
fn expected_value_shape(types: &destack_dir::TypeTable, type_id: LocalTypeId) -> (bool, bool) {
    let ty = types.get_type(type_id);

    match ty {
        destack_dir::Type::Function(_) => (true, false),
        destack_dir::Type::Object(object) => (
            !object.call_signatures.is_empty(),
            !object.construct_signatures.is_empty(),
        ),
        _ => (false, false),
    }
}

/// Collect @param documentation from a declaration's doc comments.
pub(crate) fn parameter_doc_map(
    ast: AstQueryContext<'_>,
    source: &str,
    ast_node_id: u32,
) -> HashMap<String, String> {
    // initialize the parameter doc map
    let mut param_docs = HashMap::new();

    // gather docs on the node or enclosing nodes
    let doc_strings = doc_strings_for_node_or_enclosing(ast, source, ast_node_id);

    // parse @param tags from collected docs
    for doc_text in doc_strings {
        param_docs.extend(parse_param_docs(&doc_text));
    }

    // return the parsed parameter docs
    param_docs
}
