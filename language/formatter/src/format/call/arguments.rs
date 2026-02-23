use crate::FormatNode;
use crate::format::analysis::{
    argument_has_leading_prefix_annotation_outside_span, argument_has_multiline_prefix_annotation,
    argument_has_separator_line_comment_annotation, argument_is_inline_closure_cast_object,
    argument_is_interpolated_template_literal, call_arguments_have_boundary_comments,
    call_arguments_preserve_blank_line_between, call_has_static_arguments,
    next_non_whitespace_token_after_annotation, previous_non_whitespace_token_before_annotation,
    previous_non_whitespace_token_before_span, timing,
};
use crate::format::call::layout::{
    CallArgumentLayout, argument_has_callback_blocking_comment_annotation, call_argument_layout,
    call_argument_layout_cache, call_force_expand_single_collection_for_type_binary_callee,
    call_force_expand_single_multiline_with_static_arguments, call_has_await_ancestor,
    chain_call_argument_force_expand, single_argument_requires_expanded_list,
};
use crate::format::collection::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::format::directive::any_ignore_range_for_nodes;
use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, Declaration, DestackFormatContext, DestackFormatter,
    Expression, FormatResult, FunctionKind, GroupId, HugOptions, LocalNodeId, NodeType, Span,
    TokenType, TrailingComma, TypeBinaryOperator, argument_is_function_expression,
    argument_is_lambda_expression, argument_value_id, block_indent, empty_line,
    format_block_of_properties, format_hugged, format_static_argument_list, format_with, group,
    hard_line_break, if_group_breaks, is_trivial_expression, list_like, soft_block_indent,
    soft_line_break_or_space, space, token, transparent_inner_expression,
};
use destack_ast::{Comment, CommentStyle, PostfixPosition, TokenSpan};
use destack_fir::format::{Buffer, Format};
use destack_fir::write;

/// Write an inline comma-separated call argument list.
pub(crate) fn write_inline_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    write!(f, [token("(")])?;

    for (index, argument_id) in dynamic_arguments.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        write_call_argument_for_list(f, *argument_id, all_plain_call_arguments)?;
    }

    write!(f, [token(")")])?;

    Ok(())
}

/// Return whether an argument can be emitted directly without argument-node formatting.
pub(crate) fn argument_is_plain_call_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if let Some(cached) = context.lookup_argument_plain_call_argument(argument_id) {
        context.increment_counter("call.arguments.plain.cache.hits", 1);
        return cached;
    }
    context.increment_counter("call.arguments.plain.cache.misses", 1);

    let is_plain = !context.has_annotation(argument_id)
        && matches!(
            context.tree.get(argument_id),
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
        );
    context.store_argument_plain_call_argument(argument_id, is_plain);
    is_plain
}

/// Write one call argument that is known to be plain.
pub(crate) fn write_plain_call_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
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

/// Write one argument in list context from the chosen plain-call argument mode.
fn write_call_argument_for_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    if all_plain_call_arguments {
        return write_plain_call_argument(f, argument_id);
    }

    write_plain_call_argument_or_node(f, argument_id)
}

/// Return whether a call should expand its argument list when formatted in a chain.
pub(crate) fn call_arguments_force_expand_for_chain(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(force_expand) = context.lookup_call_argument_chain_force_expand(call_node_id) {
        context.increment_counter("call.arguments.chain.simple_cache.hits", 1);
        context.increment_counter(
            if force_expand {
                "call.arguments.chain.force_expand.true"
            } else {
                "call.arguments.chain.force_expand.false"
            },
            1,
        );
        return force_expand;
    }
    context.increment_counter("call.arguments.chain.simple_cache.misses", 1);

    if dynamic_arguments.is_empty() {
        context.store_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("call.arguments.chain.simple_false.empty", 1);
        context.increment_counter("call.arguments.chain.force_expand.false", 1);
        return false;
    }

    let layout_cache = call_argument_layout_cache(context, call_node_id, dynamic_arguments);
    let should_bypass_simple_false = dynamic_arguments.len() == 1
        && single_argument_requires_expanded_list(context, dynamic_arguments);
    let can_use_simple_false = !layout_cache.has_call_infix_annotations
        && layout_cache.all_compact_simple_unannotated
        && !should_bypass_simple_false;
    if can_use_simple_false {
        context.store_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("call.arguments.chain.simple_false.simple", 1);
        context.increment_counter("call.arguments.chain.force_expand.false", 1);
        return false;
    }

    let force_expand = chain_call_argument_force_expand(context, call_node_id, dynamic_arguments);
    context.store_call_argument_chain_force_expand(call_node_id, force_expand);
    force_expand
}

