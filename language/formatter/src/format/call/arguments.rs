use crate::FormatNode;
use crate::format::analysis::{
    argument_has_leading_prefix_annotation_outside_span, argument_has_multiline_prefix_annotation,
    argument_is_inline_closure_cast_object, argument_is_interpolated_template_literal,
    call_arguments_have_boundary_comments, call_arguments_preserve_blank_line_between,
    call_has_static_arguments, next_non_whitespace_token_after_annotation,
    previous_non_whitespace_token_before_annotation, previous_non_whitespace_token_before_span,
    timing,
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
use crate::format::directive::{any_ignore_range_for_nodes, directive_for_node};
use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, Declaration, DestackFormatContext, DestackFormatter,
    Expression, FormatResult, FunctionKind, GroupId, HugOptions, LocalNodeId, NodeType, Span,
    TokenType, TrailingComma, TypeBinaryOperator, argument_is_function_expression,
    argument_is_lambda_expression, argument_value_id, block_indent, empty_line,
    format_block_of_properties, format_expression, format_hugged, format_static_argument_list,
    format_with, group, hard_line_break, if_group_breaks, is_trivial_expression, line_postfix,
    list_like, soft_block_indent, soft_line_break_or_space, space, token,
    transparent_inner_expression,
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
    write!(f, [f.context().any_prefix_annotations(argument_id)])?;

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
    /// Whether source had a blank line between separator and first comment.
    pub(crate) has_blank_line_before_first_comment: bool,
    /// Whether comments were detached from the following argument prefix.
    pub(crate) detached_from_following_prefix: bool,
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
) -> Option<(LocalNodeId<Comment>, bool, bool)> {
    let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
        return None;
    };

    if !matches!(
        position,
        AnnotationPosition::LinePostfix
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
    let following_close_delimiter = following_token.is_some_and(|token| {
        matches!(
            token.token.ty,
            TokenType::CloseBrace | TokenType::CloseBracket | TokenType::CloseParenthesis
        )
    });
    let has_virtual_trailing_separator = preceding_comma.is_none()
        && !following_separator
        && following_close_delimiter
        && position == AnnotationPosition::LinePostfixBoundary;

    if preceding_comma.is_none() && !following_separator && !has_virtual_trailing_separator {
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
        has_virtual_trailing_separator,
    );
    let has_blank_line_before_first_comment = if let Some(separator_token) = preceding_comma {
        let before_comment_span = Span::new(
            annotation_span.file,
            separator_token.span.end,
            annotation_span.start,
        );
        context.has_blank_line(before_comment_span)
    } else if following_separator {
        if let Some(separator_token) =
            next_non_whitespace_token_after_annotation(context, annotation_id)
        {
            let before_separator_span = Span::new(
                annotation_span.file,
                annotation_span.end,
                separator_token.span.start,
            );
            context.has_blank_line(before_separator_span)
        } else {
            false
        }
    } else {
        false
    };

    Some((node, is_own_line, has_blank_line_before_first_comment))
}

/// Return whether one separator comment starts on its own source line.
fn separator_line_comment_is_own_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    annotation_span: Span,
    preceding_comma: Option<TokenSpan>,
    following_separator: bool,
    has_virtual_trailing_separator: bool,
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

    if has_virtual_trailing_separator {
        return context.annotation_starts_on_own_line(annotation_id);
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

        let Some((comment_id, is_own_line, has_blank_line_before_first_comment)) =
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

            let Some((next_comment_id, _, _)) =
                separator_line_comment_annotation_info(context, next_annotation_id)
            else {
                break;
            };

            comment_ids.push(next_comment_id);
        }

        return Some(SeparatorLineCommentSource {
            comment_ids,
            is_own_line,
            has_blank_line_before_first_comment,
            detached_from_following_prefix: false,
        });
    }

    None
}

