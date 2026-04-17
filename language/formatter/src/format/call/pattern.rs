use crate::DestackFormatContext;
use crate::format::chain::{
    argument_value_id_if_present, chain_has_call_like_expression, is_expression_chain,
    transparent_inner_expression,
};
use crate::format::declaration::expression_is_in_statement_position;
use destack_ast::{
    Argument, Declaration, Expression, FunctionKind, LocalNodeId, NodeType, ScalarLiteral,
    TemplateLiteral,
};
use destack_source::Span;

/// Return one argument's transparent expression value, if present.
pub(crate) fn argument_expression_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    let value_id = argument_value_id_if_present(context.tree, argument_id)?;
    Some(transparent_inner_expression(context, value_id))
}

/// Return whether one argument is a template literal expression.
pub(crate) fn argument_is_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_expression_id(context, argument_id).is_some_and(|value_id| {
        matches!(
            context.tree.get(value_id),
            Expression::TemplateExpression { .. }
        )
    })
}

/// Return whether an argument is an interpolated template literal.
pub(crate) fn argument_is_interpolated_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_expression_id(context, argument_id) else {
        return false;
    };

    matches!(
        context.tree.get(value_id),
        Expression::TemplateExpression {
            value: TemplateLiteral::InterpolatedString { .. }
        }
    )
}

/// Return whether one expression is a multiline template that starts on the same line.
fn expression_is_multiline_template_starting_on_same_line(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_span = context.span(expression_id);
    let template_span = match context.tree.get(expression_id) {
        Expression::TemplateExpression { .. } | Expression::TaggedTemplateExpression { .. } => {
            expression_span
        }
        _ => return false,
    };

    context.source_text().contains_newline(template_span)
        && !context
            .source_text()
            .has_newline_before(expression_span.start)
}

/// Return whether one argument list contains exactly one multiline template argument.
pub(crate) fn is_multiline_template_only_args(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if arguments.len() != 1 {
        return false;
    }

    argument_expression_id(context, arguments[0]).is_some_and(|expression_id| {
        expression_is_multiline_template_starting_on_same_line(context, expression_id)
    })
}

/// Return whether one call is the head of a longer curried call chain.
pub(crate) fn expression_is_long_curried_call(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { arguments, .. } = context.tree.get(call_node_id) else {
        return false;
    };

    let Some((parent_id, parent_type)) = context.parent(call_node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_call_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Call {
        left,
        arguments: parent_dynamic_arguments,
        ..
    } = context.tree.get(parent_call_id)
    else {
        return false;
    };

    *left == call_node_id
        && arguments.len() > parent_dynamic_arguments.len()
        && !parent_dynamic_arguments.is_empty()
}

/// Return whether one call expression should use member-chain formatting.
pub(crate) fn call_should_route_to_chain(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if !is_expression_chain(context.tree, node_id)
        || !chain_has_call_like_expression(context.tree, node_id)
    {
        return false;
    }

    if is_multiline_template_only_args(context, arguments) {
        return false;
    }

    if call_uses_simple_list_layout(context, node_id, left, arguments) {
        return false;
    }

    expression_is_member_chain_callee(context, left)
}

/// Return whether one instantiation expression should use member-chain formatting.
pub(crate) fn instantiation_should_route_to_chain(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
) -> bool {
    if !is_expression_chain(context.tree, node_id)
        || !chain_has_call_like_expression(context.tree, node_id)
    {
        return false;
    }

    expression_is_member_chain_callee(context, left)
}

/// Return whether one call callee is a member-chain root.
fn expression_is_member_chain_callee(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::Member { .. } | Expression::PrivateMember { .. } | Expression::Index { .. } => {
            true
        }
        Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
            expression_is_member_chain_callee(context, *left)
        }
        _ => false,
    }
}

/// Return whether one call should use the direct flat argument writer.
pub(crate) fn call_uses_simple_list_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if is_multiline_template_only_args(context, arguments)
        || is_react_hook_with_deps_array(context, call_node_id, arguments)
    {
        return true;
    }

    matches!(context.tree.get(call_node_id), Expression::Call { .. })
        && (is_simple_module_import_call(context, call_node_id, left, arguments)
            || is_commonjs_or_amd_call(context, call_node_id, left, arguments)
            || is_test_call_expression(context, call_node_id, left, arguments))
}

/// Return whether one call is a simple module import helper.
fn is_simple_module_import_call(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if arguments.len() != 1
        || !argument_is_string_literal(context, arguments[0])
        || has_comment_in_span(context, context.span(call_node_id))
    {
        return false;
    }

    if is_require_resolve_call(context, left) {
        return true;
    }

    is_require_resolve_paths_call(context, left) || is_import_meta_resolve_call(context, left)
}

