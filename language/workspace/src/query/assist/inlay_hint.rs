use destack_dir::{Declarator, Expression, GlobalNodeIdAny, GlobalSymbolId, Pattern, Resolution};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::format::format_type_for_inlay_hint;
use crate::query::common::{dynamic_parameter_names, with_query_context_for_file};

/// Kind of inlay hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InlayHintKind {
    /// Type annotation hint (e.g., `: string`).
    Type,
    /// Parameter name hint (e.g., `name:`).
    Parameter,
}

/// An inlay hint (virtual text shown inline).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Request inlay hints for a range in a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlayHintsRequest {
    /// The document URI.
    pub uri: Uri,
    /// The start byte offset in the document.
    pub start: u32,
    /// The end byte offset in the document.
    pub end: u32,
}

/// Response payload for inlay hints queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlayHintsResponse {
    /// Inlay hints.
    pub hints: Vec<InlayHint>,
}

/// Get inlay hints for a range in a file.
pub fn inlay_hints(session: &Session, file: FileId, range: Span) -> Vec<InlayHint> {
    with_query_context_for_file(session, file, |ctx| {
        // resolve shared dir data for hint generation
        let dir_tree = ctx.tree();
        let types = ctx.types();

        // collect parameter and type hints
        let mut hints = Vec::new();

        // iterate through all call expressions for parameter hints
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
            let call_span = ctx.ast.tree.source_map.get(ast_node_id);

            // skip if outside the requested range
            if call_span.end < range.start || call_span.start > range.end {
                continue;
            }

            // resolve the target symbol for this call
            let target_symbol = call_target_symbol(&ctx, expression_id, *left);

            // get actual parameter names for this function
            let param_names = get_parameter_names(session, target_symbol, dynamic_arguments.len());

            // add parameter hints for each argument
            for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                // get the span of the argument
                let arg_ast_id = dir_tree.get_source(argument_id.id);
                let arg_span = ctx.ast.tree.source_map.get(arg_ast_id);

                // add a parameter hint at the start of the argument
                let param_name = param_names
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| format!("arg{index}"));

                hints.push(InlayHint::parameter_hint(arg_span.start, param_name));
            }
        }

        // iterate through all declarators for type hints
        for (_declarator_id, declarator) in dir_tree.iter_nodes_of_type::<Declarator>() {
            // skip if already has explicit type annotation
            if declarator.ty.is_some() {
                continue;
            }

            // get the pattern to find its span and symbol
            let pattern = dir_tree.get::<Pattern>(declarator.pattern);
            if let Pattern::Binding { symbol, .. } = pattern {
                // get the span of the binding name
                let ast_node_id = dir_tree.get_source(declarator.pattern.id);
                let pattern_span = ctx.ast.tree.source_map.get(ast_node_id);

                // skip if outside the requested range
                if pattern_span.end < range.start || pattern_span.start > range.end {
                    continue;
                }

                // try to get the value type for this symbol
                let global_symbol_id = GlobalSymbolId {
                    module_id: ctx.module_id,
                    local_id: *symbol,
                };

                if let Some(type_id) = types.get_value_type_id(global_symbol_id) {
                    let ty = types.get_type(type_id);

                    // format a widened display type for literal values
                    let type_str =
                        format_type_for_inlay_hint(ty, &types, &session.modules, &session.strings);

                    // add type hint after the pattern
                    hints.push(InlayHint::type_hint(pattern_span.end, type_str));
                }
            }
        }

        // sort hints by position, kind, and label
        hints.sort_by(|left, right| {
            let left_key = (
                left.position,
                hint_kind_rank(left.kind),
                left.label.as_str(),
            );
            let right_key = (
                right.position,
                hint_kind_rank(right.kind),
                right.label.as_str(),
            );
            left_key.cmp(&right_key)
        });

        // drop duplicate hints
        hints.dedup_by(|left, right| {
            left.position == right.position && left.kind == right.kind && left.label == right.label
        });

        hints
    })
    .unwrap_or_default()
}

/// Rank inlay hint kinds for stable sorting.
fn hint_kind_rank(kind: InlayHintKind) -> u8 {
    match kind {
        InlayHintKind::Type => 0,
        InlayHintKind::Parameter => 1,
    }
}

/// Resolve the call target symbol using recorded resolutions when available.
fn call_target_symbol(
    ctx: &crate::query::QueryContext<'_>,
    expression_id: destack_dir::LocalNodeId<Expression>,
    left: destack_dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // prefer recorded call resolution data when it exists
    let types = ctx.types();
    let node_id = GlobalNodeIdAny {
        module_id: ctx.module_id,
        local_id: expression_id.into(),
    };
    if let Some(resolution_id) = types.get_resolution_for_node(node_id) {
        let resolution = types.get_resolution(resolution_id);
        let candidates = match resolution {
            Resolution::Static { candidate, .. } => std::slice::from_ref(candidate),
            Resolution::Dynamic { candidates, .. } => candidates.as_slice(),
            Resolution::Unresolved { candidates, .. } => candidates.as_slice(),
            _ => &[],
        };

        // accept a single recorded candidate when present
        if candidates.len() == 1 {
            return candidates.first().map(|candidate| candidate.target_symbol);
        }
    }

    // fall back to direct reference targets on the callee expression
    let dir_tree = ctx.tree();
    let left_expr = dir_tree.get::<Expression>(left);
    match left_expr {
        Expression::GlobalReference { target_symbol, .. }
        | Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. } => Some(*target_symbol),
        _ => None,
    }
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
    // require a resolved target symbol for parameter extraction
    let Some(symbol_id) = target_symbol else {
        return Vec::new();
    };

    // reuse the shared parameter name extraction helper
    dynamic_parameter_names(session, symbol_id).unwrap_or_default()
}