/// Format one single call argument with an active list group id.
pub(crate) fn format_single_call_argument_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
    group_id: GroupId,
) -> FormatResult<()> {
    let single_argument = [argument_id];
    let has_boundary_comments =
        call_arguments_have_boundary_comments(f.context(), call_node_id, &single_argument);
    let separator_line_comment_source =
        plain_single_argument_separator_line_comment_source(f.context(), call_node_id, argument_id);
    let layout_cache = call_argument_layout_cache(f.context(), call_node_id, &single_argument);
    let has_call_infix_annotations = layout_cache.has_call_infix_annotations;
    let has_any_argument_annotation = layout_cache.has_any_argument_annotation;
    let call_has_static_arguments = call_has_static_arguments(f.context(), call_node_id);
    let single_argument_force_expand =
        single_argument_requires_expanded_list(f.context(), &single_argument);
    let force_expand_single_multiline_with_static_arguments =
        call_force_expand_single_multiline_with_static_arguments(
            f.context(),
            call_node_id,
            &single_argument,
        );
    let force_expand_single_collection_for_type_binary_callee =
        call_force_expand_single_collection_for_type_binary_callee(
            f.context(),
            call_node_id,
            &single_argument,
        );
    let has_hug_blocking_comment_annotation =
        argument_has_callback_blocking_comment_annotation(f.context(), argument_id);

    // separator comment path
    if let Some(comment_source) = separator_line_comment_source.as_ref() {
        format_single_plain_argument_with_separator_line_comment(f, argument_id, comment_source)?;
        return Ok(());
    }

    // simple short-circuit path
    let use_single_simple_short_circuit = !has_boundary_comments
        && !call_has_static_arguments
        && !has_call_infix_annotations
        && !single_argument_force_expand
        && !layout_cache.is_multiline_in_source
        && layout_cache.all_single_line_and_unannotated
        && {
            let value_id = argument_value_id(f.context().tree, argument_id);
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
        };
    if use_single_simple_short_circuit {
        f.context()
            .increment_counter("call.arguments.single_simple.short_circuit", 1);
        f.context()
            .increment_counter("call.arguments.path.single_simple.short_circuit", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // inline closure cast object path
    if !has_boundary_comments && argument_is_inline_closure_cast_object(f.context(), argument_id) {
        f.context()
            .increment_counter("call.arguments.path.inline_closure_cast_object", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // function inline path
    let use_single_function_argument_inline = !has_call_infix_annotations
        && !force_expand_single_multiline_with_static_arguments
        && !force_expand_single_collection_for_type_binary_callee
        && !has_boundary_comments
        && {
            let value_id = argument_value_id(f.context().tree, argument_id);
            !f.context().has_non_blank_annotation(argument_id)
                && !f.context().has_non_blank_annotation(value_id)
                && !argument_has_callback_blocking_comment_annotation(f.context(), argument_id)
                && !argument_has_leading_prefix_annotation_outside_span(f.context(), argument_id)
                && argument_is_function_expression(f.context(), argument_id)
        };
    if use_single_function_argument_inline {
        f.context()
            .increment_counter("call.arguments.path.single_function_inline", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // callback inline path
    let use_single_callback_argument_inline = !has_call_infix_annotations
        && !call_has_await_ancestor(f.context(), call_node_id)
        && !force_expand_single_multiline_with_static_arguments
        && !force_expand_single_collection_for_type_binary_callee
        && !has_boundary_comments
        && {
            let value_id = argument_value_id(f.context().tree, argument_id);
            !f.context().has_non_blank_annotation(argument_id)
                && !f.context().has_non_blank_annotation(value_id)
                && !argument_has_callback_blocking_comment_annotation(f.context(), argument_id)
                && !argument_has_leading_prefix_annotation_outside_span(f.context(), argument_id)
                && argument_is_lambda_expression(f.context(), argument_id)
        };
    if use_single_callback_argument_inline {
        f.context()
            .increment_counter("call.arguments.path.single_callback_inline", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // hugged path
    if !has_boundary_comments
        && !has_any_argument_annotation
        && !argument_has_multiline_prefix_annotation(f.context(), argument_id)
        && !has_hug_blocking_comment_annotation
        && !has_call_infix_annotations
        && !force_expand_single_multiline_with_static_arguments
        && !argument_is_lambda_expression(f.context(), argument_id)
        && !argument_is_function_expression(f.context(), argument_id)
        && !argument_is_interpolated_template_literal(f.context(), argument_id)
    {
        let force_hugged_expand = force_expand_single_collection_for_type_binary_callee;
        let used_hugged = format_hugged(
            f,
            &single_argument,
            HugOptions::CALL,
            Some(group_id),
            force_hugged_expand,
        )?;
        if used_hugged {
            f.context()
                .increment_counter("call.arguments.path.hugged", 1);
            return Ok(());
        }
    }

    // use full single argument layout selection and rendering
    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT);
    let layout = {
        let _timing = f
            .context()
            .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT_DECIDE);
        call_argument_layout(
            f.context(),
            call_node_id,
            &single_argument,
            single_argument_force_expand,
            force_expand_single_multiline_with_static_arguments,
            force_expand_single_collection_for_type_binary_callee,
            has_boundary_comments,
        )
    };

    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT_RENDER);
    format_call_argument_layout(
        f,
        &single_argument,
        layout,
        call_node_id,
        group_id,
        argument_is_plain_call_argument(f.context(), argument_id),
    )
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
        return format_empty_call_arguments(f, call_node_id);
    }

    // ignore ranges: route through list_like so raw span preservation stays consistent
    if try_format_ignore_range_call_arguments(f, dynamic_arguments, group_id)? {
        return Ok(());
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
    let layout_cache = call_argument_layout_cache(f.context(), call_node_id, dynamic_arguments);
    let all_plain_call_arguments = layout_cache.all_plain_call_arguments;
    let has_boundary_comments =
        call_arguments_have_boundary_comments(f.context(), call_node_id, dynamic_arguments);

    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT);
    let layout = {
        let _timing = f
            .context()
            .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT_DECIDE);
        call_argument_layout(
            f.context(),
            call_node_id,
            dynamic_arguments,
            false,
            false,
            false,
            has_boundary_comments,
        )
    };

    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT_RENDER);
    format_call_argument_layout(
        f,
        dynamic_arguments,
        layout,
        call_node_id,
        group_id,
        all_plain_call_arguments,
    )
}

/// Format an empty call argument list, preserving infix annotations when present.
fn format_empty_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if f.context().has_infix_annotation(call_node_id) {
        let should_expand_multiline =
            empty_call_infix_requires_multiline(f.context(), call_node_id);
        if should_expand_multiline {
            write!(
                f,
                [
                    token("("),
                    block_indent(&f.context().block_infix_annotations(call_node_id)),
                    token(")")
                ]
            )?;
        } else {
            write!(
                f,
                [
                    token("("),
                    f.context().block_infix_annotations(call_node_id),
                    token(")")
                ]
            )?;
        }
    } else {
        write!(f, [token("("), token(")")])?;
    }

    Ok(())
}

/// Try formatting call arguments through the ignore-range list writer.
fn try_format_ignore_range_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
) -> FormatResult<bool> {
    if !f.context().has_ignore_directive_markers() {
        return Ok(false);
    }

    let comment_tokens = f.context().comment_tokens();
    let has_ignore_ranges =
        any_ignore_range_for_nodes(f.context(), dynamic_arguments, comment_tokens);
    if !has_ignore_ranges {
        return Ok(false);
    }

    f.context()
        .increment_counter("call.arguments.path.ignore_ranges_list_like", 1);
    let mut list = list_like("(", ")", ",", dynamic_arguments);
    list.with_group_id(Some(group_id)).force_expand();
    write!(f, [list])?;

    Ok(true)
}

