use destack_dir as dir;
use destack_dir::{Argument, Declarator, Expression, GlobalSymbolId, Pattern, TemplateLiteral};
use destack_source::{FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use crate::core::with_query_context_for_file;
use crate::dir::{parameter_names_for_symbol, resolve_call_target};
use crate::format::format_type_for_inlay_hint;

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
        // build the type hint with default padding
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
        // build the parameter hint with default padding
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
pub fn inlay_hints(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    range: Span,
) -> Vec<InlayHint> {
    // resolve hints within the query context
    with_query_context_for_file(repository, revision, file, |ctx| {
        // resolve shared dir data for hint generation
        let dir_tree = ctx.dir().tree();
        let types = ctx.dir().types();

        // collect parameter and type hints
        let mut hints = Vec::new();

        // iterate through all call expressions for parameter hints
        for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
            // check if this is a call expression
            let Expression::Call {
                left, arguments, ..
            } = expression
            else {
                continue;
            };

            // skip if no arguments
            if arguments.is_empty() {
                continue;
            }

            // get the span of this call expression
            let ast_node_id = dir_tree.get_source(expression_id.id);
            let call_span = ctx.ast().tree().source_map.get(ast_node_id);

            // skip if outside the requested range
            if call_span.end < range.start || call_span.start > range.end {
                continue;
            }

            // resolve the target symbol for this call
            let call_target = resolve_call_target(repository, ctx.dir(), *left);
            let target_symbol = call_target.symbol;

            // get actual parameter names for this function
            let param_names = get_parameter_names(repository, revision, target_symbol);

            // skip parameter hints when we do not have names
            if param_names.is_empty() {
                continue;
            }

            // add parameter hints for each argument
            for (index, argument_id) in arguments.iter().enumerate() {
                // resolve the argument and parameter name
                let argument = dir_tree.get::<Argument>(*argument_id);
                let param_name = match param_names.get(index) {
                    Some(name) => name.as_str(),
                    None => continue,
                };
                let argument_is_literal = argument_is_literal(dir_tree, argument);

                // skip hints for arguments that already carry labels or match the name
                if should_skip_parameter_hint(
                    repository,
                    dir_tree,
                    argument,
                    param_name,
                    argument_is_literal,
                ) {
                    continue;
                }

                // get the span of the argument expression
                let arg_ast_id = dir_tree.get_source(argument.value().id);
                let arg_span = ctx.ast().tree().source_map.get(arg_ast_id);

                // add a parameter hint at the start of the argument
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

            // only emit hints for binding patterns
            if let Pattern::Binding {
                symbol, name: _, ..
            } = pattern
            {
                // get the span of the binding name
                let ast_node_id = dir_tree.get_source(declarator.pattern.id);
                let Some(name_span) = ctx.ast().tree().source_map.get_main(ast_node_id) else {
                    continue;
                };

                // skip if outside the requested range
                if name_span.end < range.start || name_span.start > range.end {
                    continue;
                }

                // try to get the value type for this symbol
                let global_symbol_id = GlobalSymbolId {
                    module_id: ctx.module_id(),
                    local_id: *symbol,
                };

                // resolve the inferred type when available
                if let Some(type_id) = types.get_value_type_id(global_symbol_id) {
                    // resolve the inferred type for the binding
                    let ty = types.get_type(type_id);

                    // format a widened display type for literal values
                    let type_str = format_type_for_inlay_hint(
                        ty,
                        types,
                        repository,
                        revision,
                        &repository.strings,
                    );

                    // add type hint after the binding name
                    hints.push(InlayHint::type_hint(name_span.end, type_str));
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

        // return the collected hints
        hints
    })
    .unwrap_or_default()
}

/// Rank inlay hint kinds for stable sorting.
fn hint_kind_rank(kind: InlayHintKind) -> u8 {
    // order types before parameter hints
    match kind {
        InlayHintKind::Type => 0,
        InlayHintKind::Parameter => 1,
    }
}

/// Get parameter names for a function.
///
/// If the target symbol points to a function declaration, extracts actual parameter names.
/// Returns an empty vec if not available (caller will skip parameter hints).
fn get_parameter_names(
    repository: &Repository,
    revision: Revision,
    target_symbol: Option<GlobalSymbolId>,
) -> Vec<String> {
    // require a resolved target symbol for parameter extraction
    let Some(symbol_id) = target_symbol else {
        return Vec::new();
    };

    // resolve parameter names from the DIR
    parameter_names_for_symbol(repository, revision, symbol_id).unwrap_or_default()
}

/// Decide whether a parameter hint should be skipped for an argument.
fn should_skip_parameter_hint(
    repository: &Repository,
    dir_tree: &dir::NodeTree,
    argument: &Argument,
    param_name: &str,
    argument_is_literal: bool,
) -> bool {
    // skip placeholders and empty names
    if param_name.is_empty() || param_name == "_" {
        return true;
    }

    // skip arguments that already carry labels
    if matches!(argument, Argument::Named { .. } | Argument::Labeled { .. }) {
        return true;
    }

    // skip spread arguments
    if matches!(argument, Argument::Spread { .. }) {
        return true;
    }

    // skip when parameter hints are disabled for non literal arguments
    if !should_emit_parameter_hint(argument_is_literal) {
        return true;
    }

    // skip when the argument already repeats the parameter name
    if let Some(reference) = argument_reference(repository, dir_tree, argument) {
        match reference {
            ArgumentReference::Name(argument_name) => {
                if argument_name == param_name && !parameter_name_hints_when_argument_matches_name()
                {
                    return true;
                }
            }
            ArgumentReference::This => return true,
        }
    }

    false
}

/// Extract a simple reference name from an argument value when available.
fn argument_reference(
    repository: &Repository,
    dir_tree: &dir::NodeTree,
    argument: &Argument,
) -> Option<ArgumentReference> {
    // resolve the argument expression
    let expr = dir_tree.get::<Expression>(argument.value());

    // resolve a simple reference name
    match expr {
        Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => path
            .last_segment()
            .map(|id| ArgumentReference::Name(repository.strings.get(id).to_string())),
        Expression::This => Some(ArgumentReference::This),
        _ => None,
    }
}

/// A simple reference extracted from an argument expression.
enum ArgumentReference {
    /// A named reference.
    Name(String),
    /// An explicit `this` reference.
    This,
}

/// Check whether an argument is a literal value.
fn argument_is_literal(dir_tree: &dir::NodeTree, argument: &Argument) -> bool {
    // resolve the argument expression
    let expr = dir_tree.get::<Expression>(argument.value());

    // treat scalar literals and static templates as literals
    match expr {
        Expression::ScalarLiteral { .. } => true,
        Expression::TemplateExpression { value } => {
            matches!(value, TemplateLiteral::String { .. })
        }
        _ => false,
    }
}

/// Decide whether parameter name hints are enabled for this argument.
fn should_emit_parameter_hint(argument_is_literal: bool) -> bool {
    // use literal detection as the hint gate
    argument_is_literal
}

/// Decide whether to include hints when argument matches the parameter name.
fn parameter_name_hints_when_argument_matches_name() -> bool {
    // disable redundant hints by default
    false
}
