use crate::format::call::layout::{
    CallArgumentLayoutFacts, argument_expression_id, argument_has_callback_blocking_comment,
    argument_has_line_comment, argument_has_prefix_line_comment, argument_is_block_callback,
    argument_is_collection_literal, argument_is_compact_inline_callback,
    argument_is_inline_closure_cast_object, argument_is_interpolated_template_literal,
    argument_is_template_literal, argument_is_trivial_unannotated_non_lambda_value,
    call_argument_layout_facts, call_has_static_arguments, chain_call_argument_force_expand,
    single_argument_requires_expanded_list,
};
use crate::format::chain::{argument_value_id_if_present, transparent_inner_expression};
use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::format::declaration::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::format::directive::{any_ignore_range_for_nodes, node_has_ignore_directive};
use crate::format::expression::{
    format_expression, format_static_argument_list, is_trivial_expression,
};
use crate::format::tree::has_multiline_jsx_argument;
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Argument, Declaration, Expression, FunctionKind, LocalNodeId, NodeType,
    PostfixPosition,
};
use destack_fir::format::{Buffer, FormatResult, GroupId};
use destack_fir::prelude::{block_indent, group, hard_line_break, soft_block_indent, space, token};
use destack_fir::{format_args, write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GroupedCallArgumentLayout {
    GroupedFirstArgument,
    GroupedLastArgument,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GroupedArgumentExpressionFamily {
    ArrayLike,
    FunctionLike,
    ObjectLike,
    Other,
}

/// Return whether one expression is a function-like call argument.
fn expression_is_function_like_argument(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = ctx.tree.get(expression_id) else {
        return false;
    };

    matches!(ctx.tree.get(*declaration_id), Declaration::Function { .. })
}

/// Return the grouped-layout family for one call argument expression.
fn grouped_argument_expression_family(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> GroupedArgumentExpressionFamily {
    match ctx.tree.get(expression_id) {
        Expression::ObjectExpression { .. } => GroupedArgumentExpressionFamily::ObjectLike,
        Expression::ArrayExpression { .. } => GroupedArgumentExpressionFamily::ArrayLike,
        Expression::TypeBinary {
            operator:
                destack_ast::TypeBinaryOperator::Cast | destack_ast::TypeBinaryOperator::Satisfies,
            left,
            ..
        } => grouped_argument_expression_family(ctx, transparent_inner_expression(ctx, *left)),
        _ if expression_is_function_like_argument(ctx, expression_id) => {
            GroupedArgumentExpressionFamily::FunctionLike
        }
        _ => GroupedArgumentExpressionFamily::Other,
    }
}

/// Return whether one expression is a block-bodied lambda call argument.
fn expression_is_block_lambda_argument(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = ctx.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = ctx.tree.get(*declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    let body_id = transparent_inner_expression(ctx, *body_id);
    matches!(ctx.tree.get(body_id), Expression::Block(_))
}

/// Return whether one expression can participate in grouped call-argument layout.
fn can_group_expression_argument(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match grouped_argument_expression_family(ctx, expression_id) {
        GroupedArgumentExpressionFamily::ObjectLike => {
            let Expression::ObjectExpression { properties, .. } = ctx.tree.get(expression_id)
            else {
                return false;
            };

            !properties.is_empty()
                || !ctx
                    .comments_in_range(ctx.span(expression_id).start, ctx.span(expression_id).end)
                    .is_empty()
        }
        GroupedArgumentExpressionFamily::ArrayLike => {
            let Expression::ArrayExpression { elements, .. } = ctx.tree.get(expression_id) else {
                return false;
            };

            !elements.is_empty()
                || !ctx
                    .comments_in_range(ctx.span(expression_id).start, ctx.span(expression_id).end)
                    .is_empty()
        }
        GroupedArgumentExpressionFamily::FunctionLike => true,
        GroupedArgumentExpressionFamily::Other => false,
    }
}

/// Return whether one expression is short enough to stay next to a grouped function argument.
fn expression_is_relatively_short_group_partner(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if ctx.node_has_newline(expression_id) || ctx.has_annotation(expression_id) {
        return false;
    }

    match ctx.tree.get(expression_id) {
        Expression::ObjectExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::If { .. }
        | Expression::Declaration(_)
        | Expression::Block(_) => false,
        Expression::TypeBinary {
            operator:
                destack_ast::TypeBinaryOperator::Cast | destack_ast::TypeBinaryOperator::Satisfies,
            left,
            ..
        } => {
            let left_id = transparent_inner_expression(ctx, *left);
            matches!(
                ctx.tree.get(left_id),
                Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
            )
        }
        expression => is_trivial_expression(ctx.tree, expression),
    }
}

/// Return whether two grouped-last arguments are the same family and should not use grouped layout.
fn grouped_last_arguments_share_family(
    ctx: &DestackFormatContext<'_>,
    penultimate_id: LocalNodeId<Expression>,
    last_id: LocalNodeId<Expression>,
) -> bool {
    let penultimate_family = grouped_argument_expression_family(ctx, penultimate_id);
    let last_family = grouped_argument_expression_family(ctx, last_id);

    penultimate_family != GroupedArgumentExpressionFamily::Other
        && penultimate_family == last_family
}

/// Return the grouped call-argument layout, if one standard grouped layout applies.
fn grouped_call_argument_layout(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_call_infix_annotations: bool,
    has_any_argument_annotation: bool,
    has_boundary_comments: bool,
) -> Option<GroupedCallArgumentLayout> {
    if has_call_infix_annotations || has_any_argument_annotation || has_boundary_comments {
        return None;
    }

    if dynamic_arguments.len() == 1 {
        let expression_id = argument_expression_id(ctx, dynamic_arguments[0])?;
        if expression_is_function_like_argument(ctx, expression_id) {
            return Some(GroupedCallArgumentLayout::GroupedFirstArgument);
        }

        return None;
    }

    if dynamic_arguments.len() != 2 {
        return None;
    }

    let first_id = argument_expression_id(ctx, dynamic_arguments[0])?;
    let second_id = argument_expression_id(ctx, dynamic_arguments[1])?;

    if expression_is_block_lambda_argument(ctx, first_id)
        && expression_is_relatively_short_group_partner(ctx, second_id)
    {
        return Some(GroupedCallArgumentLayout::GroupedFirstArgument);
    }

    if expression_is_function_like_argument(ctx, first_id) {
        return None;
    }

    if can_group_expression_argument(ctx, second_id)
        && !grouped_last_arguments_share_family(ctx, first_id, second_id)
    {
        return Some(GroupedCallArgumentLayout::GroupedLastArgument);
    }

    None
}

/// Format one grouped call-argument layout.
fn format_grouped_call_argument_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout: GroupedCallArgumentLayout,
) -> FormatResult<()> {
    match (layout, dynamic_arguments) {
        (GroupedCallArgumentLayout::GroupedFirstArgument, [argument_id]) => {
            write!(
                f,
                [group(&format_args![token("("), *argument_id, token(")")])]
            )
        }
        (GroupedCallArgumentLayout::GroupedFirstArgument, [first_id, second_id]) => write!(
            f,
            [group(&format_args![
                token("("),
                *first_id,
                token(","),
                space(),
                *second_id,
                token(")")
            ])]
        ),
        (GroupedCallArgumentLayout::GroupedLastArgument, [first_id, second_id]) => write!(
            f,
            [group(&format_args![
                token("("),
                *first_id,
                token(","),
                space(),
                group(second_id),
                token(")")
            ])]
        ),
        _ => format_default_call_argument_list(
            f,
            f.group_id("call_args_grouped_fallback"),
            dynamic_arguments,
            false,
            false,
        ),
    }
}

/// Return whether an argument can be emitted directly without argument-node formatting.
pub(crate) fn argument_is_plain_call_argument(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    !ctx.has_annotation(argument_id)
        && matches!(
            ctx.tree.get(argument_id),
            Argument::Named {
                modifiers: None,
                ..
            } | Argument::Labeled {
                modifiers: None,
                ..
            } | Argument::Positional {
                modifiers: None,
                ..
            } | Argument::Spread {
                modifiers: None,
                ..
            }
        )
}

/// Write one call argument that is known to be plain.
pub(crate) fn write_plain_call_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(
        f,
        [crate::format::annotation::prefix_annotations(
            f.context(),
            argument_id
        )]
    )?;

    match f.context().tree.get(argument_id) {
        Argument::Named { name, value, .. } => {
            write!(f, [*name, token(":"), space(), *value])?;
        }
        Argument::Labeled { label, value, .. } => {
            write!(f, [*label, token(":"), space(), *value])?;
        }
        Argument::Positional { value, .. } => {
            write!(f, [*value])?;
        }
        Argument::Spread {
            label: Some(label),
            value,
            ..
        } => {
            write!(f, [token("..."), *label, token(":"), space(), *value])?;
        }
        Argument::Spread {
            label: None, value, ..
        } => {
            write!(f, [token("..."), *value])?;
        }
        Argument::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    Ok(())
}

/// Write one call argument with a plain short-circuit and a safe default branch.
pub(crate) fn write_plain_call_argument_or_node<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    if !argument_is_plain_call_argument(f.context(), argument_id) {
        write!(f, [argument_id])?;
        return Ok(());
    }

    write_plain_call_argument(f, argument_id)
}

/// Return whether a call should expand its argument list when formatted in a chain.
pub(crate) fn call_arguments_force_expand_for_chain(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if dynamic_arguments.is_empty() {
        return false;
    }

    let layout_facts = call_argument_layout_facts(ctx, call_node_id, dynamic_arguments);
    let should_bypass_simple_false = dynamic_arguments.len() == 1
        && single_argument_requires_expanded_list(ctx, dynamic_arguments);
    let can_use_simple_false = !layout_facts.has_call_infix_annotations
        && layout_facts.all_compact_simple_unannotated
        && !should_bypass_simple_false;
    if can_use_simple_false {
        return false;
    }

    chain_call_argument_force_expand(ctx, call_node_id, dynamic_arguments)
}

/// Format one single call argument with an active list group id.
pub(crate) fn format_single_call_argument_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
    group_id: GroupId,
) -> FormatResult<()> {
    let single_argument = [argument_id];
    let layout_facts = call_argument_layout_facts(f.context(), call_node_id, &single_argument);
    let call_has_static_arguments = call_has_static_arguments(f.context(), call_node_id);
    let single_argument_force_expand =
        single_argument_requires_expanded_list(f.context(), &single_argument);

    // simple short-circuit path
    let use_single_simple_short_circuit = !layout_facts.has_boundary_comments
        && !call_has_static_arguments
        && !layout_facts.has_call_infix_annotations
        && !single_argument_force_expand
        && layout_facts.all_single_line_and_unannotated
        && {
            if let Some(value_id) = argument_value_id_if_present(f.context().tree, argument_id) {
                let value_id = transparent_inner_expression(f.context(), value_id);
                let value = f.context().tree.get(value_id);
                let value_is_short_empty_call = matches!(
                    value,
                    Expression::Call {
                        static_arguments: None,
                        dynamic_arguments,
                        ..
                    } if dynamic_arguments.is_empty()
                ) && !f.context().has_annotation(value_id);
                is_trivial_expression(f.context().tree, value) || value_is_short_empty_call
            } else {
                false
            }
        };
    if use_single_simple_short_circuit {
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // inline closure cast object path
    if !layout_facts.has_boundary_comments
        && argument_is_inline_closure_cast_object(f.context(), argument_id)
    {
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // use full single argument layout selection and rendering
    format_decided_call_argument_list(
        f,
        call_node_id,
        &single_argument,
        group_id,
        &layout_facts,
        single_argument_force_expand,
    )?;
    Ok(())
}

/// Format call arguments with an active list group id.
pub(crate) fn format_call_arguments_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
) -> FormatResult<()> {
    // empty argument lists can still carry boundary infix annotations
    if dynamic_arguments.is_empty() {
        if f.context().has_infix_annotation(call_node_id) {
            let should_expand_multiline =
                empty_call_infix_requires_multiline(f.context(), call_node_id);
            if should_expand_multiline {
                write!(
                    f,
                    [
                        token("("),
                        block_indent(&crate::format::annotation::block_infix_annotations(
                            f.context(),
                            call_node_id
                        )),
                        token(")")
                    ]
                )?;
            } else {
                write!(
                    f,
                    [
                        token("("),
                        crate::format::annotation::block_infix_annotations(
                            f.context(),
                            call_node_id
                        ),
                        token(")")
                    ]
                )?;
            }
        } else {
            write!(f, [token("("), token(")")])?;
        }

        return Ok(());
    }

    // ignore ranges: route through separated entries so raw span preservation stays consistent
    if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        let has_ignore_ranges =
            any_ignore_range_for_nodes(f.context(), dynamic_arguments, comment_tokens);
        if has_ignore_ranges {
            write!(
                f,
                [group(&format_args![
                    token("("),
                    soft_block_indent(&separated_entries(
                        ",",
                        dynamic_arguments,
                        TrailingSeparator::Omit,
                        Some(group_id),
                    )),
                    token(")")
                ])
                .with_id(Some(group_id))
                .should_expand(true)]
            )?;
            return Ok(());
        }
    }

    // single argument path has dedicated short-circuit and hugging logic
    if dynamic_arguments.len() == 1 {
        return format_single_call_argument_with_group(
            f,
            call_node_id,
            dynamic_arguments[0],
            group_id,
        );
    }

    // multi argument path: scan once, choose layout, then render
    let layout_facts = call_argument_layout_facts(f.context(), call_node_id, dynamic_arguments);
    format_decided_call_argument_list(
        f,
        call_node_id,
        dynamic_arguments,
        group_id,
        &layout_facts,
        false,
    )
}

/// Return whether empty call infix annotations should expand across lines.
fn empty_call_infix_requires_multiline(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    ctx.annotation_ids(call_node_id)
        .iter()
        .any(|annotation_id| {
            let annotation = ctx.annotation(*annotation_id);
            if annotation.position() != AnnotationPosition::BlockInfix {
                return false;
            }

            let annotation_span = ctx.annotation_span(*annotation_id);
            if ctx.has_newline(annotation_span) {
                return true;
            }

            match annotation {
                Annotation::Doc { .. } => true,
                Annotation::Decorator { .. } => false,
            }
        })
}

/// Format call arguments with list-group awareness.
pub(crate) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    // set up the list group id for conditional formatting
    let group_id = f.group_id("call_args");
    let previous_group_id = f.context().current_argument_group_id;
    f.context_mut().current_argument_group_id = Some(group_id);
    let result = format_call_arguments_with_group(f, call_node_id, dynamic_arguments, group_id);
    f.context_mut().current_argument_group_id = previous_group_id;

    result
}