/// Return whether one call is a CommonJS or AMD call pattern.
fn is_commonjs_or_amd_call(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if name_is_identifier(context, left, "require") {
        let Some(first_argument_id) = arguments.first().copied() else {
            return false;
        };

        if has_comment_in_range(
            context,
            context.span(call_node_id).start,
            context.span(first_argument_id).start,
        ) {
            return false;
        }

        return match arguments.len() {
            1 => argument_is_string_literal(context, first_argument_id),
            _ => true,
        };
    }

    if !name_is_identifier(context, left, "define")
        || !expression_is_in_statement_position(context, call_node_id)
    {
        return false;
    }

    match arguments {
        [_first] => true,
        [first, second] => {
            argument_is_array_expression(context, *first)
                && argument_expression_id(context, *second).is_some()
        }
        [first, second, third] => {
            argument_is_string_literal(context, *first)
                && argument_is_array_expression(context, *second)
                && argument_expression_id(context, *third).is_some()
        }
        _ => false,
    }
}

/// Return whether one call expression matches one test-style pattern.
fn is_test_call_expression(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    match arguments {
        [argument_id] => {
            if is_angular_test_wrapper_call(context, left)
                && context
                    .parent(call_node_id)
                    .is_some_and(|(parent_id, parent_type)| {
                        if parent_type != NodeType::Expression {
                            return false;
                        }

                        let parent_call_id = LocalNodeId::<Expression>::new(parent_id);
                        let Expression::Call {
                            left, arguments, ..
                        } = context.tree.get(parent_call_id)
                        else {
                            return false;
                        };

                        *left == call_node_id
                            && is_test_call_expression(context, parent_call_id, *left, arguments)
                    })
            {
                return argument_expression_id(context, *argument_id).is_some_and(
                    |expression_id| expression_is_function_callback(context, expression_id),
                );
            }

            if is_unit_test_setup_callee(context, left) {
                return argument_expression_id(context, *argument_id).is_some_and(
                    |expression_id| is_angular_test_wrapper_expression(context, expression_id),
                );
            }

            false
        }
        [first_argument_id, second_argument_id] | [first_argument_id, second_argument_id, _]
            if arguments.len() <= 3
                && argument_is_string_or_template_literal(context, *first_argument_id)
                && contains_test_pattern(context, left) =>
        {
            let allow_any_callback_shape = arguments.len() == 2;
            let third_argument = arguments.get(2).copied();
            if third_argument
                .is_some_and(|argument_id| !argument_is_numeric_literal(context, argument_id))
            {
                return false;
            }

            let Some(second_expression_id) = argument_expression_id(context, *second_argument_id)
            else {
                return false;
            };
            if is_angular_test_wrapper_expression(context, second_expression_id) {
                return true;
            }

            expression_is_test_callback(context, second_expression_id, allow_any_callback_shape)
        }
        _ => false,
    }
}

/// Return whether one call uses the callback and dependency-array hook layout.
fn is_react_hook_with_deps_array(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    if !(2..=3).contains(&arguments.len()) {
        return false;
    }

    let callback_index = if arguments.len() == 3 {
        if !argument_is_identifier(context, arguments[0]) {
            return false;
        }

        1
    } else {
        0
    };

    let Some(callback_id) = argument_expression_id(context, arguments[callback_index]) else {
        return false;
    };
    let Some(deps_id) = argument_expression_id(context, arguments[callback_index + 1]) else {
        return false;
    };
    if !expression_is_zero_parameter_block_callback(context, callback_id)
        || !matches!(
            context.tree.get(deps_id),
            Expression::ArrayExpression { .. }
        )
    {
        return false;
    }

    let callback_span = context.span(callback_id);
    let deps_span = context.span(deps_id);
    let call_span = context.span(call_node_id);
    let comments = context.comments();
    !comments
        .comments_in_range(call_span.start, call_span.end)
        .iter()
        .any(|comment| {
            !span_contains_span(callback_span, comment.span)
                && !span_contains_span(deps_span, comment.span)
        })
}

/// Return whether one expression is an Angular-style test wrapper call.
fn is_angular_test_wrapper_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = context.tree.get(expression_id) else {
        return false;
    };

    is_angular_test_wrapper_call(context, *left)
}

/// Return whether one callee is an Angular-style test wrapper.
fn is_angular_test_wrapper_call(
    context: &DestackFormatContext<'_>,
    callee_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(callee_id),
        Expression::Identifier { name }
            if matches!(
                context.strings.get(*name),
                "async" | "inject" | "fakeAsync" | "waitForAsync"
            )
    )
}

/// Return whether one callee is a unit-test setup helper.
fn is_unit_test_setup_callee(
    context: &DestackFormatContext<'_>,
    callee_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(callee_id),
        Expression::Identifier { name }
            if matches!(
                context.strings.get(*name),
                "beforeEach" | "beforeAll" | "afterEach" | "afterAll"
            )
    )
}

/// Return whether one expression is a function callback.
fn expression_is_function_callback(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
        Declaration::Function { .. }
    )
}

/// Return whether one expression is a test callback.
fn expression_is_test_callback(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    allow_any_callback_shape: bool,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Function(function) = context.tree.get(*declaration_id) else {
        return false;
    };

    if allow_any_callback_shape {
        return true;
    }

    let Some(body_id) = function.body else {
        return false;
    };

    function.signature.parameters.len() <= 1
        && matches!(context.tree.get(body_id), Expression::Block(_))
}

