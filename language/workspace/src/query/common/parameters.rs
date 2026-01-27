use std::collections::HashMap;

use destack_ast::{AnnotationPosition, Doc};
use destack_dir::{
    Declaration, GlobalSymbolId, LocalNodeId, Member, NodeTree, NodeType, Parameter,
};

use crate::{ModuleAst, Session};

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
    match parameter {
        Parameter::Named { name, .. } => session.strings.get(*name).to_string(),
        Parameter::Variadic { name, .. } => {
            let name_str = session.strings.get(*name).to_string();
            format!("...{name_str}")
        }
        Parameter::Pattern { .. } => "<pattern>".to_string(),
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
    let ctx = session.query_context(&module)?;

    // resolve the symbol and its primary declaration
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let global_node_id = symbol.primary_declaration?;
    drop(symbols);

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
            let docs = parameter_doc_map(ctx.ast, ast_node_id);
            let names =
                dynamic_parameter_display_names(session, &dir_tree, &signature.dynamic_parameters);

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
            let docs = parameter_doc_map(ctx.ast, ast_node_id);
            let names =
                dynamic_parameter_display_names(session, &dir_tree, &signature.dynamic_parameters);

            Some(ParameterData { names, docs })
        }

        _ => None,
    }
}

/// Collect @param documentation from a declaration's doc comments.
pub(crate) fn parameter_doc_map(ast: &ModuleAst, ast_node_id: u32) -> HashMap<String, String> {
    let mut param_docs = HashMap::new();

    // get doc annotations attached to this AST node
    let docs = ast.tree.get_docs_for(ast_node_id);
    if docs.is_empty() {
        return param_docs;
    }

    // collect prefix docs
    for (doc_id, pos) in docs {
        if !matches!(
            pos,
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            continue;
        }

        let doc = ast.tree.get::<Doc>(doc_id);
        let doc_text = ast.strings.get(doc.string);

        // parse @param tags
        param_docs.extend(parse_param_docs(&doc_text));
    }

    param_docs
}

/// Parse @param tags from documentation text.
pub(crate) fn parse_param_docs(doc: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut current_param: Option<(String, String)> = None;

    // scan documentation lines for @param tags and continuations
    for line in doc.lines() {
        let line = line.trim();

        // start a new parameter entry when we see @param
        if let Some(rest) = line.strip_prefix("@param") {
            if let Some((name, desc)) = current_param.take() {
                result.insert(name, desc.trim().to_string());
            }

            let rest = rest.trim();

            // skip optional type annotations inside braces
            let rest = if let Some(after_brace) = rest.strip_prefix('{') {
                after_brace
                    .find('}')
                    .map(|i| after_brace[i + 1..].trim())
                    .unwrap_or(rest)
            } else {
                rest
            };

            // parse parameter name and initial description
            let mut parts = rest.splitn(2, |c: char| c.is_whitespace() || c == '-');
            if let Some(name) = parts.next() {
                let name = name.trim();
                if !name.is_empty() {
                    let desc = parts
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_start_matches('-')
                        .trim();
                    current_param = Some((name.to_string(), desc.to_string()));
                }
            }

            continue;
        }

        // append continuation lines to the current parameter
        if let Some((name, desc)) = current_param.as_mut() {
            if !line.is_empty() && !line.starts_with('@') {
                if !desc.is_empty() {
                    desc.push(' ');
                }
                desc.push_str(line);
            } else if line.starts_with('@') {
                result.insert(name.clone(), desc.trim().to_string());
                current_param = None;
            }
        }
    }

    // flush the last parameter entry
    if let Some((name, desc)) = current_param {
        result.insert(name, desc.trim().to_string());
    }

    result
}