/// Return whether empty call infix annotations should expand across lines.
fn empty_call_infix_requires_multiline(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(call_node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                if annotation.position() != AnnotationPosition::BlockInfix {
                    return false;
                }

                let annotation_span = context.annotation_span(*annotation_id);
                if context.has_newline(annotation_span) {
                    return true;
                }

                match annotation {
                    Annotation::Comment { node, .. } => {
                        let comment = context.tree.get::<Comment>(node);
                        comment.style == CommentStyle::Slash
                    }
                    Annotation::Blank { .. } | Annotation::Doc { .. } => true,
                    Annotation::Decorator { .. } => false,
                }
            })
        })
        .unwrap_or(false)
}

/// Format call arguments with list-group awareness.
pub(crate) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS);

    // set up the list group id for conditional formatting
    let group_id = f.group_id("call_args");
    let previous_group_id = f.context().current_argument_group_id;
    f.context_mut().current_argument_group_id = Some(group_id);
    let result = format_call_arguments_with_group(f, call_node_id, dynamic_arguments, group_id);
    f.context_mut().current_argument_group_id = previous_group_id;

    result
}

/// Store one separator line comment and whether it started on its own source line.
pub(crate) struct SeparatorLineCommentSource {
    /// The separator comment node ids in source order.
    pub(crate) comment_ids: Vec<LocalNodeId<Comment>>,
    /// Whether the comment starts on its own line after the separator comma.
    pub(crate) is_own_line: bool,
}

/// Return one source comma token that owns one separator slash comment seam.
fn separator_line_comment_preceding_comma(
    context: &DestackFormatContext<'_>,
    annotation_span: Span,
) -> Option<TokenSpan> {
    let mut previous_token = previous_non_whitespace_token_before_span(context, annotation_span);

    while let Some(token) = previous_token {
        if token.token.ty == TokenType::Comma {
            return Some(token);
        }

        if matches!(
            token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            previous_token = previous_non_whitespace_token_before_span(context, token.span);
            continue;
        }

        return None;
    }

    None
}