/// Return whether one expression is a zero-parameter callback with a block body.
fn expression_is_zero_parameter_block_callback(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Function(function) = context.tree.get(*declaration_id) else {
        return false;
    };
    let Some(body_id) = function.body else {
        return false;
    };

    function.signature.kind == FunctionKind::Lambda
        && function.signature.parameters.is_empty()
        && matches!(context.tree.get(body_id), Expression::Block(_))
}

/// Return whether one argument is a string literal.
fn argument_is_string_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_expression_id(context, argument_id).is_some_and(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::ScalarLiteral(ScalarLiteral::String(_))
        )
    })
}

/// Return whether one argument is a string or template literal.
fn argument_is_string_or_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_expression_id(context, argument_id).is_some_and(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::ScalarLiteral(ScalarLiteral::String(_))
                | Expression::TemplateExpression { .. }
        )
    })
}

/// Return whether one argument is an identifier expression.
fn argument_is_identifier(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_expression_id(context, argument_id).is_some_and(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Identifier { .. }
        )
    })
}

/// Return whether one argument is an array expression.
fn argument_is_array_expression(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_expression_id(context, argument_id).is_some_and(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::ArrayExpression { .. }
        )
    })
}

/// Return whether one argument is a numeric literal.
fn argument_is_numeric_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_expression_id(context, argument_id).is_some_and(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_) | ScalarLiteral::Float(_))
        )
    })
}

/// Return whether one expression is an identifier with the expected text.
fn name_is_identifier(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    expected: &str,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        Expression::Identifier { name } if context.strings.get(*name) == expected
    )
}

/// Return whether one callee is `require.resolve`.
fn is_require_resolve_call(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Member { left, name, .. } = context.tree.get(expression_id) else {
        return false;
    };
    let Some(name) = name else {
        return false;
    };
    if context.strings.get(*name) != "resolve" {
        return false;
    }

    name_is_identifier(context, *left, "require")
}

/// Return whether one callee is `require.resolve.paths`.
fn is_require_resolve_paths_call(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Member { left, name, .. } = context.tree.get(expression_id) else {
        return false;
    };
    let Some(name) = name else {
        return false;
    };
    if context.strings.get(*name) != "paths" {
        return false;
    }

    is_require_resolve_call(context, *left)
}

/// Return whether one callee is `import.meta.resolve`.
fn is_import_meta_resolve_call(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Member { left, name, .. } = context.tree.get(expression_id) else {
        return false;
    };
    let Some(name) = name else {
        return false;
    };
    if context.strings.get(*name) != "resolve" {
        return false;
    }

    matches!(context.tree.get(*left), Expression::ImportMeta)
}

/// Return the static callee names for one identifier or member chain.
fn callee_names(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<Vec<destack_core::StringId>> {
    let mut names = Vec::with_capacity(5);
    let mut current_id = expression_id;

    for _ in 0..5 {
        match context.tree.get(current_id) {
            Expression::Identifier { name } => {
                names.push(*name);
                names.reverse();
                return Some(names);
            }
            Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                names.push(*name);
                current_id = *left;
            }
            _ => return None,
        }
    }

    None
}

/// Return whether one callee name chain matches a known test pattern.
fn contains_test_pattern(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(name_ids) = callee_names(context, expression_id) else {
        return false;
    };
    let names = name_ids
        .iter()
        .map(|name| context.strings.get(*name))
        .collect::<Vec<_>>();
    let names = names.as_slice();

    match names {
        ["it"]
        | ["skip"]
        | ["xit"]
        | ["xdescribe"]
        | ["xtest"]
        | ["fit"]
        | ["fdescribe"]
        | ["ftest"] => true,
        [
            "it",
            "only" | "skip" | "skipIf" | "runIf" | "concurrent" | "sequential" | "todo" | "fails",
        ] => true,
        ["describe"] => true,
        [
            "describe",
            "only" | "skip" | "skipIf" | "runIf" | "concurrent" | "sequential" | "shuffle" | "todo",
        ] => true,
        ["Deno", "test"] => true,
        ["test"] => true,
        [
            "test",
            "only" | "skip" | "skipIf" | "runIf" | "concurrent" | "sequential" | "todo" | "fails"
            | "extend" | "step" | "fixme",
        ] => true,
        ["test", "describe"] => true,
        ["test", "describe", "only" | "skip" | "fixme"] => true,
        ["test", "describe", "parallel" | "serial"] => true,
        ["test", "describe", "parallel" | "serial", "only"] => true,
        ["bench"] => true,
        ["bench", "only" | "skip" | "todo"] => true,
        _ => false,
    }
}

/// Return whether one source range contains comments.
fn has_comment_in_range(context: &DestackFormatContext<'_>, start: u32, end: u32) -> bool {
    let comments = context.comments();
    comments.has_comment_in_range(start, end)
}

/// Return whether one span contains comments.
fn has_comment_in_span(context: &DestackFormatContext<'_>, span: Span) -> bool {
    let comments = context.comments();
    comments.has_comment_in_span(span)
}

/// Return whether one span fully contains another.
fn span_contains_span(outer: Span, inner: Span) -> bool {
    outer.file == inner.file && inner.start >= outer.start && inner.end <= outer.end
}