/// Format a call expression.
#[inline]
pub(crate) fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Call {
        position,
        left,
        static_arguments,
        dynamic_arguments,
    } = f.context().tree.get(node_id)
    {
        let call_parent_is_decorator =
            f.context().parent(node_id).is_some_and(|(_, parent_type)| {
                matches!(parent_type, NodeType::Decorator | NodeType::Annotation)
            });
        if call_parent_is_decorator {
            let left_expression = f.context().tree.get(*left);
            let left_is_ignored = node_has_ignore_directive(f.context(), *left);
            format_expression(f, *left, left_expression, left_is_ignored)?;
        } else {
            write!(f, [*left])?;
        }
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(static_arguments) = static_arguments {
            format_static_argument_list(f, static_arguments)?;
        }

        format_call_arguments(f, node_id, dynamic_arguments)?;
    } else {
        debug_assert!(false, "unexpected expression kind for call formatter");
    }
    Ok(())
}

/// Format an instantiation expression.
#[inline]
pub(crate) fn format_instantiation_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Instantiation {
        left,
        static_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        format_static_argument_list(f, static_arguments)?;
    } else {
        debug_assert!(
            false,
            "unexpected expression kind for instantiation formatter"
        );
    }
    Ok(())
}

/// Write one single argument wrapped in call parentheses.
pub(crate) fn write_single_call_argument_inline_wrapped<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    write_plain_call_argument_or_node(f, argument_id)?;
    write!(f, [token(")")])
}