/// Return one separator slash comment annotation source payload.
fn separator_line_comment_annotation_info(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<(LocalNodeId<Comment>, bool)> {
    let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
        return None;
    };

    if !matches!(
        position,
        AnnotationPosition::LinePrefix
            | AnnotationPosition::LinePostfix
            | AnnotationPosition::LinePostfixBoundary
            | AnnotationPosition::BlockPostfix
    ) {
        return None;
    }

    let comment = context.tree.get::<Comment>(node);
    if comment.style != CommentStyle::Slash {
        return None;
    }

    let annotation_span = context.annotation_span(annotation_id);
    let preceding_comma = separator_line_comment_preceding_comma(context, annotation_span);
    let following_token = next_non_whitespace_token_after_annotation(context, annotation_id);
    let following_separator =
        following_token.is_some_and(|token| token.token.ty == TokenType::Comma);
    let following_close_brace =
        following_token.is_some_and(|token| token.token.ty == TokenType::CloseBrace);

    if preceding_comma.is_none() && !following_separator {
        return None;
    }

    if preceding_comma.is_some() && following_close_brace && !following_separator {
        return None;
    }

    let is_own_line = separator_line_comment_is_own_line(
        context,
        annotation_id,
        annotation_span,
        preceding_comma,
        following_separator,
    );

    Some((node, is_own_line))
}

/// Return whether one separator comment starts on its own source line.
fn separator_line_comment_is_own_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    annotation_span: Span,
    preceding_comma: Option<TokenSpan>,
    following_separator: bool,
) -> bool {
    if let Some(separator_token) = preceding_comma {
        let before_comment_span = Span::new(
            annotation_span.file,
            separator_token.span.end,
            annotation_span.start,
        );

        return context.has_newline(before_comment_span);
    }

    if following_separator {
        let Some(separator_token) =
            next_non_whitespace_token_after_annotation(context, annotation_id)
        else {
            return false;
        };

        let after_comment_span = Span::new(
            annotation_span.file,
            annotation_span.end,
            separator_token.span.start,
        );
        return context.has_newline(after_comment_span);
    }

    false
}

/// Return whether one annotation is a separator line comment.
/// Return one separator comment source from one annotation list and filter.
fn separator_line_comment_source_from_annotations<F>(
    context: &DestackFormatContext<'_>,
    annotations: &[LocalNodeId<Annotation>],
    mut annotation_allowed: F,
) -> Option<SeparatorLineCommentSource>
where
    F: FnMut(LocalNodeId<Annotation>) -> bool,
{
    for (index, annotation_id) in annotations.iter().copied().enumerate() {
        if !annotation_allowed(annotation_id) {
            continue;
        }

        let Some((comment_id, is_own_line)) =
            separator_line_comment_annotation_info(context, annotation_id)
        else {
            continue;
        };

        let mut comment_ids = vec![comment_id];
        for next_annotation_id in annotations.iter().skip(index + 1).copied() {
            if !annotation_allowed(next_annotation_id) {
                break;
            }

            if matches!(
                context.annotation(next_annotation_id),
                Annotation::Blank { .. }
            ) {
                continue;
            }

            let Some((next_comment_id, _)) =
                separator_line_comment_annotation_info(context, next_annotation_id)
            else {
                break;
            };

            comment_ids.push(next_comment_id);
        }

        return Some(SeparatorLineCommentSource {
            comment_ids,
            is_own_line,
        });
    }

    None
}

/// Return one separator line comment source attached to one argument.
pub(crate) fn single_argument_separator_line_comment_source(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> Option<SeparatorLineCommentSource> {
    let resolve_from_annotations = |annotations: &[LocalNodeId<Annotation>]| {
        separator_line_comment_source_from_annotations(context, annotations, |_| true)
    };

    if let Some(annotations) = context.annotations(argument_id)
        && let Some(comment_source) = resolve_from_annotations(&annotations)
    {
        return Some(comment_source);
    }

    let value_id = argument_value_id(context.tree, argument_id);
    let value_annotation_source = context
        .annotations(value_id)
        .and_then(|annotations| resolve_from_annotations(&annotations));
    if value_annotation_source.is_some() {
        return value_annotation_source;
    }

    let argument_span = context.span(argument_id);
    let value_span = context.span(value_id);
    let seam_start = value_span.end;
    let call_span = context.span(call_node_id);
    if value_span.file != call_span.file || seam_start >= call_span.end {
        return None;
    }

    let call_annotation_source = context.annotations(call_node_id).and_then(|annotations| {
        separator_line_comment_source_from_annotations(context, &annotations, |annotation_id| {
            let annotation_span = context.annotation_span(annotation_id);
            annotation_span.file == argument_span.file
                && annotation_span.start >= seam_start
                && annotation_span.end <= call_span.end
        })
    });
    if call_annotation_source.is_some() {
        return call_annotation_source;
    }

    None
}

/// Return one separator line comment source only when the argument uses plain argument rendering.
fn plain_single_argument_separator_line_comment_source(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> Option<SeparatorLineCommentSource> {
    if !argument_is_plain_call_argument(context, argument_id) {
        return None;
    }

    single_argument_separator_line_comment_source(context, call_node_id, argument_id)
}

/// Return whether an argument has non-separator postfix or infix annotations.
fn argument_has_non_separator_postfix_or_infix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .visit_annotations(argument_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let position = context.annotation(*annotation_id).position();
                let is_postfix_or_infix = matches!(
                    position,
                    AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                );
                is_postfix_or_infix
                    && !matches!(context.annotation(*annotation_id), Annotation::Blank { .. })
                    && separator_line_comment_annotation_info(context, *annotation_id).is_none()
            })
        })
        .unwrap_or(false)
}