/// Return one separator line comment source from following-argument prefix annotations.
fn separator_line_comment_source_from_following_prefix_annotations<F>(
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

        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            continue;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        ) {
            continue;
        }

        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Slash {
            continue;
        }

        let annotation_span = context.annotation_span(annotation_id);
        let Some(preceding_token) =
            previous_non_whitespace_token_before_span(context, annotation_span)
        else {
            continue;
        };
        if preceding_token.token.ty != TokenType::Comma {
            continue;
        }

        let before_comment_span = Span::new(
            annotation_span.file,
            preceding_token.span.end,
            annotation_span.start,
        );
        let is_own_line = context.has_newline(before_comment_span);
        let has_blank_line_before_first_comment = context.has_blank_line(before_comment_span);
        let mut comment_ids = vec![node];
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

            let Annotation::Comment {
                node: next_node,
                position: next_position,
            } = context.annotation(next_annotation_id)
            else {
                break;
            };
            if !matches!(
                next_position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ) {
                break;
            }

            let next_comment = context.tree.get::<Comment>(next_node);
            if next_comment.style != CommentStyle::Slash {
                break;
            }

            let next_annotation_span = context.annotation_span(next_annotation_id);
            let Some(next_preceding_token) =
                previous_non_whitespace_token_before_span(context, next_annotation_span)
            else {
                break;
            };
            if !matches!(
                next_preceding_token.token.ty,
                TokenType::Comma | TokenType::LineComment | TokenType::DocLineComment
            ) {
                break;
            }

            comment_ids.push(next_node);
        }

        return Some(SeparatorLineCommentSource {
            comment_ids,
            is_own_line,
            has_blank_line_before_first_comment,
            detached_from_following_prefix: true,
        });
    }

    None
}

