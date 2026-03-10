use crate::format::analysis::{
    argument_has_multiline_prefix_annotation, argument_is_inline_closure_cast_object,
    argument_is_interpolated_template_literal, call_arguments_preserve_blank_line_between,
    call_has_static_arguments, next_non_trivia_token_after_annotation,
    next_non_whitespace_token_after_annotation, previous_non_trivia_token_before_span,
    previous_non_whitespace_token_before_annotation, previous_non_whitespace_token_before_span,
    timing,
};
use crate::format::annotation::{
    AnnotationCapture, annotation_render_items, write_annotation_render_items,
};
use crate::format::call::layout::{
    CallArgumentLayout, argument_has_callback_blocking_comment_annotation, call_argument_layout,
    call_argument_layout_cache, call_force_expand_single_collection_for_type_binary_callee,
    call_force_expand_single_multiline_with_static_arguments, chain_call_argument_force_expand,
    single_argument_requires_expanded_list,
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
use crate::{FormatNode, SeparatorLineCommentSourceCache};
use destack_ast::{Comment, CommentStyle, PostfixPosition, TokenSpan};
use destack_fir::format::{Buffer, Format};
use destack_fir::write;

/// One detachable separator line comment cluster for one argument seam.
pub(crate) type SeparatorLineCommentSource = SeparatorLineCommentSourceCache;

/// Write an inline comma-separated call argument list.
pub(crate) fn write_inline_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    write!(f, [token("(")])?;

    if all_plain_call_arguments {
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }

            write_plain_call_argument(f, *argument_id)?;
        }
    } else {
        for (index, argument_id) in dynamic_arguments.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }

            write_plain_call_argument_or_node(f, *argument_id)?;
        }
    }

    write!(f, [token(")")])?;

    Ok(())
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

