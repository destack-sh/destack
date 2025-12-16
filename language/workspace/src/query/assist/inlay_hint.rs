use destack_dir::{Declaration, Expression, GlobalSymbolId, NodeType, Parameter};
use destack_source::{FileId, Span};

use crate::Session;
use crate::query::common::get_module_by_file_id;

/// Kind of inlay hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlayHintKind {
    /// Type annotation hint (e.g., `: string`).
    Type,
    /// Parameter name hint (e.g., `name:`).
    Parameter,
}

/// An inlay hint (virtual text shown inline).
#[derive(Debug, Clone)]
pub struct InlayHint {
    /// Position where the hint should be displayed.
    pub position: u32,
    /// The hint text.
    pub label: String,
    /// The kind of hint.
    pub kind: InlayHintKind,
    /// Whether there should be padding before the hint.
    pub padding_left: bool,
    /// Whether there should be padding after the hint.
    pub padding_right: bool,
}

impl InlayHint {
    /// Create a type hint.
    pub fn type_hint(position: u32, type_name: impl Into<String>) -> Self {
        Self {
            position,
            label: format!(": {}", type_name.into()),
            kind: InlayHintKind::Type,
            padding_left: false,
            padding_right: false,
        }
    }

    /// Create a parameter hint.
    pub fn parameter_hint(position: u32, param_name: impl Into<String>) -> Self {
        Self {
            position,
            label: format!("{}:", param_name.into()),
            kind: InlayHintKind::Parameter,
            padding_left: false,
            padding_right: true,
        }
    }
}

/// Get inlay hints for a range in a file.
pub fn inlay_hints(session: &Session, file: FileId, range: Span) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    // 1. get the module for this file
    let Some(module) = get_module_by_file_id(session, file) else {
        return hints;
    };

    let module_guard = module.read();
    let dir_tree = module_guard.dir.tree.read();

    // 2. iterate through all call expressions
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        // check if this is a call expression
        let Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        else {
            continue;
        };

        // skip if no arguments
        if dynamic_arguments.is_empty() {
            continue;
        }

        // get the span of this call expression
        let ast_node_id = dir_tree.get_source(expression_id.id);
        let call_span = module_guard.ast.tree.source_map.get(ast_node_id);

        // skip if outside the requested range
        if call_span.end < range.start || call_span.start > range.end {
            continue;
        }

        // get the target symbol from the function being called
        let left_expr = dir_tree.get::<Expression>(*left);
        let target_symbol = match left_expr {
            Expression::GlobalReference { target_symbol, .. }
            | Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        };

        // get actual parameter names for this function
        let param_names = get_parameter_names(session, target_symbol, dynamic_arguments.len());

        // add parameter hints for each argument
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            // get the span of the argument
            let arg_ast_id = dir_tree.get_source(argument_id.id);
            let arg_span = module_guard.ast.tree.source_map.get(arg_ast_id);

            // add a parameter hint at the start of the argument
            let param_name = param_names
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("arg{index}"));

            hints.push(InlayHint::parameter_hint(arg_span.start, param_name));
        }
    }

    // sort hints by position
    hints.sort_by_key(|h| h.position);

    hints
}

/// Get parameter names for a function.
///
/// If the target symbol points to a function declaration, extracts actual parameter names.
/// Returns an empty vec if not available (caller will use generic names).
fn get_parameter_names(
    session: &Session,
    target_symbol: Option<GlobalSymbolId>,
    _argument_count: usize,
) -> Vec<String> {
    let Some(symbol_id) = target_symbol else {
        return Vec::new();
    };

    let target_module = session.modules.get(symbol_id.module_id);
    let target_guard = target_module.read();
    let symbols = target_guard.dir.symbols.read();
    let symbol_data = symbols.get_symbol(symbol_id.local_id);

    // check if this symbol has a primary declaration
    let Some(global_node_id) = symbol_data.primary_declaration else {
        return Vec::new();
    };

    // must be a declaration node
    if global_node_id.local_id.ty != NodeType::Declaration {
        return Vec::new();
    }

    let Some(declaration_id) = global_node_id.local_id.try_into_typed().ok() else {
        return Vec::new();
    };

    let dir_tree = target_guard.dir.tree.read();
    let declaration = dir_tree.get::<Declaration>(declaration_id);

    // if it's a function, get its parameters
    let Declaration::Function { signature, .. } = declaration else {
        return Vec::new();
    };

    signature
        .dynamic_parameters
        .iter()
        .map(|param_id| {
            let param = dir_tree.get::<Parameter>(*param_id);
            match param {
                Parameter::Named { name, .. } => {
                    target_guard.ast.strings.get(*name).to_string()
                }
                Parameter::Variadic { name, .. } => {
                    let name_str = target_guard.ast.strings.get(*name).to_string();
                    format!("...{name_str}")
                }
                Parameter::Pattern { .. } => "<pattern>".to_string(),
            }
        })
        .collect()
}
