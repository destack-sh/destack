use crate::DestackFormatContext;
use crate::format::chain::{
    argument_value_id_if_present, is_expression_chain, transparent_inner_expression,
};
use crate::format::expression::{is_complex_argument, is_trivial_expression};
use crate::format::tree::has_multiline_jsx_argument;
use destack_ast::{
    AnnotationPosition, Argument, Declaration, Expression, FunctionKind, LocalNodeId, Node,
    NodeTree, NodeTreeImpl, Parameter, TemplateLiteral, TokenType,
};

/// Decide whether a call can drop one parenthesized callee wrapper.
pub(crate) fn call_drops_parenthesized_callee_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    parent_expression: &Expression,
) -> bool {
    matches!(
        parent_expression,
        Expression::Call { left, .. } if *left == parenthesized_id
    ) && crate::format::operator::expression_has_static_type_arguments(context, inner_expression_id)
        && !context.has_annotation(parenthesized_id)
        && !context.has_annotation(inner_expression_id)
        && !crate::format::expression::parenthesized_has_leading_inner_trivia(
            context,
            parenthesized_id,
            inner_expression_id,
        )
}

/// Return whether one file-local range contains a line comment.
fn range_has_line_comment(context: &DestackFormatContext<'_>, start: u32, end: u32) -> bool {
    context
        .comments_in_range(start, end)
        .iter()
        .copied()
        .any(|comment| context.comment_is_line(comment))
}

/// Return whether one expression node is a lambda declaration.
fn expression_is_lambda_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function { signature, .. }
                    if signature.kind == FunctionKind::Lambda
            )
    )
}

/// Return whether an argument is trivial and free of annotations or lambda values.
pub(crate) fn argument_is_trivial_unannotated_non_lambda_value(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.has_annotation(argument_id) {
        return false;
    }

    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    if expression_is_lambda_declaration(context, value_id) {
        return false;
    }
    if context.has_annotation(value_id) {
        return false;
    }

    let value = context.tree.get(value_id);
    is_trivial_expression(context.tree, value)
}

/// Return whether one argument is a lambda with a block body.
pub(crate) fn argument_is_block_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };
    let Declaration::Function {
        signature, body, ..
    } = context.tree.get(*declaration_id)
    else {
        return false;
    };
    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    body.is_some_and(|body_id| matches!(context.tree.get(body_id), Expression::Block(_)))
}

/// Return whether one argument is an object literal expression.
pub(crate) fn argument_is_object_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_value_id_if_present(context.tree, argument_id)
        .map(|value_id| transparent_inner_expression(context, value_id))
        .is_some_and(|value_id| {
            matches!(
                context.tree.get(value_id),
                Expression::ObjectExpression { .. }
            )
        })
}

/// Return whether one argument is an array literal expression.
pub(crate) fn argument_is_array_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_value_id_if_present(context.tree, argument_id)
        .map(|value_id| transparent_inner_expression(context, value_id))
        .is_some_and(|value_id| {
            matches!(
                context.tree.get(value_id),
                Expression::ArrayExpression { .. }
            )
        })
}

/// Return whether one argument is a template literal expression.
pub(crate) fn argument_is_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_value_id_if_present(context.tree, argument_id)
        .map(|value_id| transparent_inner_expression(context, value_id))
        .is_some_and(|value_id| {
            matches!(
                context.tree.get(value_id),
                Expression::TemplateExpression { .. }
            )
        })
}

/// Return whether one lambda body is complex enough for tree formatting.
pub(crate) fn lambda_body_is_complex_for_tree(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Declaration::Function { body, .. } = context.tree.get(declaration_id) else {
        return false;
    };
    let Some(body_id) = *body else {
        return false;
    };

    matches!(
        context.tree.get(body_id),
        Expression::Block(_) | Expression::TreeExpression { .. }
    )
}

/// Return whether one argument is a lambda with a complex body for tree literals.
pub(crate) fn argument_is_complex_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };

    lambda_body_is_complex_for_tree(context, *declaration_id)
}