/// Return whether one argument can render without separator line comment annotations.
pub(crate) fn argument_can_render_without_separator_line_comment(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_has_separator_line_comment_annotation(context, argument_id) {
        return false;
    }

    if argument_has_non_separator_postfix_or_infix_annotation(context, argument_id) {
        return false;
    }

    matches!(
        context.tree.get(argument_id),
        Argument::Positional {
            modifiers: None,
            ..
        }
    )
}

/// Return whether the list can use multiline trailing separator comment rendering.
pub(crate) fn can_format_multiline_call_argument_list_with_last_separator_line_comment(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };
    let Some(_comment_source) =
        single_argument_separator_line_comment_source(context, call_node_id, last_argument_id)
    else {
        return false;
    };

    argument_can_render_without_separator_line_comment(context, last_argument_id)
}

/// Format a multiline call list with a trailing separator line comment after the last argument.
pub(crate) fn format_multiline_call_argument_list_with_last_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<bool> {
    if !can_format_multiline_call_argument_list_with_last_separator_line_comment(
        f.context(),
        call_node_id,
        dynamic_arguments,
    ) {
        return Ok(false);
    }

    let last_argument_id = dynamic_arguments
        .last()
        .copied()
        .expect("last argument should exist when multiline separator layout is eligible");
    let comment_source =
        single_argument_separator_line_comment_source(f.context(), call_node_id, last_argument_id)
            .expect(
                "separator comment source should exist when multiline separator layout is eligible",
            );
    if !argument_can_render_without_separator_line_comment(f.context(), last_argument_id) {
        return Ok(false);
    }

    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                    if index > 0 {
                        let left_argument_id = dynamic_arguments[index - 1];
                        if call_arguments_preserve_blank_line_between(
                            f.context(),
                            left_argument_id,
                            *argument_id,
                        ) {
                            write!(f, [empty_line()])?;
                        } else {
                            write!(f, [hard_line_break()])?;
                        }
                    }

                    let is_last_argument = index + 1 == dynamic_arguments.len();
                    if is_last_argument {
                        write_argument_without_separator_line_comment(f, *argument_id)?;
                        write_separator_line_comment_after_comma(f, &comment_source)?;
                    } else {
                        write!(f, [group(argument_id), token(",")])?;
                    }
                }
                Ok(())
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token(")")])?;

    Ok(true)
}

/// Write one separator line comment after one argument comma.
pub(crate) fn write_separator_line_comment_after_comma<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    separator_comment_source: &SeparatorLineCommentSource,
) -> FormatResult<()> {
    let Some(first_comment_id) = separator_comment_source.comment_ids.first().copied() else {
        return Ok(());
    };

    if separator_comment_source.is_own_line {
        write!(f, [token(","), hard_line_break(), first_comment_id])?;
        for comment_id in separator_comment_source.comment_ids.iter().skip(1) {
            write!(f, [hard_line_break(), comment_id])?;
        }
    } else {
        write!(f, [token(","), space(), first_comment_id])?;
        for comment_id in separator_comment_source.comment_ids.iter().skip(1) {
            write!(f, [hard_line_break(), comment_id])?;
        }
    }

    Ok(())
}

/// Write one argument without separator line comments that are emitted at list level.
pub(crate) fn write_argument_without_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<bool> {
    if !argument_can_render_without_separator_line_comment(f.context(), argument_id) {
        return Ok(false);
    }

    let Argument::Positional {
        modifiers: None,
        value,
    } = f.context().tree.get(argument_id)
    else {
        return Ok(false);
    };

    write!(
        f,
        [
            f.context().any_prefix_annotations(argument_id),
            group(value)
        ]
    )?;
    Ok(true)
}

/// Format one single plain argument with a separator line comment seam.
pub(crate) fn format_single_plain_argument_with_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    separator_comment_source: &SeparatorLineCommentSource,
) -> FormatResult<()> {
    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                write_plain_call_argument(f, argument_id)?;
                write_separator_line_comment_after_comma(f, separator_comment_source)?;
                Ok(())
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token(")")])?;

    Ok(())
}

