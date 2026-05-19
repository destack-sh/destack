use destack_core::StringPool;
use destack_dir as dir;
use destack_dir::{Argument, Declarator, Expression, GlobalSymbolId, Pattern, TemplateLiteral};
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::core::{ModuleQueryContext, QueryRange};
use crate::dir::{call_target, parameter_names_for_symbol};
use crate::format::format_inlay_type;

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
    /// The queried range.
    pub range: QueryRange,
}

/// Response payload for inlay hints queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlayHintsResponse {
    /// Inlay hints.
    pub hints: Vec<InlayHint>,
}

/// Get inlay hints for a range in a file.
pub fn inlay_hints(ctx: &ModuleQueryContext<'_>, range: Span) -> Vec<InlayHint> {
    let dir_tree = ctx.dir().view();
    let types = ctx.dir().types();
    let mut hints = Vec::new();

    // collect parameter hints
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<Expression>() {
        let Expression::Call {
            left, arguments, ..
        } = expression
        else {
            continue;
        };
        if arguments.is_empty() {
            continue;
        }

        let source_node_id = dir_tree.get_source(expression_id);
        let call_span = ctx.dir().tree().source_map.get(source_node_id);
        if call_span.end < range.start || call_span.start > range.end {
            continue;
        }

        let call_target = call_target(ctx.dir(), *left);
        let param_names = get_parameter_names(ctx, call_target.symbol);
        if param_names.is_empty() {
            continue;
        }

        for (index, argument_id) in arguments.iter().enumerate() {
            let argument = dir_tree.get::<Argument>(*argument_id);
            let Some(param_name) = param_names.get(index) else {
                continue;
            };
            let Some(argument_value) = argument.value() else {
                continue;
            };
            let argument_is_literal = argument_is_literal(dir_tree, argument_value);
            if should_skip_parameter_hint(
                ctx.dir().strings(),
                dir_tree,
                argument,
                param_name,
                argument_is_literal,
            ) {
                continue;
            }

            let argument_source_node_id = dir_tree.get_source(argument_value);
            let arg_span = ctx.dir().tree().source_map.get(argument_source_node_id);
            hints.push(InlayHint::parameter_hint(arg_span.start, param_name));
        }
    }

    // collect inferred type hints
    for (_declarator_id, declarator) in dir_tree.iter_nodes_of_type::<Declarator>() {
        if declarator.ty.is_some() {
            continue;
        }

        let pattern = dir_tree.get::<Pattern>(declarator.pattern);
        if !matches!(pattern, Pattern::Binding { .. }) {
            continue;
        }

        let source_node_id = dir_tree.get_source(declarator.pattern);
        let Some(name_span) = ctx.dir().tree().source_map.get_main(source_node_id) else {
            continue;
        };
        if name_span.end < range.start || name_span.start > range.end {
            continue;
        }

        let Some(local_symbol) = ctx.dir().symbol_for_node(declarator.pattern.into()) else {
            continue;
        };
        let global_symbol_id = GlobalSymbolId::new(ctx.module_id(), local_symbol);
        let Some(type_id) = types.get_symbol_type_id(global_symbol_id) else {
            continue;
        };

        let ty = types.get_type(type_id);
        let type_str = format_inlay_type(ty, types, ctx);
        hints.push(InlayHint::type_hint(name_span.end, type_str));
    }

    // order and deduplicate hints
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
    hints.dedup_by(|left, right| {
        left.position == right.position && left.kind == right.kind && left.label == right.label
    });

    hints
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
    ctx: &ModuleQueryContext<'_>,
    target_symbol: Option<GlobalSymbolId>,
) -> Vec<String> {
    // require a resolved target symbol for parameter extraction
    let Some(symbol_id) = target_symbol else {
        return Vec::new();
    };

    // resolve parameter names from the DIR
    parameter_names_for_symbol(ctx, symbol_id).unwrap_or_default()
}

/// Decide whether a parameter hint should be skipped for an argument.
fn should_skip_parameter_hint(
    strings: &StringPool,
    dir_tree: dir::View<'_>,
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
    if let Some(reference) = argument_reference(strings, dir_tree, argument) {
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
    strings: &StringPool,
    dir_tree: dir::View<'_>,
    argument: &Argument,
) -> Option<ArgumentReference> {
    // resolve the argument expression
    let expr = dir_tree.get::<Expression>(argument.value()?);

    // resolve a simple reference name
    match expr {
        Expression::QualifiedReference { path, .. } => path
            .last_segment()
            .map(|id| ArgumentReference::Name(strings.get(id).to_string())),
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
fn argument_is_literal(
    dir_tree: dir::View<'_>,
    argument_value: dir::LocalNodeId<Expression>,
) -> bool {
    // resolve the argument expression
    let expr = dir_tree.get::<Expression>(argument_value);

    // treat scalar literals and static templates as literals
    match expr {
        Expression::ScalarLiteral(_) => true,
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