/// Return whether one expression contains a call with a complex callback.
pub(crate) fn expression_has_complex_callback(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;
    let expression_id = transparent_inner_expression(context, expression_id);

    match tree.get(expression_id) {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => {
            let has_complex_argument = dynamic_arguments
                .iter()
                .any(|arg_id| argument_is_complex_callback(context, *arg_id));
            if has_complex_argument {
                return true;
            }

            expression_has_complex_callback(context, *left)
        }
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::Parenthesized { expression: left } => {
            expression_has_complex_callback(context, *left)
        }
        Expression::Declaration(declaration_id) => {
            lambda_body_is_complex_for_tree(context, *declaration_id)
        }
        Expression::Block(block_id) => tree
            .get(*block_id)
            .iter_expressions()
            .any(|expr_id| expression_has_complex_callback(context, expr_id)),
        _ => false,
    }
}

/// Return whether an argument is a compact inline callback candidate.
pub(crate) fn argument_is_compact_inline_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);
    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };
    let Declaration::Function {
        signature, body, ..
    } = context.tree.get(*declaration_id)
    else {
        return false;
    };

    // signature shape
    if signature.this_parameter.is_some() || signature.return_type.is_some() {
        return false;
    }
    if signature.dynamic_parameters.len() > 1 {
        return false;
    }

    // parameter shape
    if signature
        .dynamic_parameters
        .first()
        .is_some_and(|parameter_id| {
            !matches!(
                context.tree.get(*parameter_id),
                Parameter::Named {
                    modifiers: None,
                    ty: None,
                    default: None,
                    ..
                }
            )
        })
    {
        return false;
    }

    // body shape
    body.is_some_and(|body_id| {
        let body_id = transparent_inner_expression(context, body_id);

        match context.tree.get(body_id) {
            Expression::Block(block_id) => context.tree.get(*block_id).len() <= 1,
            Expression::TreeExpression { .. } => true,
            expression => is_trivial_expression(context.tree, expression),
        }
    })
}

/// Return whether an argument is an interpolated template literal.
pub(crate) fn argument_is_interpolated_template_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);

    matches!(
        context.tree.get(value_id),
        Expression::TemplateExpression {
            value: TemplateLiteral::InterpolatedString { .. }
        }
    )
}

/// Return whether one argument carries a raw line comment.
pub(crate) fn argument_has_line_comment(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let argument_span = context.span(argument_id);
    range_has_line_comment(context, argument_span.start, argument_span.end)
}

/// Return whether one argument has a raw prefix line comment before its value.
pub(crate) fn argument_has_prefix_line_comment(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let argument_span = context.span(argument_id);
    let Some(argument_value_expression) = argument_value_id_if_present(context.tree, argument_id)
    else {
        return false;
    };

    let value_id = transparent_inner_expression(context, argument_value_expression);
    let value_span = context.span(value_id);
    if argument_span.file != value_span.file || argument_span.start >= value_span.start {
        return false;
    }

    range_has_line_comment(context, argument_span.start, value_span.start)
}

/// Return whether one argument carries semantic prefix signal on itself or its value path.
fn argument_has_prefix_signal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.has_prefix_annotation(argument_id) {
        return true;
    }

    let Some(argument_value_expression) = argument_value_id_if_present(context.tree, argument_id)
    else {
        return false;
    };

    let value_id = transparent_inner_expression(context, argument_value_expression);
    if context.has_prefix_annotation(value_id) {
        return true;
    }

    if let Expression::Declaration(declaration_id) = context.tree.get(value_id) {
        return context.has_prefix_annotation(*declaration_id);
    }

    false
}

/// Return whether one node span carries a raw boundary line comment.
fn node_has_boundary_line_comment<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> bool
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    let span = context.span(node_id);
    range_has_line_comment(context, span.start, span.end)
}