/// Format a call expression.
#[inline]
pub(crate) fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f.context().timing_scope(timing::FORMAT_EXPRESSION_CALL);

    if let Expression::Call {
        position,
        left,
        static_arguments,
        dynamic_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
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

/// Write one trailing collection argument while forcing its value expression to expand.
fn write_collection_argument_with_expanded_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    let write_expanded_value =
        |f: &mut DestackFormatter<'ast, '_>, value: LocalNodeId<Expression>| -> FormatResult<()> {
            match f.context().tree.get(value) {
                Expression::ObjectExpression {
                    ty: None,
                    properties,
                } => write!(
                    f,
                    [
                        token("{"),
                        hard_line_break(),
                        block_indent(&format_with(|f| format_block_of_properties(
                            f, properties, ","
                        ))),
                        hard_line_break(),
                        token("}")
                    ]
                ),
                Expression::ArrayExpression { elements } => {
                    let mut list = list_like("[", "]", ",", elements);
                    list.should_expand(true);
                    write!(f, [list])
                }
                _ => write!(f, [group(&value).should_expand(true)]),
            }
        };

    match f.context().tree.get(argument_id) {
        Argument::Named {
            modifiers: None,
            name,
            value,
        } => {
            write!(f, [*name, token(":"), space()])?;
            write_expanded_value(f, *value)
        }
        Argument::Labeled {
            modifiers: None,
            label,
            value,
        } => {
            write!(f, [*label, token(":"), space()])?;
            write_expanded_value(f, *value)
        }
        Argument::Positional {
            modifiers: None,
            value,
        } => write_expanded_value(f, *value),
        Argument::Spread {
            modifiers: None,
            label: None,
            value,
        } => {
            write!(f, [token("...")])?;
            write_expanded_value(f, *value)
        }
        Argument::Spread {
            modifiers: None,
            label: Some(label),
            value,
        } => {
            write!(f, [token("..."), *label, token(":"), space()])?;
            write_expanded_value(f, *value)
        }
        _ => write!(f, [group(&argument_id).should_expand(true)]),
    }
}

/// Format one expanded call argument list with compact leading arguments and a trailing collection.
fn format_trailing_collection_hug_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let Some((&last_argument_id, leading_arguments)) = dynamic_arguments.split_last() else {
        write!(f, [token("("), token(")")])?;
        return Ok(());
    };

    write!(f, [token("(")])?;

    for (index, argument_id) in leading_arguments.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }

        write_call_argument_for_list(f, argument_id, all_plain_call_arguments)?;
    }

    if !leading_arguments.is_empty() {
        write!(f, [token(","), space()])?;
    }

    write_collection_argument_with_expanded_value(f, last_argument_id)?;
    write!(f, [token(")")])?;

    Ok(())
}

/// Format plain default call arguments directly when all argument separators are stable.
fn format_plain_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let allow_trailing_comma = f.context().options.trailing_comma == TrailingComma::All;

    let body = format_with(|f| {
        for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(","), soft_line_break_or_space()])?;
            }

            write_call_argument_for_list(f, argument_id, all_plain_call_arguments)?;
        }

        if allow_trailing_comma {
            write!(f, [if_group_breaks(&token(","))])?;
        }

        Ok(())
    });

    let content = format_with(|f| write!(f, [token("("), soft_block_indent(&body), token(")")]));

    group(&content)
        .with_id(Some(group_id))
        .should_expand(force_expand)
        .format(f)
}

/// Format call arguments with the default list formatter.
fn format_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    use_separator_comment_multiline: bool,
    use_plain_default_short_circuit: bool,
    disallow_trailing_separator: bool,
    force_trailing_separator: bool,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LIST_DEFAULT);

    if use_separator_comment_multiline {
        format_separator_comment_multiline_layout(
            f,
            call_node_id,
            dynamic_arguments,
            "call.arguments.path.list_default.separator_comment_multiline",
        )?;
        return Ok(());
    }

    if use_plain_default_short_circuit {
        f.context()
            .increment_counter("call.arguments.path.list_default_plain_short_circuit", 1);

        return format_plain_default_call_argument_list(
            f,
            group_id,
            dynamic_arguments,
            force_expand,
            all_plain_call_arguments,
        );
    }

    let mut list = list_like("(", ")", ",", dynamic_arguments);
    list.with_group_id(Some(group_id))
        .should_expand(force_expand);

    if disallow_trailing_separator {
        list.disallow_trailing_separator();
    }

    if force_trailing_separator {
        list.force_trailing_separator();
    }

    write!(f, [list])
}