/// Return the next argument in one call-like or array argument list.
fn next_argument_in_expression(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Argument>> {
    let argument_list = match context.tree.get(call_node_id) {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments.as_slice(),
        Expression::ArrayExpression { elements } => elements.as_slice(),
        _ => return None,
    };

    let argument_index = argument_list
        .iter()
        .position(|candidate_id| *candidate_id == argument_id)?;
    argument_list.get(argument_index + 1).copied()
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

    let call_prefix_annotation_source = context.annotations(call_node_id).and_then(|annotations| {
        separator_line_comment_source_from_following_prefix_annotations(
            context,
            &annotations,
            |annotation_id| {
                let annotation_span = context.annotation_span(annotation_id);
                annotation_span.file == argument_span.file
                    && annotation_span.start >= seam_start
                    && annotation_span.end <= call_span.end
            },
        )
    });
    if call_prefix_annotation_source.is_some() {
        return call_prefix_annotation_source;
    }

    let following_argument_id = next_argument_in_expression(context, call_node_id, argument_id)?;
    let resolve_from_following_prefix_annotations = |annotations: &[LocalNodeId<Annotation>]| {
        separator_line_comment_source_from_following_prefix_annotations(
            context,
            annotations,
            |_| true,
        )
    };

    if let Some(annotations) = context.annotations(following_argument_id)
        && let Some(comment_source) = resolve_from_following_prefix_annotations(&annotations)
    {
        return Some(comment_source);
    }

    let following_value_id = argument_value_id(context.tree, following_argument_id);
    context
        .annotations(following_value_id)
        .and_then(|annotations| resolve_from_following_prefix_annotations(&annotations))
}

/// Return one separator line comment source when the argument can detach boundary separator comments.
fn plain_single_argument_separator_line_comment_source(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
) -> Option<SeparatorLineCommentSource> {
    if !argument_can_render_without_separator_line_comment(context, argument_id) {
        return None;
    }

    single_argument_separator_line_comment_source(context, call_node_id, argument_id)
}

/// Return whether one argument has non-separator boundary-postfix annotations.
fn argument_has_non_separator_boundary_postfix_annotation(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    context
        .visit_annotations(argument_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let position = context.annotation(*annotation_id).position();
                let is_boundary_postfix = position == AnnotationPosition::LinePostfixBoundary;
                is_boundary_postfix
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
    if argument_has_non_separator_boundary_postfix_annotation(context, argument_id) {
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

/// Return whether the list can use multiline separator comment rendering on any argument.
pub(crate) fn can_format_multiline_call_argument_list_with_separator_line_comment(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    dynamic_arguments.iter().copied().any(|argument_id| {
        single_argument_separator_line_comment_source(context, call_node_id, argument_id).is_some()
            && argument_can_render_without_separator_line_comment(context, argument_id)
    })
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
        let comment_break = if separator_comment_source.has_blank_line_before_first_comment {
            empty_line()
        } else {
            hard_line_break()
        };
        write!(f, [token(","), comment_break, first_comment_id])?;
        for comment_id in separator_comment_source.comment_ids.iter().skip(1) {
            write!(f, [hard_line_break(), comment_id])?;
        }
    } else {
        let inline_suffix = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [space(), first_comment_id])
        });
        write!(f, [token(","), line_postfix(&inline_suffix, 0)])?;
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

    let argument = f.context().tree.get(argument_id);
    let should_emit_prefix_annotations =
        argument_should_emit_prefix_annotations(f.context(), argument_id);
    let has_lambda_value = argument_contains_lambda_value(f.context(), argument);
    let force_break_after_lambda_prefix_comment = has_lambda_value
        && argument_prefix_lambda_comment_needs_forced_break(f.context(), argument_id);

    if has_lambda_value || should_emit_prefix_annotations {
        write!(f, [f.context().any_prefix_annotations(argument_id)])?;
    }

    write_argument_with_modifiers_and_value(argument, force_break_after_lambda_prefix_comment, f)?;
    write!(
        f,
        [f.context()
            .any_infix_or_postfix_except_line_postfix_boundary_annotations(argument_id)]
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
        let call_parent_is_decorator =
            f.context().parent(node_id).is_some_and(|(_, parent_type)| {
                matches!(parent_type, NodeType::Decorator | NodeType::Annotation)
            });
        if call_parent_is_decorator {
            let left_expression = f.context().tree.get(*left);
            let left_directive = directive_for_node(f.context(), *left);
            format_expression(f, *left, left_expression, left_directive)?;
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
        let should_emit_trailing_separator = force_trailing_separator
            || (!disallow_trailing_separator
                && f.context().options.trailing_comma == TrailingComma::All);
        format_separator_comment_multiline_layout(
            f,
            call_node_id,
            dynamic_arguments,
            "call.arguments.path.list_default.separator_comment_multiline",
            should_emit_trailing_separator,
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
fn preserve_blank_line_before_argument_with_separator_comments(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    separator_line_comment_sources: &[Option<SeparatorLineCommentSource>],
    argument_index: usize,
) -> bool {
    let left_argument_id = dynamic_arguments[argument_index - 1];
    let right_argument_id = dynamic_arguments[argument_index];

    // when separator comments are rendered after the previous argument comma,
    // preserve blank lines only between that rendered comment cluster and the next argument
    if let Some(previous_separator_source) =
        separator_line_comment_sources[argument_index - 1].as_ref()
        && previous_separator_source.is_own_line
        && let Some(last_comment_id) = previous_separator_source.comment_ids.last().copied()
    {
        let last_comment_span = context.span(last_comment_id);
        let right_argument_span = context.span(right_argument_id);
        if let Some(between_span) = last_comment_span.gap_to(right_argument_span) {
            return context.has_blank_line(between_span);
        }
    }

    call_arguments_preserve_blank_line_between(context, left_argument_id, right_argument_id)
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
        let should_emit_trailing_separator =
            use_trailing_comma || force_trailing_comma_for_separator_comment;
        format_separator_comment_multiline_layout(
            f,
            call_node_id,
            dynamic_arguments,
            "call.arguments.path.comment_expanded.separator_comment_multiline",
            should_emit_trailing_separator,
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

    let separator_line_comment_sources = dynamic_arguments
        .iter()
        .copied()
        .map(|argument_id| {
            single_argument_separator_line_comment_source(f.context(), call_node_id, argument_id)
        })
        .collect::<Vec<_>>();

    write!(f, [token("("), hard_line_break()])?;

    let format_result = write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                    if index > 0 {
                        if preserve_blank_line_before_argument_with_separator_comments(
                            f.context(),
                            dynamic_arguments,
                            &separator_line_comment_sources,
                            index,
                        ) {
                            write!(f, [empty_line()])?;
                        } else {
                            write!(f, [hard_line_break()])?;
                        }
                    }

                    if index > 0
                        && let Some(previous_separator_source) =
                            separator_line_comment_sources[index - 1].as_ref()
                        && previous_separator_source.detached_from_following_prefix
                        && write_argument_without_separator_line_comment(f, *argument_id)?
                    {
                        if index + 1 < dynamic_arguments.len()
                            || use_trailing_comma
                            || force_trailing_comma_for_separator_comment
                        {
                            write!(f, [token(",")])?;
                        }
                        continue;
                    }

                    if let Some(comment_source) = separator_line_comment_sources[index].as_ref()
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
    should_emit_trailing_separator: bool,
) -> FormatResult<()> {
    let separator_line_comment_sources = dynamic_arguments
        .iter()
        .copied()
        .map(|argument_id| {
            single_argument_separator_line_comment_source(f.context(), call_node_id, argument_id)
        })
        .collect::<Vec<_>>();

    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
                for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                    if index > 0 {
                        if preserve_blank_line_before_argument_with_separator_comments(
                            f.context(),
                            dynamic_arguments,
                            &separator_line_comment_sources,
                            index,
                        ) {
                            write!(f, [empty_line()])?;
                        } else {
                            write!(f, [hard_line_break()])?;
                        }
                    }

                    if index > 0
                        && let Some(previous_separator_source) =
                            separator_line_comment_sources[index - 1].as_ref()
                        && previous_separator_source.detached_from_following_prefix
                        && write_argument_without_separator_line_comment(f, *argument_id)?
                    {
                        if index + 1 < dynamic_arguments.len() || should_emit_trailing_separator {
                            write!(f, [token(",")])?;
                        }
                        continue;
                    }

                    if let Some(comment_source) = separator_line_comment_sources[index].as_ref()
                        && write_argument_without_separator_line_comment(f, *argument_id)?
                    {
                        write_separator_line_comment_after_comma(f, comment_source)?;
                        continue;
                    }

                    write!(f, [group(argument_id)])?;

                    if index + 1 < dynamic_arguments.len() || should_emit_trailing_separator {
                        write!(f, [token(",")])?;
                    }
                }

                Ok(())
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token(")")])?;

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

    if argument_is_call_or_new(context, argument_id)
        && argument_has_only_separator_prefix_comment_cluster(context, argument_id)
        && !argument_is_first_in_call_or_new(context, argument_id)
    {
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

/// Return whether one argument has only separator comment prefix annotations.
fn argument_has_only_separator_prefix_comment_cluster(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some(annotations) = context.annotations(argument_id) else {
        return false;
    };

    let mut has_separator_comment = false;
    for annotation_id in annotations.iter().copied() {
        let annotation = context.annotation(annotation_id);
        if !matches!(
            annotation.position(),
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        ) {
            continue;
        }

        match annotation {
            Annotation::Blank { .. } => {}
            Annotation::Comment { node, .. } => {
                let comment = context.tree.get::<Comment>(node);
                if comment.style != CommentStyle::Slash {
                    return false;
                }

                let annotation_span = context.annotation_span(annotation_id);
                let Some(preceding_token) =
                    previous_non_whitespace_token_before_span(context, annotation_span)
                else {
                    return false;
                };
                if !matches!(
                    preceding_token.token.ty,
                    TokenType::Comma | TokenType::LineComment | TokenType::DocLineComment
                ) {
                    return false;
                }

                has_separator_comment = true;
            }
            Annotation::Doc { .. } | Annotation::Decorator { .. } => return false,
        }
    }

    has_separator_comment
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
    use super::{
        argument_can_render_without_separator_line_comment, single_argument_requires_expanded_list,
        single_argument_separator_line_comment_source,
    };
    use crate::format::analysis::call_arguments_have_boundary_comments;
    use crate::format::call::layout::{
        CallArgumentLayout, call_argument_comments, call_argument_layout,
        call_argument_layout_cache, call_force_expand_single_collection_for_type_binary_callee,
        call_force_expand_single_multiline_with_static_arguments,
    };
    use crate::{
        DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, TestFormatter,
        assert_format, assert_format_output_eq, assert_format_program_idempotent_with_file_type,
        statement_list,
    };
    use destack_ast::{Argument, Expression, LocalNodeId, NodeParentIndex};
    use destack_source::FileType;

    /// Build a formatter context for call-argument separator assertions.
    fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
        DestackFormatContext::new(
            DestackFormatOptions::default(),
            DestackFormatArtifacts {
                file: &formatter.file,
                tree: &formatter.tree,
                tokens: &formatter.tokens,
                side_tokens: &formatter.side_tokens,
                side_span: &formatter.side_span,
                strings: &formatter.strings,
                parents: NodeParentIndex::from_tree(&formatter.tree),
            },
        )
    }

    /// Return the argument ids from one call expression.
    fn call_dynamic_arguments(
        context: &DestackFormatContext<'_>,
        call_id: LocalNodeId<Expression>,
    ) -> Vec<LocalNodeId<Argument>> {
        let Expression::Call {
            dynamic_arguments, ..
        } = context.tree.get(call_id)
        else {
            panic!("expected call expression");
        };
        dynamic_arguments.clone()
    }

    /// Return the element ids from one array expression.
    fn array_elements(
        context: &DestackFormatContext<'_>,
        array_id: LocalNodeId<Expression>,
    ) -> Vec<LocalNodeId<Argument>> {
        let Expression::ArrayExpression { elements } = context.tree.get(array_id) else {
            panic!("expected array expression");
        };

        elements.clone()
    }

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

    #[test]
    fn test_separator_comment_source_and_plain_render_eligibility_for_oxfmt_conditional_fixture() {
        let source = "cb(
    overflowing ? 'absolute top-0' : 'relative', // inline-separator-marker
    parameter
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse conditional separator fixture");
        let context = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&context, call_id);
        let first_argument_id = arguments[0];
        let (has_line_comment_annotations, has_prefix_line_comment_annotations) =
            call_argument_comments(&context, &arguments);
        let has_boundary_comments =
            call_arguments_have_boundary_comments(&context, call_id, &arguments);
        let layout = call_argument_layout(
            &context,
            call_id,
            &arguments,
            false,
            false,
            false,
            has_boundary_comments,
        );

        let separator_source =
            single_argument_separator_line_comment_source(&context, call_id, first_argument_id);
        assert!(
            separator_source.is_some(),
            "expected separator comment source for first argument",
        );
        assert!(
            has_line_comment_annotations,
            "expected call argument scan to detect line comment annotations",
        );
        assert!(
            !has_prefix_line_comment_annotations,
            "expected no prefix line comment annotations for this fixture",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&context, first_argument_id),
            "expected first argument to support separator comment list-level rendering",
        );
        assert!(
            matches!(layout, CallArgumentLayout::CommentExpanded { .. }),
            "expected conditional separator fixture to use comment-expanded layout",
        );
    }

    #[test]
    fn test_separator_comment_source_and_plain_render_eligibility_with_inline_block_comment() {
        let source = "cb(
    overflowing ? 'absolute top-0' : 'relative' /* */, // inline-separator-marker
    parameter
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse conditional separator fixture with inline block comment");
        let context = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&context, call_id);
        let first_argument_id = arguments[0];
        let (has_line_comment_annotations, has_prefix_line_comment_annotations) =
            call_argument_comments(&context, &arguments);
        let has_boundary_comments =
            call_arguments_have_boundary_comments(&context, call_id, &arguments);
        let layout = call_argument_layout(
            &context,
            call_id,
            &arguments,
            false,
            false,
            false,
            has_boundary_comments,
        );

        let separator_source =
            single_argument_separator_line_comment_source(&context, call_id, first_argument_id);
        assert!(
            separator_source.is_some(),
            "expected separator comment source for first argument",
        );
        assert!(
            has_line_comment_annotations,
            "expected call argument scan to detect line comment annotations",
        );
        assert!(
            !has_prefix_line_comment_annotations,
            "expected no prefix line comment annotations for this fixture",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&context, first_argument_id),
            "inline block comment should remain eligible when only boundary separator comments are detached",
        );
        assert!(
            matches!(layout, CallArgumentLayout::CommentExpanded { .. }),
            "expected conditional separator fixture with inline block to use comment-expanded layout",
        );
    }

    #[test]
    fn test_separator_comment_source_ignores_object_property_trailing_comment_in_single_argument() {
        let source = "Component({
  selector: \"my-component\", // test
})";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse call with trailing property comment");
        let context = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&context, call_id);
        let first_argument_id = arguments[0];

        let separator_source =
            single_argument_separator_line_comment_source(&context, call_id, first_argument_id);
        assert!(
            separator_source.is_none(),
            "property trailing comment should not be classified as an argument separator comment",
        );
    }

    #[test]
    fn test_separator_comment_source_detects_last_array_element_own_line_comment_before_close() {
        let source = "[
  // comment1
  1, 2, 3
  // comment2
]";
        let (formatter, array_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse array expression with trailing own-line separator comment");
        let context = context_from_formatter(&formatter);
        let elements = array_elements(&context, array_id);
        let last_element_id = *elements.last().expect("array should have a last element");

        let separator_source =
            single_argument_separator_line_comment_source(&context, array_id, last_element_id);
        let separator_source =
            separator_source.expect("expected separator source for last array element");
        assert!(
            separator_source.is_own_line,
            "expected trailing separator comment source to be own-line",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&context, last_element_id),
            "expected last array element to support separator comment list-level rendering",
        );
    }

    #[test]
    fn test_separator_comment_source_from_call_annotations_allows_positional_argument_detach() {
        let source = "useEffect(
  () => {
    console.log(\"x\");
  }

  ,
  // deps-marker
  []
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse call-level separator comment source");
        let context = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&context, call_id);
        let first_argument_id = arguments[0];
        let separator_source =
            single_argument_separator_line_comment_source(&context, call_id, first_argument_id);

        assert!(
            separator_source.is_some(),
            "expected separator source from call-level annotations",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&context, first_argument_id),
            "expected first positional argument to allow separator-comment detachment",
        );
    }

    #[test]
    fn test_separator_comment_source_detects_react_hook_deps_comment_cluster() {
        let source = "useEffect(
  () => {
    console.log(\"some code\", props.foo);
  },

  // We need to disable the eslint warning here,
  // because of some complicated reason.
  // eslint-disable line react-hooks/exhaustive-deps
  []
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse react hook deps comment source");
        let context = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&context, call_id);
        let first_argument_id = arguments[0];
        let separator_source =
            single_argument_separator_line_comment_source(&context, call_id, first_argument_id);

        assert!(
            separator_source.is_some(),
            "expected separator source for callback argument before deps array",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&context, first_argument_id),
            "expected callback positional argument to allow separator-comment detachment",
        );
    }

    #[test]
    fn test_format_react_hook_separator_comment_cluster_is_idempotent() {
        let source = r#"useEffect(
  () => {
    console.log("some code", props.foo);
  }

  ,
  // We need to disable the eslint warning here,
  // because of some complicated reason.
  // eslint-disable line react-hooks/exhaustive-deps
  []
)"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScriptXml, options);
    }

    #[test]
    fn test_separator_comment_source_stays_detectable_after_first_pass() {
        let source = r#"useEffect(
  () => {
    console.log("some code", props.foo);
  }

  ,
  // We need to disable the eslint warning here,
  // because of some complicated reason.
  // eslint-disable line react-hooks/exhaustive-deps
  []
)"#;

        let (first_formatter, first_call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScriptXml, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse first react hook separator source");
        let first_context = context_from_formatter(&first_formatter);
        let first_arguments = call_dynamic_arguments(&first_context, first_call_id);
        let first_separator_source = single_argument_separator_line_comment_source(
            &first_context,
            first_call_id,
            first_arguments[0],
        );
        assert!(
            first_separator_source.is_some(),
            "expected separator source before first pass format",
        );

        let first_output = first_formatter.format(&first_call_id, DestackFormatOptions::default());
        let (second_formatter, second_call_id) =
            TestFormatter::parse_with_file_type(&first_output, FileType::JavaScriptXml, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse second react hook separator source");
        let second_context = context_from_formatter(&second_formatter);
        let second_arguments = call_dynamic_arguments(&second_context, second_call_id);
        let second_separator_source = single_argument_separator_line_comment_source(
            &second_context,
            second_call_id,
            second_arguments[0],
        );
        assert!(
            second_separator_source.is_some(),
            "expected separator source after first pass format",
        );
    }

    #[test]
    fn test_format_react_hook_separator_comment_program_slice_is_idempotent() {
        let source = r#"function MyComponent(props) {
  useEffect(
    () => {
      console.log("some code", props.foo);
    },

    // We need to disable the eslint warning here,
    // because of some complicated reason.
    // eslint-disable line react-hooks/exhaustive-deps
    []
  );

  return null;
}"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        let (first_formatter, first_roots) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScriptXml, |p| Ok(p.parse()))
                .expect("parse first react hook program slice");
        let first_output = first_formatter.format(&statement_list(&first_roots), options.clone());

        let (second_formatter, second_roots) =
            TestFormatter::parse_with_file_type(&first_output, FileType::JavaScriptXml, |p| {
                Ok(p.parse())
            })
            .expect("parse second react hook program slice");
        let second_output = second_formatter.format(&statement_list(&second_roots), options);
        assert_format_output_eq(&first_output, &second_output);
    }

    /// Method-chain calls with leading argument line comments should stay idempotent.
    #[test]
    fn test_format_method_chain_leading_argument_comment_is_idempotent() {
        let source = r#"Something
  // flow-fix-marker
  .getInstance(this.props.dao)
  .getters()"#;
        assert_format_program_idempotent_with_file_type(
            source,
            FileType::JavaScript,
            DestackFormatOptions::default(),
        );

        let source = r#"/* very very very very very very very long such that it is longer than 80 columns */ Something.getInstance(
// flow-fix-marker
this.props.dao)"#;
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse single-call leading line comment source");
        let context = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&context, call_id);
        let has_boundary_comments =
            call_arguments_have_boundary_comments(&context, call_id, &arguments);
        assert!(
            has_boundary_comments,
            "leading line comment between call `(` and first argument should count as boundary comment",
        );

        assert_format_program_idempotent_with_file_type(
            source,
            FileType::JavaScript,
            DestackFormatOptions::default(),
        );
    }

    #[test]
    fn test_chain_single_argument_call_layout_stays_inline_with_blank_line_before_chain_call() {
        let source = "this.connections

  .concat(this.activities.concat(this.operators))
  .filter(x => x.selected)";
        let (formatter, root_call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse chain single argument call source");
        let context = context_from_formatter(&formatter);
        let Expression::Call { left, .. } = context.tree.get(root_call_id) else {
            panic!("expected root call expression");
        };
        let Expression::Member {
            left: concat_call_id,
            ..
        } = context.tree.get(*left)
        else {
            panic!("expected root call receiver member expression");
        };
        let call_id = *concat_call_id;
        let arguments = call_dynamic_arguments(&context, call_id);
        let has_boundary_comments =
            call_arguments_have_boundary_comments(&context, call_id, &arguments);
        let single_argument_force_expand =
            single_argument_requires_expanded_list(&context, &arguments);
        let force_expand_single_multiline_with_static_arguments =
            call_force_expand_single_multiline_with_static_arguments(&context, call_id, &arguments);
        let force_expand_single_collection_for_type_binary_callee =
            call_force_expand_single_collection_for_type_binary_callee(
                &context, call_id, &arguments,
            );
        let layout_cache = call_argument_layout_cache(&context, call_id, &arguments);
        let layout = call_argument_layout(
            &context,
            call_id,
            &arguments,
            single_argument_force_expand,
            force_expand_single_multiline_with_static_arguments,
            force_expand_single_collection_for_type_binary_callee,
            has_boundary_comments,
        );

        assert!(
            layout_cache.has_call_chain_parent,
            "expected call to be recognized as call-chain parent",
        );
        assert!(
            !has_boundary_comments,
            "blank chain seams should not force boundary-comment expansion",
        );
        assert!(
            !single_argument_force_expand,
            "single-argument chain call should not force expanded argument list",
        );
        assert!(
            !force_expand_single_multiline_with_static_arguments,
            "single-argument chain call should not force multiline due static arguments",
        );
        assert!(
            !force_expand_single_collection_for_type_binary_callee,
            "single-argument chain call should not force multiline due type-binary callee",
        );

        let layout_name = match layout {
            CallArgumentLayout::InlineAll => "InlineAll",
            CallArgumentLayout::InlineSingle => "InlineSingle",
            CallArgumentLayout::TrailingCollectionExpanded => "TrailingCollectionExpanded",
            CallArgumentLayout::CommentExpanded { .. } => "CommentExpanded",
            CallArgumentLayout::ListDefault { .. } => "ListDefault",
        };

        assert!(
            matches!(layout, CallArgumentLayout::InlineSingle),
            "expected chain call to keep single nested-call argument inline, got {layout_name}",
        );
    }

    #[test]
    fn test_chain_single_argument_expect_call_layout_stays_inline() {
        let source = "expect(new LongLongLongLongLongRange([0, 0], [0, 0])).toEqualAtomLongLongLongLongRange(new LongLongLongRange([0, 0], [0, 0]))";
        let (formatter, root_call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse break-calls expect chain source");
        let context = context_from_formatter(&formatter);
        let Expression::Call { left, .. } = context.tree.get(root_call_id) else {
            panic!("expected root call expression");
        };
        let Expression::Member {
            left: expect_call_id,
            ..
        } = context.tree.get(*left)
        else {
            panic!("expected root call receiver member expression");
        };
        let call_id = *expect_call_id;
        let arguments = call_dynamic_arguments(&context, call_id);
        let has_boundary_comments =
            call_arguments_have_boundary_comments(&context, call_id, &arguments);
        let single_argument_force_expand =
            single_argument_requires_expanded_list(&context, &arguments);
        let force_expand_single_multiline_with_static_arguments =
            call_force_expand_single_multiline_with_static_arguments(&context, call_id, &arguments);
        let force_expand_single_collection_for_type_binary_callee =
            call_force_expand_single_collection_for_type_binary_callee(
                &context, call_id, &arguments,
            );
        let layout = call_argument_layout(
            &context,
            call_id,
            &arguments,
            single_argument_force_expand,
            force_expand_single_multiline_with_static_arguments,
            force_expand_single_collection_for_type_binary_callee,
            has_boundary_comments,
        );

        assert!(
            matches!(layout, CallArgumentLayout::InlineSingle),
            "expected expect(...) head call to keep one argument inline",
        );
    }
}