/// Compute call argument layout facts for one call expression.
pub(crate) fn call_argument_layout_facts(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> (bool, bool, bool, bool, bool, bool, usize, usize, bool, bool) {
    let has_call_infix_annotations = ctx.has_infix_annotation(call_node_id);

    if dynamic_arguments.is_empty() {
        return (
            has_call_infix_annotations,
            false,
            false,
            true,
            true,
            false,
            0,
            0,
            false,
            false,
        );
    }

    let mut has_any_argument_annotation = false;
    let mut has_line_comments = false;
    let mut all_single_line_and_unannotated = true;
    let mut all_compact_simple_unannotated = true;
    let mut arrow_argument_count = 0usize;
    let mut function_argument_count = 0usize;
    let mut trailing_collection_argument = false;
    let mut has_complex_non_callback_argument = false;
    let mut has_boundary_comments = false;

    let call_span = ctx.span(call_node_id);
    let last_argument_index = dynamic_arguments.len().saturating_sub(1);

    // scan each argument once and collect layout flags
    for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
        let argument_span = ctx.span(argument_id);

        // boundary comment seams
        if index == 0
            && call_span.file == argument_span.file
            && call_span.start < argument_span.start
            && let Some(open_parenthesis_token) =
                ctx.previous_non_whitespace_token_before_span(argument_span)
            && open_parenthesis_token.token.ty == TokenType::OpenParenthesis
            && open_parenthesis_token.span.end < argument_span.start
        {
            let leading_boundary_span = destack_source::Span::new(
                argument_span.file,
                open_parenthesis_token.span.end,
                argument_span.start,
            );
            has_boundary_comments |=
                range_has_line_comment(ctx, leading_boundary_span.start, leading_boundary_span.end);
        }
        if index > 0 && !has_boundary_comments {
            let previous_argument_span = ctx.span(dynamic_arguments[index - 1]);
            if let Some(between_span) = previous_argument_span.gap_to(argument_span) {
                has_boundary_comments =
                    range_has_line_comment(ctx, between_span.start, between_span.end);
            }
        }

        // layout decisions only treat non blank annotations as comment signals
        let has_annotation = ctx.has_annotation(argument_id);
        let has_newline = ctx.node_has_newline(argument_id);
        let is_last_argument = index == last_argument_index;
        let has_line_comment = argument_has_line_comment(ctx, argument_id);
        if has_annotation {
            has_any_argument_annotation = true;
            if !has_line_comments && has_line_comment {
                has_line_comments = true;
            }
        }

        let is_single_line_and_unannotated = !has_annotation && !has_newline;
        let should_check_compact_simple_unannotated =
            is_single_line_and_unannotated && all_compact_simple_unannotated;
        let compact_simple_unannotated = should_check_compact_simple_unannotated
            .then(|| argument_is_trivial_unannotated_non_lambda_value(ctx, argument_id));

        all_single_line_and_unannotated &= is_single_line_and_unannotated;
        if all_compact_simple_unannotated {
            all_compact_simple_unannotated = compact_simple_unannotated.unwrap_or(false);
        }

        let argument = ctx.tree.get(argument_id);
        let Some(value_id) = argument_value_id_if_present(ctx.tree, argument_id) else {
            continue;
        };
        let value_id = transparent_inner_expression(ctx, value_id);
        let value = ctx.tree.get(value_id);

        if is_last_argument {
            trailing_collection_argument = matches!(
                value,
                Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
            );
        }

        let (is_lambda_argument, is_function_argument, _is_block_callback) = match value {
            Expression::Declaration(declaration_id) => match ctx.tree.get(*declaration_id) {
                Declaration::Function {
                    signature, body, ..
                } => {
                    let is_lambda_argument = signature.kind == FunctionKind::Lambda;
                    let is_function_argument = !is_lambda_argument;
                    let is_block_callback = is_lambda_argument
                        && body.is_some_and(|body_id| {
                            let body_id = transparent_inner_expression(ctx, body_id);
                            matches!(ctx.tree.get(body_id), Expression::Block(_))
                        });
                    (is_lambda_argument, is_function_argument, is_block_callback)
                }
                _ => (false, false, false),
            },
            _ => (false, false, false),
        };

        if is_lambda_argument {
            arrow_argument_count += 1;
        }
        if is_function_argument {
            function_argument_count += 1;
        }

        if is_lambda_argument || is_function_argument {
            continue;
        }

        if !has_complex_non_callback_argument
            && !matches!(value, Expression::TreeExpression { .. })
            && is_complex_argument(ctx.tree, argument)
        {
            has_complex_non_callback_argument = true;
        }
    }

    // trailing boundary seam
    if !has_boundary_comments && let Some(last_argument_id) = dynamic_arguments.last().copied() {
        let last_argument_span = ctx.span(last_argument_id);
        if call_span.file == last_argument_span.file && last_argument_span.end < call_span.end {
            let token_index = ctx
                .tokens
                .partition_point(|token| token.span.start < last_argument_span.end);
            let close_parenthesis_start = ctx.tokens[token_index..]
                .iter()
                .find(|token| token.token.ty == TokenType::CloseParenthesis)
                .map(|token| token.span.start)
                .unwrap_or(call_span.end);
            let boundary_end = close_parenthesis_start.min(call_span.end);
            let boundary_span = destack_source::Span::new(
                last_argument_span.file,
                last_argument_span.end,
                boundary_end,
            );
            has_boundary_comments =
                range_has_line_comment(ctx, boundary_span.start, boundary_span.end);
        }
    }

    (
        has_call_infix_annotations,
        has_boundary_comments,
        has_any_argument_annotation,
        all_single_line_and_unannotated,
        all_compact_simple_unannotated,
        has_line_comments,
        arrow_argument_count,
        function_argument_count,
        has_complex_non_callback_argument,
        trailing_collection_argument,
    )
}

