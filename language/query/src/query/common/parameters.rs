use std::collections::HashMap;

use destack_dir::{
    Declaration, GlobalSymbolId, LocalNodeId, Member, NodeTree, NodeType, Parameter,
};

use crate::common::{doc_strings_for_node_or_enclosing, parse_param_docs};
use destack_workspace::{ModuleAst, Session};

/// Parameter names and documentation collected from a declaration.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParameterData {
    /// The parameter display names in declared order.
    pub names: Vec<String>,
    /// Documentation keyed by parameter name.
    pub docs: HashMap<String, String>,
}

/// Format a parameter into a display name.
pub fn parameter_display_name(session: &Session, parameter: &Parameter) -> String {
    // choose the display name based on the parameter shape
    match parameter {
        Parameter::Named { name, .. } => session.strings.get(*name).to_string(),
        Parameter::VariadicNamed { name, .. } => {
            let name_str = session.strings.get(*name).to_string();
            format!("...{name_str}")
        }
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => "<pattern>".to_string(),
    }
}

/// Collect parameter display names from dynamic parameter nodes.
pub(crate) fn dynamic_parameter_display_names(
    session: &Session,
    tree: &NodeTree,
    dynamic_parameters: &[LocalNodeId<Parameter>],
) -> Vec<String> {
    // collect parameter display names in declared order
    dynamic_parameters
        .iter()
        .map(|param_id| {
            let param = tree.get::<Parameter>(*param_id);
            parameter_display_name(session, param)
        })
        .collect()
}

/// Get dynamic parameter names for a function symbol.
pub fn dynamic_parameter_names(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<Vec<String>> {
    // collect parameter data for the symbol declaration
    let data = parameter_data_for_symbol(session, symbol_id)?;

    // return none for empty parameter lists
    if data.names.is_empty() {
        return None;
    }

    Some(data.names)
}

/// Collect parameter names and docs for a function or method symbol.
pub(crate) fn parameter_data_for_symbol(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<ParameterData> {
    // read the target module and build a query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = crate::query_context(session, &module)?;

    // resolve the symbol and its primary declaration
    let global_node_id = {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration?
    };

    // resolve the source text for doc parsing
    let source_file = session.files.get(ctx.file_id);
    let source = source_file.text();

    // resolve parameter data based on the declaration node type
    let dir_tree = ctx.tree();
    match global_node_id.local_id.ty {
        // collect parameters from function declarations
        NodeType::Declaration => {
            let declaration_id = global_node_id.local_id.try_into_typed().ok()?;
            let declaration = dir_tree.get::<Declaration>(declaration_id);
            let Declaration::Function { signature, .. } = declaration else {
                return None;
            };

            let ast_node_id = dir_tree.get_source(declaration_id.id);
            let docs = parameter_doc_map(ctx.ast, source, ast_node_id);
            let names =
                dynamic_parameter_display_names(session, dir_tree, &signature.dynamic_parameters);

            Some(ParameterData { names, docs })
        }

        // collect parameters from method members
        NodeType::Member => {
            let member_id = global_node_id.local_id.try_into_typed().ok()?;
            let member = dir_tree.get::<Member>(member_id);
            let Member::Method { signature, .. } = member else {
                return None;
            };

            let ast_node_id = dir_tree.get_source(member_id.id);
            let docs = parameter_doc_map(ctx.ast, source, ast_node_id);
            let names =
                dynamic_parameter_display_names(session, dir_tree, &signature.dynamic_parameters);

            Some(ParameterData { names, docs })
        }

        _ => None,
    }
}

/// Collect @param documentation from a declaration's doc comments.
pub(crate) fn parameter_doc_map(
    ast: &ModuleAst,
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
