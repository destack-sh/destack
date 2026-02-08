use crate::DestackFormatContext;
use destack_ast::{Expression, FunctionMode, LocalNodeId, Parameter};

/// Return whether this parameter is variadic.
pub(crate) fn parameter_is_variadic(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    matches!(
        context.tree.get(parameter_id),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
    )
}

/// Return whether this parameter declares any modifiers.
pub(crate) fn parameter_has_modifier(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    match context.tree.get(parameter_id) {
        Parameter::Named { modifiers, .. }
        | Parameter::Pattern { modifiers, .. }
        | Parameter::VariadicNamed { modifiers, .. }
        | Parameter::VariadicPattern { modifiers, .. } => modifiers.is_some(),
    }
}

/// Return whether constructor parameter lists should break by default.
pub(crate) fn constructor_parameters_should_expand(
    context: &DestackFormatContext<'_>,
    mode: Option<FunctionMode>,
    parameters: &[LocalNodeId<Parameter>],
) -> bool {
    matches!(mode, Some(FunctionMode::Constructor | FunctionMode::New))
        && parameters.len() > 1
        && parameters
            .iter()
            .any(|parameter_id| parameter_has_modifier(context, *parameter_id))
}

/// Return whether a single parameter should keep compact outer parentheses.
pub(crate) fn single_parameter_should_hug(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    if parameter_is_variadic(context, parameter_id) {
        return false;
    }

    let has_multiline_collection_default = match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => default
            .is_some_and(|default_id| {
                matches!(
                    context.tree.get(default_id),
                    Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
                ) && context.has_newline(context.get_span(default_id))
            }),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => false,
    };
    if has_multiline_collection_default {
        return false;
    }

    let has_newline = context.has_newline(context.get_span(parameter_id));
    match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } => !(has_newline && default.is_some()),
        Parameter::VariadicNamed { .. } => !has_newline,
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => true,
    }
}