/// Return whether one argument has callback-blocking line comments or multiline prefix signal.
pub(crate) fn argument_has_callback_blocking_comment(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_has_line_comment(ctx, argument_id)
        || (ctx.node_has_newline(argument_id) && argument_has_prefix_signal(ctx, argument_id))
}

// call argument layout thresholds
const MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT: usize = 2;

/// Resolve chain call argument force-expand state.
pub(crate) fn chain_call_argument_force_expand(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let (
        has_call_infix_annotations,
        _has_boundary_comments,
        _has_any_argument_annotation,
        _all_single_line_and_unannotated,
        _all_compact_simple_unannotated,
        has_line_comments,
        arrow_argument_count,
        function_argument_count,
        has_complex_non_callback_argument,
        trailing_collection_argument,
    ) = call_argument_layout_facts(ctx, call_node_id, dynamic_arguments);
    if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];
        let has_collection_source_comment = argument_is_collection_literal(ctx, argument_id) && {
            let argument_span = ctx.span(argument_id);
            !ctx.comments_in_range(argument_span.start, argument_span.end)
                .is_empty()
        };

        has_multiline_jsx_argument(ctx, dynamic_arguments)
            || has_call_infix_annotations
            || argument_has_line_comment(ctx, argument_id)
            || argument_has_prefix_line_comment(ctx, argument_id)
            || has_collection_source_comment
            || single_argument_requires_expanded_list(ctx, dynamic_arguments)
    } else {
        let has_callback_prefix = dynamic_arguments[..dynamic_arguments.len().saturating_sub(1)]
            .iter()
            .copied()
            .any(|argument_id| argument_is_block_callback(ctx, argument_id));

        has_multiline_jsx_argument(ctx, dynamic_arguments)
            || has_call_infix_annotations
            || has_line_comments
            || has_complex_non_callback_argument
            || arrow_argument_count >= MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT
            || function_argument_count >= MULTIPLE_FUNCTION_ARGUMENT_MIN_COUNT
            || (trailing_collection_argument && has_callback_prefix)
    }
}