/// Format call arguments with the default list formatter.
fn format_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    disallow_trailing_separator: bool,
) -> FormatResult<()> {
    let trailing_separator = if disallow_trailing_separator {
        TrailingSeparator::Omit
    } else {
        match f.context().options.trailing_comma {
            destack_workspace::TrailingComma::All => TrailingSeparator::Allowed,
            destack_workspace::TrailingComma::Es5 | destack_workspace::TrailingComma::None => {
                TrailingSeparator::Omit
            }
        }
    };

    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&separated_entries(
                ",",
                dynamic_arguments,
                trailing_separator,
                Some(group_id),
            )),
            token(")")
        ])
        .with_id(Some(group_id))
        .should_expand(force_expand)]
    )
}

/// Decide and render one call argument list directly.
pub(crate) fn format_decided_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
    layout_facts: &CallArgumentLayoutFacts,
    single_argument_force_expand: bool,
) -> FormatResult<()> {
    // grouped standard layouts
    if let Some(layout) = grouped_call_argument_layout(
        f.context(),
        dynamic_arguments,
        layout_facts.has_call_infix_annotations,
        layout_facts.has_any_argument_annotation,
        layout_facts.has_boundary_comments,
    ) {
        return format_grouped_call_argument_layout(f, dynamic_arguments, layout);
    }

    if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];

        if !single_argument_force_expand
            && !layout_facts.has_any_argument_annotation
            && argument_is_trivial_unannotated_non_lambda_value(f.context(), argument_id)
        {
            return write_single_call_argument_inline_wrapped(f, argument_id);
        }

        if !layout_facts.has_call_infix_annotations
            && !layout_facts.has_boundary_comments
            && !layout_facts.has_any_argument_annotation
            && argument_is_compact_inline_callback(f.context(), argument_id)
        {
            return write_single_call_argument_inline_wrapped(f, argument_id);
        }
    }
    let (force_expand_regular, trailing_collection_argument) = if dynamic_arguments.is_empty() {
        (false, false)
    } else if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];
        let has_line_comments = argument_has_line_comment(f.context(), argument_id);
        let trailing_collection_argument = argument_is_collection_literal(f.context(), argument_id);
        let has_collection_source_comment = trailing_collection_argument && {
            let argument_span = f.context().span(argument_id);
            !f.context()
                .comments_in_range(argument_span.start, argument_span.end)
                .is_empty()
        };
        let has_line_comments = has_line_comments || has_collection_source_comment;

        let force_expand_jsx = has_multiline_jsx_argument(f.context(), dynamic_arguments);
        let force_expand_single_commented_callback =
            argument_is_block_callback(f.context(), argument_id)
                && (argument_has_callback_blocking_comment(f.context(), argument_id)
                    || layout_facts.has_call_infix_annotations);
        let force_expand_single_prefix_line_commented_argument =
            argument_has_prefix_line_comment(f.context(), argument_id);

        (
            force_expand_jsx
                || has_line_comments
                || force_expand_single_commented_callback
                || single_argument_force_expand
                || force_expand_single_prefix_line_commented_argument
                || layout_facts.has_call_infix_annotations,
            trailing_collection_argument,
        )
    } else if !layout_facts.has_call_infix_annotations
        && layout_facts.all_compact_simple_unannotated
    {
        (false, layout_facts.trailing_collection_argument)
    } else {
        let force_expand_jsx = has_multiline_jsx_argument(f.context(), dynamic_arguments);
        let force_expand_callback_with_collection_tail = layout_facts.trailing_collection_argument
            && dynamic_arguments[..dynamic_arguments.len().saturating_sub(1)]
                .iter()
                .copied()
                .any(|argument_id| argument_is_block_callback(f.context(), argument_id));

        let has_multiple_function_arguments =
            layout_facts.arrow_argument_count >= 2 || layout_facts.function_argument_count >= 2;
        if has_multiple_function_arguments {
            (true, layout_facts.trailing_collection_argument)
        } else {
            (
                force_expand_jsx
                    || layout_facts.has_complex_non_callback_argument
                    || layout_facts.has_line_comments
                    || force_expand_callback_with_collection_tail
                    || has_multiple_function_arguments
                    || layout_facts.has_call_infix_annotations,
                layout_facts.trailing_collection_argument,
            )
        }
    };
    let trailing_collection_comment_force_expand = dynamic_arguments.len() > 1
        && trailing_collection_argument
        && dynamic_arguments
            .last()
            .copied()
            .is_some_and(|last_argument_id| {
                let has_last_line_comment = layout_facts.has_line_comments
                    && argument_has_line_comment(f.context(), last_argument_id);
                let has_last_source_comment = {
                    let argument_span = f.context().span(last_argument_id);
                    !f.context()
                        .comments_in_range(argument_span.start, argument_span.end)
                        .is_empty()
                };
                has_last_line_comment || has_last_source_comment
            });
    let force_expand = force_expand_regular
        || layout_facts.has_boundary_comments
        || trailing_collection_comment_force_expand;
    format_default_call_argument_list(
        f,
        group_id,
        dynamic_arguments,
        force_expand,
        dynamic_arguments
            .first()
            .copied()
            .is_some_and(|argument_id| {
                argument_is_template_literal(f.context(), argument_id)
                    && !argument_is_interpolated_template_literal(f.context(), argument_id)
            }),
    )
}