/// Format call arguments with explicit multiline comment expansion.
fn format_comment_expanded_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    use_separator_comment_multiline: bool,
    use_single_plain_separator_comment_layout: bool,
    use_trailing_comma: bool,
    force_trailing_comma_for_separator_comment: bool,
) -> FormatResult<()> {
    if use_separator_comment_multiline {
        format_separator_comment_multiline_layout(
            f,
            call_node_id,
            dynamic_arguments,
            "call.arguments.path.comment_expanded.separator_comment_multiline",
        )?;
        return Ok(());
    }

    if use_single_plain_separator_comment_layout {
        debug_assert_eq!(
            dynamic_arguments.len(),
            1,
            "single plain separator layout should have one argument",
        );

        let argument_id = dynamic_arguments[0];

        let separator_line_comment_source = plain_single_argument_separator_line_comment_source(
            f.context(),
            call_node_id,
            argument_id,
        );

        if let Some(comment_source) = separator_line_comment_source.as_ref() {
            return format_single_plain_argument_with_separator_line_comment(
                f,
                argument_id,
                comment_source,
            );
        }
    }

    let last_argument_separator_line_comment_source =
        dynamic_arguments.last().copied().and_then(|argument_id| {
            single_argument_separator_line_comment_source(f.context(), call_node_id, argument_id)
        });

    write!(f, [token("("), hard_line_break()])?;

    let format_result = write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                    if index > 0 {
                        let left_argument_id = dynamic_arguments[index - 1];

                        if call_arguments_preserve_blank_line_between(
                            f.context(),
                            left_argument_id,
                            *argument_id,
                        ) {
                            write!(f, [empty_line()])?;
                        } else {
                            write!(f, [hard_line_break()])?;
                        }
                    }

                    let is_last_argument = index + 1 == dynamic_arguments.len();
                    if is_last_argument
                        && let Some(comment_source) =
                            last_argument_separator_line_comment_source.as_ref()
                        && write_argument_without_separator_line_comment(f, *argument_id)?
                    {
                        write_separator_line_comment_after_comma(f, comment_source)?;
                        continue;
                    }

                    write!(f, [group(argument_id)])?;

                    if index + 1 < dynamic_arguments.len()
                        || use_trailing_comma
                        || force_trailing_comma_for_separator_comment
                    {
                        write!(f, [token(",")])?;
                    }
                }

                Ok(())
            }
        ))]
    );

    format_result?;
    write!(f, [hard_line_break(), token(")")])?;

    Ok(())
}

/// Format the shared separator-comment multiline list path and increment one counter key.
fn format_separator_comment_multiline_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    counter_key: &'static str,
) -> FormatResult<()> {
    let used_multiline_separator_layout =
        format_multiline_call_argument_list_with_last_separator_line_comment(
            f,
            call_node_id,
            dynamic_arguments,
        )?;

    debug_assert!(
        used_multiline_separator_layout,
        "separator-comment multiline layout should stay eligible from layout rules",
    );

    f.context().increment_counter(counter_key, 1);

    Ok(())
}

/// Render one decided call argument layout.
pub(crate) fn format_call_argument_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout: CallArgumentLayout,
    call_node_id: LocalNodeId<Expression>,
    group_id: GroupId,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    match layout {
        CallArgumentLayout::InlineAll => {
            write_inline_call_argument_list(f, dynamic_arguments, all_plain_call_arguments)
        }
        CallArgumentLayout::InlineSingle => {
            debug_assert_eq!(dynamic_arguments.len(), 1);

            let Some(argument_id) = dynamic_arguments.first().copied() else {
                debug_assert!(false, "single inline layout requires one argument");
                return Ok(());
            };

            write_single_call_argument_inline_wrapped(f, argument_id)
        }
        CallArgumentLayout::TrailingCollectionExpanded => {
            format_trailing_collection_hug_list(f, dynamic_arguments, all_plain_call_arguments)
        }
        CallArgumentLayout::CommentExpanded {
            use_separator_comment_multiline,
            use_single_plain_separator_comment_layout,
            use_trailing_comma,
            force_trailing_comma_for_separator_comment,
        } => format_comment_expanded_call_argument_list(
            f,
            call_node_id,
            dynamic_arguments,
            use_separator_comment_multiline,
            use_single_plain_separator_comment_layout,
            use_trailing_comma,
            force_trailing_comma_for_separator_comment,
        ),
        CallArgumentLayout::ListDefault {
            force_expand,
            use_separator_comment_multiline,
            use_plain_default_short_circuit,
            disallow_trailing_separator,
            force_trailing_separator,
        } => format_default_call_argument_list(
            f,
            call_node_id,
            group_id,
            dynamic_arguments,
            force_expand,
            use_separator_comment_multiline,
            use_plain_default_short_circuit,
            disallow_trailing_separator,
            force_trailing_separator,
            all_plain_call_arguments,
        ),
    }
}

/// Return whether an argument should emit its prefix annotations.
pub(crate) fn argument_should_emit_prefix_annotations(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if argument_satisfies_static_seam_comment_annotation_id(context, argument_id).is_some() {
        return false;
    }

    if !argument_is_call_or_new(context, argument_id) {
        return true;
    }

    if argument_has_non_blank_prefix_annotation(context, argument_id) {
        return true;
    }

    if argument_has_blank_prefix_annotation_before_separator(context, argument_id) {
        return false;
    }

    if !argument_is_first_in_call_or_new(context, argument_id) {
        return true;
    }

    !argument_has_blank_prefix_annotation(context, argument_id)
}