/// Return whether a call should expand its argument list when formatted in a chain.
pub(crate) fn call_arguments_force_expand_for_chain(
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(force_expand) = ctx.lookup_call_argument_chain_force_expand(call_node_id) {
        ctx.increment_counter("call.arguments.chain.simple_cache.hits", 1);
        ctx.increment_counter(
            if force_expand {
                "call.arguments.chain.force_expand.true"
            } else {
                "call.arguments.chain.force_expand.false"
            },
            1,
        );
        return force_expand;
    }
    ctx.increment_counter("call.arguments.chain.simple_cache.misses", 1);

    if dynamic_arguments.is_empty() {
        ctx.store_call_argument_chain_force_expand(call_node_id, false);
        ctx.increment_counter("call.arguments.chain.simple_false.empty", 1);
        ctx.increment_counter("call.arguments.chain.force_expand.false", 1);
        return false;
    }

    let layout_cache = call_argument_layout_cache(ctx, call_node_id, dynamic_arguments);
    let should_bypass_simple_false = dynamic_arguments.len() == 1
        && single_argument_requires_expanded_list(ctx, dynamic_arguments);
    let can_use_simple_false = !layout_cache.has_call_infix_annotations
        && layout_cache.all_compact_simple_unannotated
        && !should_bypass_simple_false;
    if can_use_simple_false {
        ctx.store_call_argument_chain_force_expand(call_node_id, false);
        ctx.increment_counter("call.arguments.chain.simple_false.simple", 1);
        ctx.increment_counter("call.arguments.chain.force_expand.false", 1);
        return false;
    }

    let force_expand = chain_call_argument_force_expand(ctx, call_node_id, dynamic_arguments);
    ctx.store_call_argument_chain_force_expand(call_node_id, force_expand);
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
    let layout_cache = call_argument_layout_cache(f.context(), call_node_id, &single_argument);
    let has_boundary_comments = layout_cache.has_boundary_comments;
    let all_plain_call_arguments = layout_cache.all_plain_call_arguments;
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
        let context = f.context();
        call_argument_layout(
            context,
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
        group_id,
        all_plain_call_arguments,
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
    let has_boundary_comments = layout_cache.has_boundary_comments;

    let _timing = f
        .context()
        .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT);
    let layout = {
        let _timing = f
            .context()
            .timing_scope(timing::FORMAT_EXPRESSION_CALL_ARGUMENTS_LAYOUT_DECIDE);
        let context = f.context();
        call_argument_layout(
            context,
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
    ctx: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    ctx.visit_annotations(call_node_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            let annotation = ctx.annotation(*annotation_id);
            if annotation.position() != AnnotationPosition::BlockInfix {
                return false;
            }

            let annotation_span = ctx.annotation_span(*annotation_id);
            if ctx.has_newline(annotation_span) {
                return true;
            }

            match annotation {
                Annotation::Comment { node, .. } => {
                    let comment = ctx.tree.get::<Comment>(node);
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

/// Return the parent call-like or array expression that owns one argument.
fn argument_parent_expression(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    let (parent_id, parent_type) = ctx.parent(argument_id)?;
    (parent_type == NodeType::Expression).then_some(LocalNodeId::new(parent_id))
}

/// Control virtual trailing-separator detection for closing-delimiter comment seams.
#[derive(Clone, Copy, PartialEq, Eq)]
enum VirtualTrailingSeparatorPolicy {
    /// Disable virtual separator detection.
    Disabled,
    /// Allow virtual separators for boundary comments on any line.
    BoundaryAnyLine,
}

/// Return one source comma token that owns one separator slash comment seam.
fn separator_line_comment_preceding_comma(
    ctx: &DestackFormatContext<'_>,
    annotation_span: Span,
) -> Option<TokenSpan> {
    let previous_token = previous_non_trivia_token_before_span(ctx, annotation_span)?;
    (previous_token.token.ty == TokenType::Comma).then_some(previous_token)
}

/// Return whether a boundary comment is on a member or index continuation seam.
fn separator_comment_is_member_or_index_continuation(
    following_token: Option<TokenSpan>,
    position: AnnotationPosition,
) -> bool {
    if position != AnnotationPosition::LinePostfixBoundary {
        return false;
    }

    following_token
        .is_some_and(|token| matches!(token.token.ty, TokenType::Dot | TokenType::OpenBracket))
}

/// Return one separator slash comment annotation source payload.
fn separator_line_comment_annotation_info(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    virtual_trailing_separator_policy: VirtualTrailingSeparatorPolicy,
) -> Option<(LocalNodeId<Comment>, bool, bool)> {
    let Annotation::Comment { node, position } = ctx.annotation(annotation_id) else {
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

    let comment = ctx.tree.get::<Comment>(node);
    if comment.style != CommentStyle::Slash {
        return None;
    }

    let annotation_span = ctx.annotation_span(annotation_id);
    let preceding_comma = separator_line_comment_preceding_comma(ctx, annotation_span);
    let following_token = next_non_trivia_token_after_annotation(ctx, annotation_id);
    if separator_comment_is_member_or_index_continuation(following_token, position) {
        return None;
    }

    let following_separator =
        following_token.is_some_and(|token| token.token.ty == TokenType::Comma);
    let following_close_brace =
        following_token.is_some_and(|token| token.token.ty == TokenType::CloseBrace);
    let following_close_bracket =
        following_token.is_some_and(|token| token.token.ty == TokenType::CloseBracket);
    let following_close_parenthesis =
        following_token.is_some_and(|token| token.token.ty == TokenType::CloseParenthesis);
    let has_virtual_trailing_separator = virtual_trailing_separator_policy
        != VirtualTrailingSeparatorPolicy::Disabled
        && preceding_comma.is_none()
        && !following_separator
        && (following_close_parenthesis || following_close_bracket)
        && position == AnnotationPosition::LinePostfixBoundary;

    if preceding_comma.is_none() && !following_separator && !has_virtual_trailing_separator {
        return None;
    }

    if preceding_comma.is_some() && following_close_brace && !following_separator {
        return None;
    }

    let is_own_line = separator_line_comment_is_own_line(
        ctx,
        annotation_id,
        annotation_span,
        preceding_comma,
        following_token.filter(|token| token.token.ty == TokenType::Comma),
        has_virtual_trailing_separator,
    );
    let has_blank_line_before_first_comment =
        separator_line_comment_has_blank_line_before_first_comment(
            ctx,
            annotation_span,
            preceding_comma,
            following_token.filter(|token| token.token.ty == TokenType::Comma),
            has_virtual_trailing_separator,
            false,
        );

    Some((node, is_own_line, has_blank_line_before_first_comment))
}

/// Return whether one separator comment starts on its own source line.
pub(crate) fn separator_line_comment_is_own_line(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
    annotation_span: Span,
    preceding_comma: Option<TokenSpan>,
    following_comma: Option<TokenSpan>,
    has_virtual_trailing_separator: bool,
) -> bool {
    if let Some(separator_token) = preceding_comma {
        let before_comment_span = Span::new(
            annotation_span.file,
            separator_token.span.end,
            annotation_span.start,
        );

        return ctx.has_newline(before_comment_span);
    }

    if let Some(separator_token) = following_comma {
        let after_comment_span = Span::new(
            annotation_span.file,
            annotation_span.end,
            separator_token.span.start,
        );
        return ctx.has_newline(after_comment_span);
    }

    if has_virtual_trailing_separator {
        return ctx.annotation_starts_on_own_line(annotation_id);
    }

    false
}

/// Return whether one separator comment has one preserved blank line before the first comment token.
pub(crate) fn separator_line_comment_has_blank_line_before_first_comment(
    ctx: &DestackFormatContext<'_>,
    annotation_span: Span,
    preceding_comma: Option<TokenSpan>,
    following_comma: Option<TokenSpan>,
    has_virtual_trailing_separator: bool,
    include_virtual_trailing_blank_line: bool,
) -> bool {
    if let Some(separator_token) = preceding_comma {
        let before_comment_span = Span::new(
            annotation_span.file,
            separator_token.span.end,
            annotation_span.start,
        );
        return ctx.has_blank_line(before_comment_span);
    }

    if let Some(separator_token) = following_comma {
        let before_separator_span = Span::new(
            annotation_span.file,
            annotation_span.end,
            separator_token.span.start,
        );
        return ctx.has_blank_line(before_separator_span);
    }

    if has_virtual_trailing_separator && include_virtual_trailing_blank_line {
        let Some(previous_token) = previous_non_trivia_token_before_span(ctx, annotation_span)
        else {
            return false;
        };
        let before_comment_span = Span::new(
            annotation_span.file,
            previous_token.span.end,
            annotation_span.start,
        );
        return ctx.has_blank_line(before_comment_span);
    }

    false
}

/// Return whether one annotation is a separator line comment.
/// Return one separator comment source from one annotation list and filter.
pub(crate) fn separator_line_comment_source_from_annotations<F, G>(
    ctx: &DestackFormatContext<'_>,
    annotations: &[LocalNodeId<Annotation>],
    mut annotation_info: F,
    mut annotation_allowed: G,
) -> Option<SeparatorLineCommentSource>
where
    F: FnMut(LocalNodeId<Annotation>) -> Option<(LocalNodeId<Comment>, bool, bool)>,
    G: FnMut(LocalNodeId<Annotation>) -> bool,
{
    for (index, annotation_id) in annotations.iter().copied().enumerate() {
        if !annotation_allowed(annotation_id) {
            continue;
        }

        let Some((comment_id, is_own_line, has_blank_line_before_first_comment)) =
            annotation_info(annotation_id)
        else {
            continue;
        };

        let mut comment_ids = smallvec::smallvec![comment_id];
        for next_annotation_id in annotations.iter().skip(index + 1).copied() {
            if !annotation_allowed(next_annotation_id) {
                break;
            }

            if matches!(ctx.annotation(next_annotation_id), Annotation::Blank { .. }) {
                continue;
            }

            let Some((next_comment_id, _, _)) = annotation_info(next_annotation_id) else {
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

/// Return one prefix comment style for one annotation.
fn annotation_prefix_comment_style(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<CommentStyle> {
    let Annotation::Comment { node, position } = ctx.annotation(annotation_id) else {
        return None;
    };
    if !matches!(
        position,
        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
    ) {
        return None;
    }

    let comment = ctx.tree.get::<Comment>(node);
    Some(comment.style)
}

/// Return one slash prefix comment node for one annotation.
fn annotation_slash_prefix_comment_node(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<LocalNodeId<Comment>> {
    let Annotation::Comment { node, .. } = ctx.annotation(annotation_id) else {
        return None;
    };
    (annotation_prefix_comment_style(ctx, annotation_id) == Some(CommentStyle::Slash))
        .then_some(node)
}

/// Return one comma token immediately before one annotation span.
fn annotation_preceding_comma_token(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<TokenSpan> {
    let annotation_span = ctx.annotation_span(annotation_id);
    let preceding_token = previous_non_trivia_token_before_span(ctx, annotation_span)?;
    (preceding_token.token.ty == TokenType::Comma).then_some(preceding_token)
}

/// Return one separator line comment source from following-argument prefix annotations.
fn separator_line_comment_source_from_following_prefix_annotations<F>(
    ctx: &DestackFormatContext<'_>,
    annotations: &[LocalNodeId<Annotation>],
    annotation_allowed: F,
) -> Option<SeparatorLineCommentSource>
where
    F: Fn(LocalNodeId<Annotation>) -> bool,
{
    // separator extraction from following prefixes only applies to slash comment clusters
    let has_non_slash_prefix_comment = annotations.iter().copied().any(|annotation_id| {
        annotation_allowed(annotation_id)
            && annotation_prefix_comment_style(ctx, annotation_id)
                .is_some_and(|style| style != CommentStyle::Slash)
    });
    if has_non_slash_prefix_comment {
        return None;
    }

    for (index, annotation_id) in annotations.iter().copied().enumerate() {
        if !annotation_allowed(annotation_id) {
            continue;
        }

        let Some(node) = annotation_slash_prefix_comment_node(ctx, annotation_id) else {
            continue;
        };

        let Some(preceding_token) = annotation_preceding_comma_token(ctx, annotation_id) else {
            continue;
        };

        let annotation_span = ctx.annotation_span(annotation_id);
        let before_comment_span = Span::new(
            annotation_span.file,
            preceding_token.span.end,
            annotation_span.start,
        );
        let is_own_line = ctx.has_newline(before_comment_span);
        let has_blank_line_before_first_comment = ctx.has_blank_line(before_comment_span);
        let mut comment_ids = smallvec::smallvec![node];
        for next_annotation_id in annotations.iter().skip(index + 1).copied() {
            if !annotation_allowed(next_annotation_id) {
                break;
            }

            if matches!(ctx.annotation(next_annotation_id), Annotation::Blank { .. }) {
                continue;
            }

            let Some(next_node) = annotation_slash_prefix_comment_node(ctx, next_annotation_id)
            else {
                break;
            };
            if annotation_preceding_comma_token(ctx, next_annotation_id).is_none() {
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
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Argument>> {
    let call_node_id = argument_parent_expression(ctx, argument_id)?;
    let argument_list = match ctx.tree.get(call_node_id) {
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

/// Return one virtual trailing-separator policy for one expression.
fn expression_virtual_trailing_separator_policy(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> VirtualTrailingSeparatorPolicy {
    match ctx.tree.get(expression_id) {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => {
            if dynamic_arguments.is_empty() {
                return VirtualTrailingSeparatorPolicy::Disabled;
            }
            VirtualTrailingSeparatorPolicy::BoundaryAnyLine
        }
        Expression::ArrayExpression { .. } => VirtualTrailingSeparatorPolicy::BoundaryAnyLine,
        _ => VirtualTrailingSeparatorPolicy::Disabled,
    }
}

/// Compute one separator line comment source attached to one argument.
fn compute_separator_line_comment_source(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<SeparatorLineCommentSource> {
    let call_node_id = argument_parent_expression(ctx, argument_id)?;
    let virtual_trailing_separator_policy =
        expression_virtual_trailing_separator_policy(ctx, call_node_id);
    let resolve_from_annotations = |annotations: &[LocalNodeId<Annotation>]| {
        separator_line_comment_source_from_annotations(
            ctx,
            annotations,
            |annotation_id| {
                separator_line_comment_annotation_info(
                    ctx,
                    annotation_id,
                    virtual_trailing_separator_policy,
                )
            },
            |_| true,
        )
    };

    if (ctx.has_postfix_annotation(argument_id) || ctx.has_boundary_comment_annotation(argument_id))
        && let Some(comment_source) = ctx
            .visit_annotations(argument_id, resolve_from_annotations)
            .flatten()
    {
        return Some(comment_source);
    }

    let value_id = argument_value_id(ctx.tree, argument_id);
    let value_span = ctx.span(value_id);
    if ctx.has_postfix_annotation(value_id) || ctx.has_boundary_comment_annotation(value_id) {
        let value_annotation_source = ctx
            .visit_annotations(value_id, |annotations| {
                separator_line_comment_source_from_annotations(
                    ctx,
                    annotations,
                    |annotation_id| {
                        separator_line_comment_annotation_info(
                            ctx,
                            annotation_id,
                            virtual_trailing_separator_policy,
                        )
                    },
                    |annotation_id| {
                        let annotation_span = ctx.annotation_span(annotation_id);
                        annotation_span.file == value_span.file
                            && annotation_span.start >= value_span.end
                    },
                )
            })
            .flatten();
        if value_annotation_source.is_some() {
            return value_annotation_source;
        }
    }

    let argument_span = ctx.span(argument_id);
    let seam_start = value_span.end;
    let call_span = ctx.span(call_node_id);
    if value_span.file != call_span.file || seam_start >= call_span.end {
        return None;
    }
    let following_argument_id = next_argument_in_expression(ctx, argument_id);
    let seam_end = following_argument_id
        .map(|next_argument_id| ctx.span(next_argument_id).start)
        .unwrap_or(call_span.end);
    if seam_start >= seam_end {
        return None;
    }

    if ctx.has_postfix_annotation(call_node_id) || ctx.has_boundary_comment_annotation(call_node_id)
    {
        let call_annotation_source = ctx
            .visit_annotations(call_node_id, |annotations| {
                separator_line_comment_source_from_annotations(
                    ctx,
                    annotations,
                    |annotation_id| {
                        separator_line_comment_annotation_info(
                            ctx,
                            annotation_id,
                            virtual_trailing_separator_policy,
                        )
                    },
                    |annotation_id| {
                        let annotation_span = ctx.annotation_span(annotation_id);
                        annotation_span.file == argument_span.file
                            && annotation_span.start >= seam_start
                            && annotation_span.end <= seam_end
                    },
                )
            })
            .flatten();
        if call_annotation_source.is_some() {
            return call_annotation_source;
        }
    }

    if ctx.has_prefix_annotation(call_node_id) {
        let call_prefix_annotation_source = ctx
            .visit_annotations(call_node_id, |annotations| {
                separator_line_comment_source_from_following_prefix_annotations(
                    ctx,
                    annotations,
                    |annotation_id| {
                        let annotation_span = ctx.annotation_span(annotation_id);
                        annotation_span.file == argument_span.file
                            && annotation_span.start >= seam_start
                            && annotation_span.end <= seam_end
                    },
                )
            })
            .flatten();
        if call_prefix_annotation_source.is_some() {
            return call_prefix_annotation_source;
        }
    }

    let following_argument_id = following_argument_id?;
    let following_argument_span = ctx.span(following_argument_id);
    if ctx.has_prefix_annotation(following_argument_id)
        && let Some(comment_source) = ctx
            .visit_annotations(following_argument_id, |annotations| {
                separator_line_comment_source_from_following_prefix_annotations(
                    ctx,
                    annotations,
                    |annotation_id| {
                        let annotation_span = ctx.annotation_span(annotation_id);
                        annotation_span.file == following_argument_span.file
                            && annotation_span.end <= following_argument_span.start
                    },
                )
            })
            .flatten()
    {
        return Some(comment_source);
    }

    let following_value_id = argument_value_id(ctx.tree, following_argument_id);
    let following_value_span = ctx.span(following_value_id);
    if !ctx.has_prefix_annotation(following_value_id) {
        return None;
    }

    ctx.visit_annotations(following_value_id, |annotations| {
        separator_line_comment_source_from_following_prefix_annotations(
            ctx,
            annotations,
            |annotation_id| {
                let annotation_span = ctx.annotation_span(annotation_id);
                annotation_span.file == following_value_span.file
                    && annotation_span.start <= following_value_span.start
            },
        )
    })
    .flatten()
}

/// Return one separator line comment source attached to one argument.
pub(crate) fn single_argument_separator_line_comment_source(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<SeparatorLineCommentSource> {
    ctx.separator_line_comment_source_cache(argument_id, || {
        compute_separator_line_comment_source(ctx, argument_id)
    })
}

/// Return whether one separator line comment source exists for one argument.
pub(crate) fn single_argument_separator_line_comment_source_exists(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.has_separator_line_comment_source(argument_id, || {
        compute_separator_line_comment_source(ctx, argument_id)
    })
}

/// Return whether one argument has non-separator boundary-postfix annotations.
fn argument_has_non_separator_boundary_postfix_annotation(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.visit_annotations(argument_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            let position = ctx.annotation(*annotation_id).position();
            let is_boundary_postfix = position == AnnotationPosition::LinePostfixBoundary;
            is_boundary_postfix
                && !matches!(ctx.annotation(*annotation_id), Annotation::Blank { .. })
                && separator_line_comment_annotation_info(
                    ctx,
                    *annotation_id,
                    VirtualTrailingSeparatorPolicy::BoundaryAnyLine,
                )
                .is_none()
        })
    })
    .unwrap_or(false)
}

/// Return whether one argument can render without separator line comment annotations.
pub(crate) fn argument_can_render_without_separator_line_comment(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let is_plain_positional = matches!(
        ctx.tree.get(argument_id),
        Argument::Positional {
            modifiers: None,
            ..
        }
    );
    if !is_plain_positional {
        return false;
    }

    if !ctx.has_boundary_comment_annotation(argument_id) {
        return true;
    }

    if argument_has_non_separator_boundary_postfix_annotation(ctx, argument_id) {
        return false;
    }

    true
}

/// Return whether one call argument has one separator line comment on a comma seam.
pub(crate) fn argument_has_separator_line_comment_annotation(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !ctx.has_prefix_annotation(argument_id)
        && !ctx.has_postfix_annotation(argument_id)
        && !ctx.has_boundary_comment_annotation(argument_id)
    {
        return false;
    }

    ctx.visit_annotations(argument_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            let Annotation::Comment { node, position } = ctx.annotation(*annotation_id) else {
                return false;
            };

            if !matches!(
                position,
                AnnotationPosition::LinePrefix
                    | AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
                    | AnnotationPosition::BlockPostfix
            ) {
                return false;
            }

            let comment = ctx.tree.get::<Comment>(node);
            if comment.style != CommentStyle::Slash {
                return false;
            }

            let has_preceding_separator =
                previous_non_whitespace_token_before_annotation(ctx, *annotation_id)
                    .is_some_and(|token| token.token.ty == TokenType::Comma);
            let has_following_separator =
                next_non_whitespace_token_after_annotation(ctx, *annotation_id)
                    .is_some_and(|token| token.token.ty == TokenType::Comma);
            has_preceding_separator || has_following_separator
        })
    })
    .unwrap_or(false)
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

/// Write one argument's prefix annotations while suppressing detached separator comment nodes.
fn write_argument_prefix_annotations_with_suppressed_separator_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    suppressed_comment_ids: Option<&[LocalNodeId<Comment>]>,
) -> FormatResult<()> {
    let mut items = annotation_render_items(f.context(), AnnotationCapture::AnyPrefix, argument_id);
    if let Some(comment_ids) = suppressed_comment_ids {
        items.retain(|item| {
            let annotation = f.context().annotation(item.annotation_id);
            !matches!(annotation, Annotation::Comment { node, .. } if comment_ids.contains(&node))
        });
    }

    write_annotation_render_items(f, AnnotationCapture::AnyPrefix, argument_id, &items)
}

/// Write one argument without separator line comments and with optional postfix blank suppression.
fn write_argument_without_separator_line_comment_with_prefix_filter_and_postfix_blank_policy<
    'ast,
>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
    suppressed_prefix_separator_comment_ids: Option<&[LocalNodeId<Comment>]>,
    suppress_postfix_blank_annotations: bool,
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
        write_argument_prefix_annotations_with_suppressed_separator_comments(
            f,
            argument_id,
            suppressed_prefix_separator_comment_ids,
        )?;
    }

    write_argument_with_modifiers_and_value(argument, force_break_after_lambda_prefix_comment, f)?;

    if suppress_postfix_blank_annotations {
        let mut items = annotation_render_items(
            f.context(),
            AnnotationCapture::AnyInfixOrPostfixExceptLinePostfixBoundary,
            argument_id,
        );
        items.retain(|item| {
            !(item.node_type == NodeType::Blank
                && matches!(
                    item.position,
                    AnnotationPosition::LinePostfix
                        | AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfixBoundary
                ))
        });
        write_annotation_render_items(
            f,
            AnnotationCapture::AnyInfixOrPostfixExceptLinePostfixBoundary,
            argument_id,
            &items,
        )?;
    } else {
        write!(
            f,
            [f.context()
                .any_infix_or_postfix_except_line_postfix_boundary_annotations(argument_id)]
        )?;
    }

    Ok(true)
}

/// Write one argument without separator line comments that are emitted at list level.
pub(crate) fn write_argument_without_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<bool> {
    write_argument_without_separator_line_comment_with_prefix_filter_and_postfix_blank_policy(
        f,
        argument_id,
        None,
        false,
    )
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

    if all_plain_call_arguments {
        for (index, argument_id) in leading_arguments.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }

            write_plain_call_argument(f, argument_id)?;
        }
    } else {
        for (index, argument_id) in leading_arguments.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }

            write_plain_call_argument_or_node(f, argument_id)?;
        }
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
    should_emit_trailing_separator: bool,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let body = format_with(|f| {
        if all_plain_call_arguments {
            for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
                if index > 0 {
                    write!(f, [token(","), soft_line_break_or_space()])?;
                }

                write_plain_call_argument(f, argument_id)?;
            }
        } else {
            for (index, argument_id) in dynamic_arguments.iter().copied().enumerate() {
                if index > 0 {
                    write!(f, [token(","), soft_line_break_or_space()])?;
                }

                write_plain_call_argument_or_node(f, argument_id)?;
            }
        }

        if should_emit_trailing_separator {
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
    let should_emit_trailing_separator = force_trailing_separator
        || (!disallow_trailing_separator
            && f.context().options.trailing_comma == TrailingComma::All);

    if use_separator_comment_multiline {
        format_separator_comment_multiline_layout(
            f,
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
            should_emit_trailing_separator,
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
    ctx: &DestackFormatContext<'_>,
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
        let last_comment_span = ctx.span(last_comment_id);
        let right_argument_span = ctx.span(right_argument_id);
        if let Some(between_span) = last_comment_span.gap_to(right_argument_span) {
            return ctx.has_blank_line(between_span);
        }
    }

    call_arguments_preserve_blank_line_between(ctx, left_argument_id, right_argument_id)
}

/// Collect separator line comment sources for each argument in source order.
fn collect_separator_line_comment_sources(
    ctx: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> Vec<Option<SeparatorLineCommentSource>> {
    dynamic_arguments
        .iter()
        .copied()
        .map(|argument_id| single_argument_separator_line_comment_source(ctx, argument_id))
        .collect::<Vec<_>>()
}

/// Write multiline call arguments with separator comment ownership and trailing separator policy.
fn write_separator_comment_multiline_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    separator_line_comment_sources: &[Option<SeparatorLineCommentSource>],
    should_emit_trailing_separator: bool,
) -> FormatResult<()> {
    for (index, argument_id) in dynamic_arguments.iter().enumerate() {
        if index > 0 {
            if preserve_blank_line_before_argument_with_separator_comments(
                f.context(),
                dynamic_arguments,
                separator_line_comment_sources,
                index,
            ) {
                write!(f, [empty_line()])?;
            } else {
                write!(f, [hard_line_break()])?;
            }
        }

        if index > 0
            && let Some(detached_separator_source) =
                separator_line_comment_sources[index - 1].as_ref()
                && detached_separator_source.detached_from_following_prefix
                    && write_argument_without_separator_line_comment_with_prefix_filter_and_postfix_blank_policy(
                        f,
                        *argument_id,
                        Some(detached_separator_source.comment_ids.as_slice()),
                        false,
                    )?
                {
                    if index + 1 < dynamic_arguments.len() || should_emit_trailing_separator {
                        write!(f, [token(",")])?;
                    }
                    continue;
                }

        if let Some(comment_source) = separator_line_comment_sources[index].as_ref()
            && write_argument_without_separator_line_comment_with_prefix_filter_and_postfix_blank_policy(
                f,
                *argument_id,
                None,
                true,
            )? {
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

/// Format call arguments with explicit multiline comment expansion.
fn format_comment_expanded_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    use_separator_comment_multiline: bool,
    use_single_plain_separator_comment_layout: bool,
    use_trailing_comma: bool,
    force_trailing_comma_for_separator_comment: bool,
) -> FormatResult<()> {
    if use_separator_comment_multiline {
        let should_emit_trailing_separator =
            use_trailing_comma || force_trailing_comma_for_separator_comment;
        // keep this path delegated to the shared separator-multiline renderer
        format_separator_comment_multiline_layout(
            f,
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
        if argument_can_render_without_separator_line_comment(f.context(), argument_id) {
            let separator_source =
                single_argument_separator_line_comment_source(f.context(), argument_id);
            if let Some(comment_source) = separator_source.as_ref() {
                return format_single_plain_argument_with_separator_line_comment(
                    f,
                    argument_id,
                    comment_source,
                );
            }
        }
    }

    let should_emit_trailing_separator =
        use_trailing_comma || force_trailing_comma_for_separator_comment;
    write_separator_comment_multiline_layout_body(
        f,
        dynamic_arguments,
        should_emit_trailing_separator,
    )
}

/// Write one separator-comment multiline call argument list body.
fn write_separator_comment_multiline_layout_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    should_emit_trailing_separator: bool,
) -> FormatResult<()> {
    let separator_line_comment_sources =
        collect_separator_line_comment_sources(f.context(), dynamic_arguments);

    write!(f, [token("("), hard_line_break()])?;
    write!(
        f,
        [block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                write_separator_comment_multiline_arguments(
                    f,
                    dynamic_arguments,
                    &separator_line_comment_sources,
                    should_emit_trailing_separator,
                )
            }
        ))]
    )?;
    write!(f, [hard_line_break(), token(")")])?;

    Ok(())
}

/// Format the shared separator-comment multiline list path and increment one counter key.
fn format_separator_comment_multiline_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    counter_key: &'static str,
    should_emit_trailing_separator: bool,
) -> FormatResult<()> {
    write_separator_comment_multiline_layout_body(
        f,
        dynamic_arguments,
        should_emit_trailing_separator,
    )?;

    f.context().increment_counter(counter_key, 1);

    Ok(())
}

/// Render one decided call argument layout.
pub(crate) fn format_call_argument_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout: CallArgumentLayout,
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
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if argument_satisfies_static_seam_comment_annotation_id(ctx, argument_id).is_some() {
        return false;
    }

    if argument_is_call_or_new(ctx, argument_id)
        && argument_has_only_separator_prefix_comment_cluster(ctx, argument_id)
        && !argument_is_first_in_call_or_new(ctx, argument_id)
    {
        return false;
    }

    if !argument_is_call_or_new(ctx, argument_id) {
        return true;
    }

    if argument_has_non_blank_prefix_annotation(ctx, argument_id) {
        return true;
    }

    if argument_has_blank_prefix_annotation_before_separator(ctx, argument_id) {
        return false;
    }

    if !argument_is_first_in_call_or_new(ctx, argument_id) {
        return true;
    }

    !argument_has_blank_prefix_annotation(ctx, argument_id)
}

/// Return one satisfies static seam line comment annotation id for this argument when present.
pub(crate) fn argument_satisfies_static_seam_comment_annotation_id(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Annotation>> {
    if !argument_is_first_static_argument_of_satisfies_right_path(ctx, argument_id) {
        return None;
    }

    ctx.find_annotation_id(argument_id, |annotation_id| {
        let Annotation::Comment {
            node,
            position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
        } = ctx.annotation(annotation_id)
        else {
            return None;
        };

        let comment = ctx.tree.get::<Comment>(node);
        (comment.style == CommentStyle::Slash).then_some(annotation_id)
    })
}

/// Return whether one argument is the first static argument in a satisfies rhs path with multiple arguments.
fn argument_is_first_static_argument_of_satisfies_right_path(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    if !argument_is_first_static_argument_of_multi_argument_path(ctx, argument_id) {
        return false;
    }

    let Some((path_expression_id, path_parent_type)) = ctx.parent(argument_id) else {
        return false;
    };
    if path_parent_type != NodeType::Expression {
        return false;
    }

    let path_expression_id = LocalNodeId::<Expression>::new(path_expression_id);
    let Some((type_binary_id, type_binary_parent_type)) = ctx.parent(path_expression_id) else {
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
    } = ctx.tree.get(type_binary_id)
    else {
        return false;
    };

    let right_expression_id = match ctx.tree.get(*right) {
        Expression::Parenthesized { expression } | Expression::Statement(expression) => *expression,
        _ => *right,
    };

    right_expression_id == path_expression_id
}

/// Return whether one argument is the first static argument of a multi-argument path static list.
fn argument_is_first_static_argument_of_multi_argument_path(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = ctx.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(parent_id);
    let static_arguments = match ctx.tree.get(expression_id) {
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
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = ctx.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = ctx.tree.get(LocalNodeId::<Expression>::new(parent_id));
    matches!(expression, Expression::Call { .. } | Expression::New { .. })
}

/// Return whether an argument is the first in its call or new argument list.
fn argument_is_first_in_call_or_new(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let Some((parent_id, parent_type)) = ctx.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let expression = ctx.tree.get(LocalNodeId::<Expression>::new(parent_id));
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
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.any_annotation_id(argument_id, |annotation_id| {
        match ctx.annotation(annotation_id) {
            Annotation::Blank { .. } => false,
            Annotation::Doc { position, .. }
            | Annotation::Comment { position, .. }
            | Annotation::Decorator { position, .. } => matches!(
                position,
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ),
        }
    })
}

/// Return whether one argument has only separator comment prefix annotations.
fn argument_has_only_separator_prefix_comment_cluster(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.visit_annotations(argument_id, |annotations| {
        let mut has_separator_comment = false;

        for annotation_id in annotations.iter().copied() {
            let annotation = ctx.annotation(annotation_id);
            if !matches!(
                annotation.position(),
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            ) {
                continue;
            }

            match annotation {
                Annotation::Blank { .. } => {}
                Annotation::Comment { node, .. } => {
                    let comment = ctx.tree.get::<Comment>(node);
                    if comment.style != CommentStyle::Slash {
                        return false;
                    }

                    let annotation_span = ctx.annotation_span(annotation_id);
                    let Some(preceding_token) =
                        previous_non_whitespace_token_before_span(ctx, annotation_span)
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
    })
    .unwrap_or(false)
}

/// Return whether an argument has a blank prefix annotation.
fn argument_has_blank_prefix_annotation(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.any_annotation_id(argument_id, |annotation_id| {
        matches!(
            ctx.annotation(annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

/// Return whether an argument has a blank prefix annotation before a separator.
fn argument_has_blank_prefix_annotation_before_separator(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.any_annotation_id(argument_id, |annotation_id| {
        let Annotation::Blank {
            position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
            ..
        } = ctx.annotation(annotation_id)
        else {
            return false;
        };

        next_non_whitespace_token_after_annotation(ctx, annotation_id)
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
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    ctx.any_annotation_id(argument_id, |annotation_id| {
        let Annotation::Comment { node, position } = ctx.annotation(annotation_id) else {
            return false;
        };
        if position != AnnotationPosition::BlockPrefix {
            return false;
        }

        let comment = ctx.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Star {
            return false;
        }

        let previous_token = previous_non_whitespace_token_before_annotation(ctx, annotation_id);
        let next_token = next_non_whitespace_token_after_annotation(ctx, annotation_id);
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
fn argument_contains_lambda_value(ctx: &DestackFormatContext<'_>, argument: &Argument) -> bool {
    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
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
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        argument_can_render_without_separator_line_comment, single_argument_requires_expanded_list,
        single_argument_separator_line_comment_source,
    };
    use crate::format::call::layout::{
        CallArgumentLayout, call_argument_layout, call_argument_layout_cache,
        call_force_expand_single_collection_for_type_binary_callee,
        call_force_expand_single_multiline_with_static_arguments,
    };
    use crate::{
        Annotation, DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions,
        TestFormatter, assert_format, assert_format_output_eq,
        assert_format_program_idempotent_with_file_type,
        assert_format_program_roundtrip_with_file_type, statement_list,
    };
    use destack_ast::{AnnotationPosition, Argument, Expression, LocalNodeId, NodeParentIndex};
    use destack_source::FileType;

    /// Build a formatter ctx for call-argument separator assertions.
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
        ctx: &DestackFormatContext<'_>,
        call_id: LocalNodeId<Expression>,
    ) -> Vec<LocalNodeId<Argument>> {
        let Expression::Call {
            dynamic_arguments, ..
        } = ctx.tree.get(call_id)
        else {
            panic!("expected call expression");
        };
        dynamic_arguments.clone()
    }

    /// Return the element ids from one array expression.
    fn array_elements(
        ctx: &DestackFormatContext<'_>,
        array_id: LocalNodeId<Expression>,
    ) -> Vec<LocalNodeId<Argument>> {
        let Expression::ArrayExpression { elements } = ctx.tree.get(array_id) else {
            panic!("expected array expression");
        };

        elements.clone()
    }

    /// Return whether one call has boundary comments in the cached layout scan.
    fn call_has_boundary_comments(
        ctx: &DestackFormatContext<'_>,
        call_id: LocalNodeId<Expression>,
        arguments: &[LocalNodeId<Argument>],
    ) -> bool {
        call_argument_layout_cache(ctx, call_id, arguments).has_boundary_comments
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
    fn test_separator_comment_source_and_plain_render_eligibility_for_conditional_fixture() {
        let source = "cb(
    overflowing ? 'absolute top-0' : 'relative', // inline-separator-marker
    parameter
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse conditional separator fixture");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let first_argument_id = arguments[0];
        let layout_cache = call_argument_layout_cache(&ctx, call_id, &arguments);
        let has_boundary_comments = call_has_boundary_comments(&ctx, call_id, &arguments);
        let layout = call_argument_layout(
            &ctx,
            call_id,
            &arguments,
            false,
            false,
            false,
            has_boundary_comments,
        );

        let separator_source =
            single_argument_separator_line_comment_source(&ctx, first_argument_id);
        assert!(
            separator_source.is_some(),
            "expected separator comment source for first argument",
        );
        assert!(
            layout_cache.has_line_comment_annotations,
            "expected call argument scan to detect line comment annotations",
        );
        assert!(
            !layout_cache.has_prefix_line_comment_annotations,
            "expected no prefix line comment annotations for this fixture",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&ctx, first_argument_id),
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
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let first_argument_id = arguments[0];
        let layout_cache = call_argument_layout_cache(&ctx, call_id, &arguments);
        let has_boundary_comments = call_has_boundary_comments(&ctx, call_id, &arguments);
        let layout = call_argument_layout(
            &ctx,
            call_id,
            &arguments,
            false,
            false,
            false,
            has_boundary_comments,
        );

        let separator_source =
            single_argument_separator_line_comment_source(&ctx, first_argument_id);
        assert!(
            separator_source.is_some(),
            "expected separator comment source for first argument",
        );
        assert!(
            layout_cache.has_line_comment_annotations,
            "expected call argument scan to detect line comment annotations",
        );
        assert!(
            !layout_cache.has_prefix_line_comment_annotations,
            "expected no prefix line comment annotations for this fixture",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&ctx, first_argument_id),
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
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let first_argument_id = arguments[0];

        let separator_source =
            single_argument_separator_line_comment_source(&ctx, first_argument_id);
        assert!(
            separator_source.is_none(),
            "property trailing comment should not be classified as an argument separator comment",
        );
    }

    #[test]
    fn test_separator_comment_source_ignores_member_dot_boundary_comment_cluster() {
        let source = "verylongidentifierthatwillwrap123123123123123(
  a.b
    // prettier-ignore
    // Some other comment here
    .c
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse call with member-dot boundary comments");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let first_argument_id = arguments[0];

        let separator_source =
            single_argument_separator_line_comment_source(&ctx, first_argument_id);
        assert!(
            separator_source.is_none(),
            "member-dot boundary comments should stay on the expression seam, not on separator rendering",
        );
    }

    #[test]
    fn test_separator_comment_source_ignores_nested_function_body_comment_cluster() {
        let source = "(function webpackUniversalModuleDefinition() {})(
  this,
  function (__WEBPACK_EXTERNAL_MODULE_85__, __WEBPACK_EXTERNAL_MODULE_115__) {
    return /******/ (function (modules) {
      // webpackBootstrap
      /******/
    })(
      /************************************************************************/
      /******/ [1],
    );
  },
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse wrapper call with nested function body comments");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let first_argument_id = arguments[0];

        let separator_source =
            single_argument_separator_line_comment_source(&ctx, first_argument_id);
        assert!(
            separator_source.is_none(),
            "nested function body comments should not become separator comment sources",
        );
    }

    #[test]
    fn test_call_layout_ignores_nested_collection_dangling_line_comment_for_separator_forcing() {
        let source = "expect(() => {}).toTriggerReadyStateChanges([
  // Nothing.
]);";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse call with nested collection dangling comment");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);

        let has_boundary_comments = call_has_boundary_comments(&ctx, call_id, &arguments);
        let layout = call_argument_layout(
            &ctx,
            call_id,
            &arguments,
            false,
            false,
            false,
            has_boundary_comments,
        );
        let CallArgumentLayout::ListDefault {
            force_trailing_separator,
            ..
        } = layout
        else {
            panic!("expected default list layout for nested collection dangling comment");
        };
        assert!(
            !force_trailing_separator,
            "nested collection dangling comments should not force call trailing separators",
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
        let ctx = context_from_formatter(&formatter);
        let elements = array_elements(&ctx, array_id);
        let last_element_id = *elements.last().expect("array should have a last element");

        let separator_source = single_argument_separator_line_comment_source(&ctx, last_element_id);
        let separator_source =
            separator_source.expect("expected separator source for last array element");
        assert!(
            separator_source.is_own_line,
            "expected trailing separator comment source to be own-line",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&ctx, last_element_id),
            "expected last array element to support separator comment list-level rendering",
        );
    }

    #[test]
    fn test_single_argument_call_detects_virtual_separator_comment_source_on_boundary_line_comment()
    {
        let source = "func(
  {} // comment
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse single argument call with trailing line comment");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let argument_id = arguments[0];

        let separator_source = single_argument_separator_line_comment_source(&ctx, argument_id);
        let separator_source =
            separator_source.expect("expected virtual separator source for boundary line comment");
        assert!(
            !separator_source.is_own_line,
            "same-line boundary comments should preserve inline separator rendering",
        );
    }

    #[test]
    fn test_multi_argument_call_detects_virtual_separator_comment_source_before_close() {
        let source = "call(
  value,
  other // marker
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse multi-argument call with trailing boundary line comment");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let last_argument_id = *arguments.last().expect("call should have a last argument");

        let separator_source =
            single_argument_separator_line_comment_source(&ctx, last_argument_id);
        let separator_source =
            separator_source.expect("expected virtual separator source before closing delimiter");
        assert!(
            !separator_source.is_own_line,
            "same-line trailing boundary comments should preserve inline separator rendering",
        );
    }

    #[test]
    fn test_multi_argument_call_virtual_separator_comment_formats_with_comma_before_comment() {
        let source = "call(
  value,
  other // marker
)";
        let expected = "call(
    value,
    other, // marker
);
";
        assert_format_program_roundtrip_with_file_type(
            source,
            expected,
            FileType::JavaScript,
            DestackFormatOptions::default(),
        );
    }

    #[test]
    fn test_single_argument_call_detects_closing_delimiter_separator_line_comment_source() {
        let source = "func(
  {}
  // marker
  ,
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse single argument call with closing-delimiter separator comment");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let argument_id = arguments[0];

        let separator_source = single_argument_separator_line_comment_source(&ctx, argument_id);
        let separator_source =
            separator_source.expect("expected separator source for closing-delimiter separator");
        assert!(
            separator_source.is_own_line,
            "expected closing-delimiter separator comment source to be own-line",
        );
    }

    #[test]
    fn test_single_argument_call_detects_virtual_separator_comment_source_before_close() {
        let source = "func(
  {}
  // marker
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse single argument call with virtual separator comment");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let argument_id = arguments[0];

        let separator_source = single_argument_separator_line_comment_source(&ctx, argument_id);
        let separator_source =
            separator_source.expect("expected virtual separator source before closing delimiter");
        assert!(
            separator_source.is_own_line,
            "expected virtual separator comment source to be own-line",
        );
    }

    #[test]
    fn test_single_argument_call_closing_delimiter_separator_uses_separator_multiline_layout() {
        let source = "foo(
  {}
  // Hi
  ,
)";
        let (formatter, call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse single argument call with closing-delimiter separator");
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let has_boundary_comments = call_has_boundary_comments(&ctx, call_id, &arguments);
        let layout = call_argument_layout(
            &ctx,
            call_id,
            &arguments,
            false,
            false,
            false,
            has_boundary_comments,
        );

        let CallArgumentLayout::ListDefault {
            use_separator_comment_multiline,
            ..
        } = layout
        else {
            panic!("expected list default layout for closing-delimiter separator comment");
        };
        assert!(
            use_separator_comment_multiline,
            "expected closing-delimiter separator comment to use separator multiline layout",
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
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let first_argument_id = arguments[0];
        let separator_source =
            single_argument_separator_line_comment_source(&ctx, first_argument_id);

        assert!(
            separator_source.is_some(),
            "expected separator source from call-level annotations",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&ctx, first_argument_id),
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
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let first_argument_id = arguments[0];
        let separator_source =
            single_argument_separator_line_comment_source(&ctx, first_argument_id);

        assert!(
            separator_source.is_some(),
            "expected separator source for callback argument before deps array",
        );
        assert!(
            argument_can_render_without_separator_line_comment(&ctx, first_argument_id),
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
        let first_ctx = context_from_formatter(&first_formatter);
        let first_arguments = call_dynamic_arguments(&first_ctx, first_call_id);
        let first_separator_source =
            single_argument_separator_line_comment_source(&first_ctx, first_arguments[0]);
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
        let second_ctx = context_from_formatter(&second_formatter);
        let second_arguments = call_dynamic_arguments(&second_ctx, second_call_id);
        let second_separator_source =
            single_argument_separator_line_comment_source(&second_ctx, second_arguments[0]);
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

    #[test]
    fn test_format_trailing_collection_last_argument_expansion_is_idempotent() {
        let source =
            r#"func(one, two, three, four, five, six, seven, eig, is, this, too, long, yes, []);"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
    }

    #[test]
    fn test_format_trailing_collection_object_with_inner_comment_is_idempotent() {
        let source = r#"func(one, two, three, four, five, six, seven, eig, is, this, too, long, yes, {
  // Comments
});"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
    }

    #[test]
    fn test_format_trailing_collection_last_argument_expansion_sequence_is_idempotent() {
        let source = r#"func(one, two, three, four, five, six, seven, eig, is, this, too, long, no, []);
func(one, two, three, four, five, six, seven, eig, is, this, too, long, yes, []);
func(one, two, three, four, five, six, seven, eig, is, this, too, long, yes, {
  // Comments
});"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
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
        let ctx = context_from_formatter(&formatter);
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let has_boundary_comments = call_has_boundary_comments(&ctx, call_id, &arguments);
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

    /// Single chain-valued arguments inside member chains should not force unstable expansion.
    #[test]
    fn test_format_member_chain_single_chain_argument_is_idempotent() {
        let source = r#"const sel = this.connections

  .concat(this.activities.concat(this.operators))
  .filter(x => x.selected);
"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
    }

    /// Member-chain nested calls should keep stable chain argument expansion state across passes.
    #[test]
    fn test_member_chain_nested_call_chain_expansion_signal_is_stable() {
        let source = r#"this.connections

  .concat(this.activities.concat(this.operators))
  .filter(x => x.selected)"#;

        let (first_formatter, root_call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse first member-chain call");
        let first_ctx = context_from_formatter(&first_formatter);
        let concat_call_id = nested_concat_call_id(&first_ctx, root_call_id);
        let concat_arguments = call_dynamic_arguments(&first_ctx, concat_call_id);
        let first_force_expand = super::call_arguments_force_expand_for_chain(
            &first_ctx,
            concat_call_id,
            &concat_arguments,
        );
        let first_has_blank_prefix = first_ctx.has_blank_prefix_annotation(concat_arguments[0]);
        let first_argument_value_id = super::argument_value_id(first_ctx.tree, concat_arguments[0]);
        let first_has_value_blank_prefix =
            first_ctx.has_blank_prefix_annotation(first_argument_value_id);

        let first_output = first_formatter.format(
            &root_call_id,
            DestackFormatOptions::default_with_line_width(80),
        );
        let (second_formatter, second_root_call_id) =
            TestFormatter::parse_with_file_type(&first_output, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse second member-chain call");
        let second_ctx = context_from_formatter(&second_formatter);
        let second_concat_call_id = nested_concat_call_id(&second_ctx, second_root_call_id);
        let second_concat_arguments = call_dynamic_arguments(&second_ctx, second_concat_call_id);
        let second_force_expand = super::call_arguments_force_expand_for_chain(
            &second_ctx,
            second_concat_call_id,
            &second_concat_arguments,
        );
        let second_has_blank_prefix =
            second_ctx.has_blank_prefix_annotation(second_concat_arguments[0]);
        let second_argument_value_id =
            super::argument_value_id(second_ctx.tree, second_concat_arguments[0]);
        let second_has_value_blank_prefix =
            second_ctx.has_blank_prefix_annotation(second_argument_value_id);

        assert_eq!(
            first_force_expand, second_force_expand,
            "nested concat call chain expansion should stay stable across passes",
        );
        assert_eq!(
            first_has_blank_prefix, second_has_blank_prefix,
            "nested concat argument blank-prefix ownership should stay stable across passes",
        );
        assert_eq!(
            first_has_value_blank_prefix, second_has_value_blank_prefix,
            "nested concat argument value blank-prefix ownership should stay stable across passes",
        );
    }

    /// Preserve-line call-argument clusters should keep statement spacing stable across passes.
    #[test]
    fn test_format_preserve_line_argument_cluster_spacing_is_idempotent() {
        let source = r#"differentArgTypes(

  () => {
    return true
  },

  isTrue ?
    doSomething() : 12,

);
moreArgTypes(

  [1, 2,
    3],

  {
    name: 'Hello World',
    age: 29
  },

  doSomething(

    // Hello world


    // Hello world again
    { name: 'Hello World', age: 34 },


    oneThing
      + anotherThing,

    // Comment

  ),

);"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
    }

    #[test]
    fn test_format_preserve_line_argument_list_fixture_slice_is_idempotent() {
        let source = r#"comments(
  // Comment

  /* Some comments */
  short,
  /* Another comment */

  short2, // Even more comments
);

evenMoreArgTypes(
  doSomething(
    { name: "Hello World", age: 34 },

    true,
  ),

  14,
);
"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
    }

    /// Preserve-line statement gap ownership should stay stable across formatting passes.
    #[test]
    fn test_preserve_line_argument_cluster_statement_gap_signals_are_stable() {
        let source = r#"differentArgTypes(

  () => {
    return true
  },

  isTrue ?
    doSomething() : 12,

);

moreArgTypes(

  [1, 2,
    3],

  {
    name: 'Hello World',
    age: 29
  },

  doSomething(

    // Hello world


    // Hello world again
    { name: 'Hello World', age: 34 },


    oneThing
      + anotherThing,

    // Comment

  ),

);
"#;

        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        let (first_formatter, first_roots) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
                .expect("parse first preserve-line argument cluster");
        let first_ctx = context_from_formatter(&first_formatter);
        let first_previous_has_blank_postfix =
            expression_has_blank_postfix_annotation(&first_ctx, first_roots[0]);
        let first_following_has_blank_prefix =
            first_ctx.has_blank_prefix_annotation(first_roots[1]);
        let first_output = first_formatter.format(&statement_list(&first_roots), options.clone());

        let (second_formatter, second_roots) =
            TestFormatter::parse_with_file_type(&first_output, FileType::JavaScript, |p| {
                Ok(p.parse())
            })
            .expect("parse second preserve-line argument cluster");
        let second_ctx = context_from_formatter(&second_formatter);
        let second_previous_has_blank_postfix =
            expression_has_blank_postfix_annotation(&second_ctx, second_roots[0]);
        let second_following_has_blank_prefix =
            second_ctx.has_blank_prefix_annotation(second_roots[1]);

        assert_eq!(
            first_previous_has_blank_postfix, second_previous_has_blank_postfix,
            "previous statement blank postfix signal should stay stable across passes",
        );
        assert_eq!(
            first_following_has_blank_prefix, second_following_has_blank_prefix,
            "following statement blank prefix signal should stay stable across passes",
        );
    }

    #[test]
    fn test_separator_comment_blank_line_signal_is_stable_for_preserve_line_slice() {
        let source = r#"comments(
  // Comment

  /* Some comments */
  short,
  /* Another comment */

  short2, // Even more comments
)"#;
        let (first_formatter, first_call_id) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse first preserve-line comments call");
        let first_ctx = context_from_formatter(&first_formatter);
        let first_arguments = call_dynamic_arguments(&first_ctx, first_call_id);
        let first_source =
            single_argument_separator_line_comment_source(&first_ctx, first_arguments[0]);

        let first_output = first_formatter.format(
            &first_call_id,
            DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
        );
        let (second_formatter, second_call_id) =
            TestFormatter::parse_with_file_type(&first_output, FileType::JavaScript, |p| {
                p.eat_expression(Default::default())
            })
            .expect("parse second preserve-line comments call");
        let second_ctx = context_from_formatter(&second_formatter);
        let second_arguments = call_dynamic_arguments(&second_ctx, second_call_id);
        let second_source =
            single_argument_separator_line_comment_source(&second_ctx, second_arguments[0]);

        assert_eq!(
            first_source.is_some(),
            second_source.is_some(),
            "separator comment source presence should stay stable across passes",
        );
    }

    #[test]
    fn test_preserve_line_argument_cluster_following_statement_gap_signals_are_stable() {
        let source = r#"moreArgTypes(
  [1, 2, 3],

  {
    name: "Hello World",
    age: 29,
  },

  doSomething(
    // Hello world

    // Hello world again
    { name: "Hello World", age: 34 },

    oneThing + anotherThing,

    // Comment
  ),
);

evenMoreArgTypes(
  doSomething(
    { name: "Hello World", age: 34 },

    true,
  ),

  14,

  1 + 2 - 90 / 80,

  !98 * 60 - 90,
);
"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        let (first_formatter, first_roots) =
            TestFormatter::parse_with_file_type(source, FileType::JavaScript, |p| Ok(p.parse()))
                .expect("parse first preserve-line following statement slice");
        let first_ctx = context_from_formatter(&first_formatter);
        let first_previous_has_blank_postfix =
            expression_has_blank_postfix_annotation(&first_ctx, first_roots[0]);
        let first_following_has_blank_prefix =
            first_ctx.has_blank_prefix_annotation(first_roots[1]);
        let first_output = first_formatter.format(&statement_list(&first_roots), options.clone());

        let (second_formatter, second_roots) =
            TestFormatter::parse_with_file_type(&first_output, FileType::JavaScript, |p| {
                Ok(p.parse())
            })
            .expect("parse second preserve-line following statement slice");
        let second_ctx = context_from_formatter(&second_formatter);
        let second_previous_has_blank_postfix =
            expression_has_blank_postfix_annotation(&second_ctx, second_roots[0]);
        let second_following_has_blank_prefix =
            second_ctx.has_blank_prefix_annotation(second_roots[1]);

        assert_eq!(
            first_previous_has_blank_postfix, second_previous_has_blank_postfix,
            "previous statement blank postfix signal should stay stable across passes",
        );
        assert_eq!(
            first_following_has_blank_prefix, second_following_has_blank_prefix,
            "following statement blank prefix signal should stay stable across passes",
        );
    }

    #[test]
    fn test_preserve_line_even_more_and_apply_slice_is_idempotent() {
        let source = r#"evenMoreArgTypes(
  doSomething(
    { name: "Hello World", age: 34 },

    true,
  ),

  14,

  1 + 2 - 90 / 80,

  !98 * 60 - 90,
);

foo.apply(
  null,

  // Array here
  [1, 2],
);
"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScript, options);
    }

    #[test]
    fn test_preserve_line_argument_list_jsx_mode_slice_is_idempotent() {
        let source = r#"comments(
  // Comment

  /* Some comments */
  short,
  /* Another comment */

  short2, // Even more comments

  /* Another comment */

  // Long Long Long Long Long Comment

  /* Long Long Long Long Long Comment */
  // Long Long Long Long Long Comment

  short3,
  // More comments
);

differentArgTypes(
  () => {
    return true;
  },

  isTrue ? doSomething() : 12,
);

moreArgTypes(
  [1, 2, 3],

  {
    name: "Hello World",
    age: 29,
  },

  doSomething(
    // Hello world

    // Hello world again
    { name: "Hello World", age: 34 },

    oneThing + anotherThing,

    // Comment
  ),
);

evenMoreArgTypes(
  doSomething(
    { name: "Hello World", age: 34 },

    true,
  ),

  14,

  1 + 2 - 90 / 80,

  !98 * 60 - 90,
);

foo.apply(
  null,

  // Array here
  [1, 2],
);

bar.on(
  "readable",

  () => {
    doStuff();
  },
);
"#;
        let options = DestackFormatOptions::default_with_line_width(80).with_indent_width(2);
        assert_format_program_idempotent_with_file_type(source, FileType::JavaScriptXml, options);
    }

    /// Return the nested `.concat(...)` call id from one `.filter(...)` chain expression.
    fn nested_concat_call_id(
        ctx: &DestackFormatContext<'_>,
        root_call_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let Expression::Call {
            left: filter_callee,
            ..
        } = ctx.tree.get(root_call_id)
        else {
            panic!("expected root call expression");
        };
        let Expression::Member {
            left: concat_call_id,
            ..
        } = ctx.tree.get(*filter_callee)
        else {
            panic!("expected filter member callee");
        };
        let Expression::Call { .. } = ctx.tree.get(*concat_call_id) else {
            panic!("expected nested concat call");
        };

        *concat_call_id
    }

    /// Return whether one expression has a blank postfix annotation.
    fn expression_has_blank_postfix_annotation(
        ctx: &DestackFormatContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let Some(annotation_ids) = ctx.annotations(expression_id) else {
            return false;
        };

        annotation_ids.iter().any(|annotation_id| {
            matches!(
                ctx.annotation(*annotation_id),
                Annotation::Blank {
                    position: AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            )
        })
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
        let ctx = context_from_formatter(&formatter);
        let Expression::Call { left, .. } = ctx.tree.get(root_call_id) else {
            panic!("expected root call expression");
        };
        let Expression::Member {
            left: concat_call_id,
            ..
        } = ctx.tree.get(*left)
        else {
            panic!("expected root call receiver member expression");
        };
        let call_id = *concat_call_id;
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let has_boundary_comments = call_has_boundary_comments(&ctx, call_id, &arguments);
        let single_argument_force_expand = single_argument_requires_expanded_list(&ctx, &arguments);
        let force_expand_single_multiline_with_static_arguments =
            call_force_expand_single_multiline_with_static_arguments(&ctx, call_id, &arguments);
        let force_expand_single_collection_for_type_binary_callee =
            call_force_expand_single_collection_for_type_binary_callee(&ctx, call_id, &arguments);
        let layout_cache = call_argument_layout_cache(&ctx, call_id, &arguments);
        let layout = call_argument_layout(
            &ctx,
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
        let ctx = context_from_formatter(&formatter);
        let Expression::Call { left, .. } = ctx.tree.get(root_call_id) else {
            panic!("expected root call expression");
        };
        let Expression::Member {
            left: expect_call_id,
            ..
        } = ctx.tree.get(*left)
        else {
            panic!("expected root call receiver member expression");
        };
        let call_id = *expect_call_id;
        let arguments = call_dynamic_arguments(&ctx, call_id);
        let has_boundary_comments = call_has_boundary_comments(&ctx, call_id, &arguments);
        let single_argument_force_expand = single_argument_requires_expanded_list(&ctx, &arguments);
        let force_expand_single_multiline_with_static_arguments =
            call_force_expand_single_multiline_with_static_arguments(&ctx, call_id, &arguments);
        let force_expand_single_collection_for_type_binary_callee =
            call_force_expand_single_collection_for_type_binary_callee(&ctx, call_id, &arguments);
        let layout = call_argument_layout(
            &ctx,
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

    #[test]
    fn test_format_optional_chain_single_boolean_argument_is_idempotent() {
        let source = r#"a = Boolean(
  a_long_long_long_long_condition || a_long_long_long_long_condition || a_long_long_long_long_condition,
)?.toString();"#;
        assert_format_program_idempotent_with_file_type(
            source,
            FileType::JavaScriptXml,
            DestackFormatOptions::default_with_line_width(80).with_indent_width(2),
        );
    }
}
