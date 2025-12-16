use destack_dir::Expression;
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
            dynamic_arguments, ..
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

        // add parameter hints for each argument
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            // get the span of the argument
            let arg_ast_id = dir_tree.get_source(argument_id.id);
            let arg_span = module_guard.ast.tree.source_map.get(arg_ast_id);

            // add a parameter hint at the start of the argument
            // use generic parameter names (arg0, arg1, etc.) nocheckin #Suspicious
            hints.push(InlayHint::parameter_hint(
                arg_span.start,
                format!("arg{index}"),
            ));
        }
    }

    // sort hints by position
    hints.sort_by_key(|h| h.position);

    hints
}