/// Return whether a single static argument call should force expansion.
pub(crate) fn call_force_expand_single_multiline_with_static_arguments(
    _ctx: &DestackFormatContext<'_>,
    _call_node_id: LocalNodeId<Expression>,
    _dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    false
}

/// Return whether a single collection argument should expand for type binary callees.
pub(crate) fn call_force_expand_single_collection_for_type_binary_callee(
    _ctx: &DestackFormatContext<'_>,
    _call_node_id: LocalNodeId<Expression>,
    _dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    false
}

/// Return whether a single argument call should force expanded list layout.
pub(crate) fn single_argument_requires_expanded_list(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    // only single argument calls can use this path
    if dynamic_arguments.len() != 1 {
        return false;
    }

    // require a chain shaped argument value
    let argument_id = dynamic_arguments[0];
    let Some(raw_value_id) = argument_value_id_if_present(ctx.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(ctx, raw_value_id);
    let is_chain_layout_candidate = is_expression_chain(ctx.tree, value_id)
        || matches!(
            ctx.tree.get(value_id),
            Expression::QualifiedReference { path, .. } if path.segments.len() > 1
        )
        || matches!(
            ctx.tree.get(value_id),
            Expression::Call { left, .. } | Expression::Instantiation { left, .. }
                if matches!(
                    ctx.tree.get(*left),
                    Expression::QualifiedReference { path, .. } if path.segments.len() > 1
                )
        );
    if !is_chain_layout_candidate {
        return false;
    }

    // force expand only from non-boundary annotation signals on the argument value path
    let has_non_boundary_annotation_signal = ctx.has_non_boundary_annotation(argument_id)
        || ctx.has_non_boundary_annotation(raw_value_id)
        || ctx.has_non_boundary_annotation(value_id);
    let has_boundary_signal = node_has_boundary_line_comment(ctx, argument_id)
        || node_has_boundary_line_comment(ctx, raw_value_id)
        || node_has_boundary_line_comment(ctx, value_id);

    has_non_boundary_annotation_signal || has_boundary_signal
}

/// Return whether an argument is a collection literal.
pub(crate) fn argument_is_collection_literal(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    argument_is_object_literal(context, argument_id)
        || argument_is_array_literal(context, argument_id)
}

/// Return whether one argument is an inline closure-cast object argument.
pub(crate) fn argument_is_inline_closure_cast_object(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if context.node_has_newline(argument_id) {
        return false;
    }

    let argument_span = context.span(argument_id);
    let Some(value_id) = argument_value_id_if_present(context.tree, argument_id) else {
        return false;
    };
    let value_id = transparent_inner_expression(context, value_id);
    if !matches!(
        context.tree.get(value_id),
        Expression::ObjectExpression { .. }
    ) {
        return false;
    }

    let value_span = context.span(value_id);
    let argument_annotations = context.annotation_ids(argument_id);
    let argument_has_inline_prefix = !argument_annotations.is_empty()
        && argument_annotations.iter().all(|annotation_id| {
            match context.annotation(*annotation_id) {
                crate::Annotation::Doc { position, .. } => {
                    if !matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    ) {
                        return false;
                    }

                    let annotation_span = context.annotation_span(*annotation_id);
                    annotation_span.start < argument_span.start
                }
                crate::Annotation::Decorator { .. } => false,
            }
        });
    let value_annotations = context.annotation_ids(value_id);
    let value_has_inline_prefix = !value_annotations.is_empty()
        && value_annotations
            .iter()
            .all(|annotation_id| match context.annotation(*annotation_id) {
                crate::Annotation::Doc { position, .. } => {
                    if !matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    ) {
                        return false;
                    }

                    let annotation_span = context.annotation_span(*annotation_id);
                    annotation_span.start < value_span.start
                }
                crate::Annotation::Decorator { .. } => false,
            });

    argument_has_inline_prefix || value_has_inline_prefix
}

/// Return whether a call-like expression has static type arguments.
pub(crate) fn call_has_static_arguments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(node_id) {
        Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        _ => false,
    }
}