/// Return one satisfies static seam line comment annotation id for this argument when present.
pub(crate) fn argument_satisfies_static_seam_comment_annotation_id(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Annotation>> {
    if !argument_is_first_static_argument_of_satisfies_right_path(context, argument_id) {
        return None;
    }

    let Some(annotations) = context.annotations(argument_id) else {
        return None;
    };

    let mut seam_comment_id = None;
    for annotation_id in annotations {
        let Annotation::Comment {
            node,
            position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
        } = context.annotation(annotation_id)
        else {
            continue;
        };

        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Slash {
            continue;
        }

        seam_comment_id = Some(annotation_id);
        break;
    }

    seam_comment_id
}

/// Return whether one argument is the first static argument in a satisfies rhs path with multiple arguments.
fn argument_is_first_static_argument_of_satisfies_right_path(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_first_static_argument_of_multi_argument_path(context, argument_id) {
        return false;
    }

    let Some((path_expression_id, path_parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if path_parent_type != NodeType::Expression {
        return false;
    }

    let path_expression_id = LocalNodeId::<Expression>::new(path_expression_id);
    let Some((type_binary_id, type_binary_parent_type)) = context.parent(path_expression_id) else {
        return false;
    };
    if type_binary_parent_type != NodeType::Expression {
        return false;
    }

    let type_binary_id = LocalNodeId::<Expression>::new(type_binary_id);
    let Expression::TypeBinary {
        operator: TypeBinaryOperator::Satisfies,
        right,
        ..
    } = context.tree.get(type_binary_id)
    else {
        return false;
    };

    let right_expression_id = match context.tree.get(*right) {
        Expression::Parenthesized { expression } | Expression::Statement(expression) => *expression,
        _ => *right,
    };

    right_expression_id == path_expression_id
}

/// Return whether one argument is the first static argument of a multi-argument path static list.
fn argument_is_first_static_argument_of_multi_argument_path(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(parent_id);
    let static_arguments = match context.tree.get(expression_id) {
        // keep seam remapping constrained to the formatter path we re-render explicitly
        Expression::Path {
            path,
            static_arguments,
        } if path.segments.len() == 1 => static_arguments.as_ref(),
        _ => None,
    };

    let Some(static_arguments) = static_arguments else {
        return false;
    };
    if static_arguments.len() <= 1 {
        return false;
    }

    static_arguments
        .first()
        .is_some_and(|first| *first == argument_id)
}

/// Return whether an argument belongs to a call or new expression.
fn argument_is_call_or_new(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    matches!(expression, Expression::Call { .. } | Expression::New { .. })
}

/// Return whether an argument is the first in its call or new argument list.
fn argument_is_first_in_call_or_new(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    match expression {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments
            .first()
            .is_some_and(|first| *first == argument_id),
        _ => false,
    }
}

/// Return whether an argument has a non-blank prefix annotation.
fn argument_has_non_blank_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations
        .iter()
        .any(|annotation_id| match context.annotation(*annotation_id) {
            Annotation::Blank { .. } => false,
            Annotation::Doc { position, .. }
            | Annotation::Comment { position, .. }
            | Annotation::Decorator { position, .. } => matches!(
                position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ),
        })
}

/// Return whether an argument has a blank prefix annotation.
fn argument_has_blank_prefix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        matches!(
            context.annotation(*annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

/// Return whether an argument has a blank prefix annotation before a separator.
fn argument_has_blank_prefix_annotation_before_separator(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        let Annotation::Blank {
            position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
            ..
        } = context.annotation(*annotation_id)
        else {
            return false;
        };

        next_non_whitespace_token_after_annotation(context, *annotation_id)
            .is_some_and(|token| token.token.ty == TokenType::Comma)
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

        let should_emit_prefix_annotations =
            argument_should_emit_prefix_annotations(f.context(), node_id);
        let has_lambda_value = argument_contains_lambda_value(f.context(), self);
        let force_break_after_lambda_prefix_comment = has_lambda_value
            && argument_prefix_lambda_comment_needs_forced_break(f.context(), node_id);

        if has_lambda_value || should_emit_prefix_annotations {
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
        }

        write_argument_with_modifiers_and_value(self, force_break_after_lambda_prefix_comment, f)?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

/// Return whether a lambda argument has an inline prefix comment that must break.
fn argument_prefix_lambda_comment_needs_forced_break(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(*annotation_id) else {
            return false;
        };
        if position != AnnotationPosition::BlockPrefix {
            return false;
        }

        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Star {
            return false;
        }

        let previous_token =
            previous_non_whitespace_token_before_annotation(context, *annotation_id);
        let next_token = next_non_whitespace_token_after_annotation(context, *annotation_id);
        previous_token.is_some_and(|token| {
            matches!(
                token.token.ty,
                TokenType::OpenParenthesis
                    | TokenType::OpenBracket
                    | TokenType::OpenBrace
                    | TokenType::LessThan
            )
        }) && next_token.is_some_and(|token| token.token.ty == TokenType::OpenParenthesis)
    })
}

/// Return whether this argument wraps a lambda declaration expression.
fn argument_contains_lambda_value(context: &DestackFormatContext<'_>, argument: &Argument) -> bool {
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };

    matches!(
        context.tree.get(*declaration_id),
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
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_argument_named() {
        assert_format!(
            "x: 1",
            "x: 1",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named_shorthand() {
        assert_format!(
            "x",
            "x",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_positional() {
        assert_format!(
            "1",
            "1",
            |p| p.eat_tree_argument(),
            DestackFormatOptions::default()
        );
    }
}