/// Return whether an argument has a prefix annotation.
fn argument_has_prefix_annotation(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.annotation_ids(argument_id)
        .iter()
        .copied()
        .any(|annotation_id| match ctx.annotation(annotation_id) {
            Annotation::Doc { position, .. } | Annotation::Decorator { position, .. } => matches!(
                position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ),
        })
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        node_id: LocalNodeId<Argument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if argument_is_plain_call_argument(f.context(), node_id) {
            write_plain_call_argument(f, node_id)?;
            return Ok(());
        }

        let has_lambda_value = argument_contains_lambda_value(f.context(), self);

        if has_lambda_value || argument_has_prefix_annotation(f.context(), node_id) {
            write!(
                f,
                [crate::format::annotation::prefix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        }

        write_argument_with_modifiers_and_value(self, false, f)?;

        write!(
            f,
            [crate::format::annotation::infix_or_postfix_annotations(
                f.context(),
                node_id
            )]
        )?;

        Ok(())
    }
}

/// Return whether this argument wraps a lambda declaration expression.
fn argument_contains_lambda_value(ctx: &DestackFormatContext<'_>, argument: &Argument) -> bool {
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => return false,
    };

    let Expression::Declaration(declaration_id) = ctx.tree.get(value_id) else {
        return false;
    };

    matches!(
        ctx.tree.get(*declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

/// Write one argument with modifiers and value payload.
fn write_argument_with_modifiers_and_value<'ast>(
    argument: &Argument,
    force_break_after_lambda_prefix_comment: bool,
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    match argument {
        Argument::Named {
            modifiers,
            name,
            value,
        } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;
            write!(f, [name])?;
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            write!(f, [token(":"), space(), value])?;
        }
        Argument::Labeled {
            modifiers,
            label,
            value,
        } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;
            write!(f, [label])?;
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            write!(f, [token(":"), space(), value])?;
        }
        Argument::Positional { modifiers, value } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;

            if force_break_after_lambda_prefix_comment {
                write!(f, [hard_line_break()])?;
            }

            write!(f, [value])?;
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;
        }
        Argument::Spread {
            modifiers,
            label,
            value,
        } => {
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;
            write!(f, [token("...")])?;

            if let Some(label) = label {
                write!(f, [label])?;
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                write!(f, [token(":"), space(), value])?;
            } else {
                if force_break_after_lambda_prefix_comment {
                    write!(f, [hard_line_break()])?;
                }

                write!(f, [value])?;
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
            }
        }
        Argument::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    }

    Ok(())
}
