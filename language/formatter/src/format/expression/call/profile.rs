use super::super::*;
use super::analyze::*;
use super::classify::*;
use crate::CachedCallArgumentLayoutClass;
use crate::directive::any_ignore_range_for_nodes;
use crate::timing::tags;
use destack_ast::{Comment, CommentStyle};
use destack_fir::write;

/// Decide default list layout after profile-driven checks.
fn list_default_call_argument_layout_decision(
    force_expand: bool,
    has_line_comment_annotations: bool,
) -> CallArgumentLayoutDecision {
    CallArgumentLayoutDecision::ListDefault {
        force_expand,
        has_line_comment_annotations,
    }
}

/// Return whether comment profiling forces explicit multiline expansion.
fn call_argument_comments_force_expanded_layout(
    dynamic_arguments: &[LocalNodeId<Argument>],
    comment_profile: &CallArgumentCommentProfile,
) -> bool {
    if dynamic_arguments.len() <= 1 {
        return false;
    }

    comment_profile.has_line_comment_annotations
        || comment_profile.has_prefix_line_comment_annotations
}
/// Return whether trailing collection comments should force list expansion.
fn call_argument_trailing_collection_comment_force_expand(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    trailing_collection_argument: bool,
    has_line_comment_annotations: bool,
) -> bool {
    if dynamic_arguments.len() <= 1 || !trailing_collection_argument {
        return false;
    }

    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };

    let has_last_line_comment_annotation = has_line_comment_annotations
        && argument_has_line_comment_annotation(context, last_argument_id);
    let last_argument_span = context.get_span(last_argument_id);
    let has_last_source_comment = context.has_comment(last_argument_span);

    has_last_line_comment_annotation || has_last_source_comment
}

/// Return whether the trailing collection argument has non blank comment signals.
fn trailing_collection_argument_has_comment_signal(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let Some(last_argument_id) = dynamic_arguments.last().copied() else {
        return false;
    };

    if !argument_is_collection_literal(context, last_argument_id) {
        return false;
    }

    if context.has_non_blank_annotation(last_argument_id) {
        return true;
    }

    let last_argument_value_id = argument_value_id(context.tree, last_argument_id);
    context.has_non_blank_annotation(last_argument_value_id)
}
/// Build call argument comment profile only when annotations require it.
fn resolve_call_argument_comment_profile(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    has_any_argument_annotation: bool,
    has_call_infix_annotations: bool,
) -> CallArgumentCommentProfile {
    if has_any_argument_annotation || has_call_infix_annotations {
        let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_COMMENT_PROFILE);
        context.increment_counter("profile.call.arguments.comment_profile.calls", 1);
        return collect_call_argument_comment_profile(context, dynamic_arguments);
    }

    context.increment_counter(
        "profile.call.arguments.comment_profile.skip_no_annotation",
        1,
    );
    CallArgumentCommentProfile::default()
}

/// Decide post-hugged call argument layout.
fn decide_post_hugged_call_argument_layout(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    planner_state: CallArgumentPlannerState,
    has_boundary_comments: bool,
) -> CallArgumentLayoutDecision {
    // keep single callback arguments wrapped directly to avoid list-style trailing commas
    let use_single_callback_argument_inline = call_arguments_use_single_callback_argument_inline(
        context,
        dynamic_arguments,
        planner_state.has_call_infix_annotations,
        planner_state.force_expand_single_long_with_static_arguments,
        planner_state.force_expand_single_collection_for_type_binary_callee,
    );
    if use_single_callback_argument_inline {
        context.increment_counter("profile.call.arguments.path.single_callback_inline", 1);
        return CallArgumentLayoutDecision::InlineSingle;
    }

    // keep short single positional arguments inline
    let use_single_simple_argument = call_arguments_use_single_simple_argument_inline(
        context,
        dynamic_arguments,
        SingleSimpleArgumentInlineOptions {
            call_node_id,
            line_width: planner_state.line_width,
            call_has_static_arguments: planner_state.call_has_static_arguments,
            force_expand_single_long_with_static_arguments: planner_state
                .force_expand_single_long_with_static_arguments,
            force_expand_single_collection_for_type_binary_callee: planner_state
                .force_expand_single_collection_for_type_binary_callee,
            single_argument_force_expand: planner_state.single_argument_force_expand,
        },
        planner_state.argument_shape,
        planner_state.inline_call_len_without_static_arguments,
    );
    if use_single_simple_argument {
        context.increment_counter("profile.call.arguments.path.single_simple", 1);
        return CallArgumentLayoutDecision::InlineSingle;
    }

    // keep leading callback plus short tail calls inline
    let has_leading_block_callback_with_simple_tail =
        call_has_leading_block_callback_with_simple_tail(context, call_node_id, dynamic_arguments);
    let use_leading_block_callback_inline = has_leading_block_callback_with_simple_tail
        && expression_source_len(context, call_node_id) <= planner_state.line_width;
    if use_leading_block_callback_inline {
        context.increment_counter(
            "profile.call.arguments.path.leading_block_callback_inline",
            1,
        );
        return CallArgumentLayoutDecision::InlineAll;
    }

    // boundary comments should block aggressive inline and hug-last behavior
    let has_any_argument_annotation = planner_state.argument_shape.has_any_argument_annotation;

    // comments can force explicit multiline argument rendering
    let comment_profile = resolve_call_argument_comment_profile(
        context,
        dynamic_arguments,
        has_any_argument_annotation,
        planner_state.has_call_infix_annotations,
    );
    let has_line_comment_annotations = comment_profile.has_line_comment_annotations;
    if call_argument_comments_force_expanded_layout(dynamic_arguments, &comment_profile) {
        context.increment_counter("profile.call.arguments.path.comment_expanded", 1);
        return CallArgumentLayoutDecision::CommentExpanded(comment_profile);
    }

    // expansion profile drives default and hug-last fallback paths
    let expansion_profile = {
        let _timing =
            context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_EXPANSION_PROFILE);
        resolve_regular_call_argument_expansion_profile(context, call_node_id, dynamic_arguments)
    };
    let has_call_infix_annotations = expansion_profile.has_call_infix_annotations;
    let trailing_collection_argument = expansion_profile.trailing_collection_argument;
    let force_expand = expansion_profile.force_expand
        || has_boundary_comments
        || call_argument_trailing_collection_comment_force_expand(
            context,
            dynamic_arguments,
            trailing_collection_argument,
            has_line_comment_annotations,
        );

    // hug-last candidates can still end in default list rendering
    let can_consider_hug_last_argument = can_consider_hug_last_call_arguments(
        context,
        dynamic_arguments,
        has_line_comment_annotations || has_boundary_comments,
        has_call_infix_annotations,
    );
    if can_consider_hug_last_argument {
        let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_HUG_LAST);
        if let Some(layout) = resolve_hug_last_call_argument_layout(
            context,
            call_node_id,
            dynamic_arguments,
            planner_state,
            force_expand,
            trailing_collection_argument,
        ) {
            return match layout {
                HugLastCallArgumentLayout::Inline => CallArgumentLayoutDecision::InlineAll,
                HugLastCallArgumentLayout::ListDefault => {
                    context.increment_counter("profile.call.arguments.path.list_default", 1);
                    list_default_call_argument_layout_decision(
                        force_expand,
                        has_line_comment_annotations,
                    )
                }
            };
        }
    }

    context.increment_counter("profile.call.arguments.path.list_default", 1);
    list_default_call_argument_layout_decision(force_expand, has_line_comment_annotations)
}

/// Format default call arguments via direct argument emission for annotation-free lists.
fn format_fast_default_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let allow_trailing_comma = f.context().options.trailing_comma == TrailingComma::All;
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
    group_id: GroupId,
    dynamic_arguments: &[LocalNodeId<Argument>],
    force_expand: bool,
    has_line_comment_annotations: bool,
    has_any_argument_annotation: bool,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    let is_single_argument = dynamic_arguments.len() == 1;
    let single_argument_id = if is_single_argument {
        Some(dynamic_arguments[0])
    } else {
        None
    };
    let has_single_template_literal_argument = if is_single_argument {
        single_argument_id
            .is_some_and(|argument_id| argument_is_template_literal(f.context(), argument_id))
    } else {
        false
    };
    let trailing_collection_has_comment_signal =
        trailing_collection_argument_has_comment_signal(f.context(), dynamic_arguments);
    let single_argument_has_source_separator_line_comment_annotation = if is_single_argument {
        single_argument_id.is_some_and(|argument_id| {
            argument_has_source_separator_line_comment_annotation(f.context(), argument_id)
        })
    } else {
        false
    };
    let can_use_plain_default_fast_path = !f.context().has_ignore_directive_markers()
        && dynamic_arguments.len() > 1
        && !has_any_argument_annotation
        && !has_line_comment_annotations
        && !trailing_collection_has_comment_signal
        && !is_single_argument;

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_LIST_DEFAULT);
    if can_use_plain_default_fast_path {
        f.context()
            .increment_counter("profile.call.arguments.path.list_default_plain_fast", 1);
        return format_fast_default_call_argument_list(
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

    if dynamic_arguments.len() == 1
        && argument_is_collection_literal(f.context(), dynamic_arguments[0])
    {
        list.disallow_trailing_separator();
    }
    if has_single_template_literal_argument
        && !argument_is_interpolated_template_literal(f.context(), dynamic_arguments[0])
    {
        list.disallow_trailing_separator();
    }
    if trailing_collection_has_comment_signal
        || single_argument_has_source_separator_line_comment_annotation
    {
        list.disallow_trailing_separator();
    }

    write!(f, [list])
}

/// Format call arguments with explicit multiline comment expansion.
fn format_comment_expanded_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let has_trailing_collection_with_source_comment =
        trailing_collection_argument_has_comment_signal(f.context(), dynamic_arguments);
    let has_single_argument_source_separator_line_comment_annotation = dynamic_arguments.len() == 1
        && argument_has_source_separator_line_comment_annotation(f.context(), dynamic_arguments[0]);
    let use_trailing_comma = f.context().options.trailing_comma == TrailingComma::All
        && !has_trailing_collection_with_source_comment
        && !has_single_argument_source_separator_line_comment_annotation;

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

                    write!(f, [group(argument_id)])?;
                    if index + 1 < dynamic_arguments.len() || use_trailing_comma {
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

/// Write one single argument wrapped in call parentheses.
fn write_single_call_argument_inline_wrapped<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    write!(f, [token("(")])?;
    write_plain_call_argument_or_node(f, argument_id)?;
    write!(f, [token(")")])
}

/// Render one decided call argument layout.
fn format_call_argument_layout_decision<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_decision: CallArgumentLayoutDecision,
    render_state: CallArgumentLayoutRenderState,
) -> FormatResult<()> {
    match layout_decision {
        CallArgumentLayoutDecision::InlineAll => write_inline_call_argument_list(
            f,
            dynamic_arguments,
            render_state.all_plain_call_arguments,
        ),
        CallArgumentLayoutDecision::InlineSingle => {
            debug_assert_eq!(dynamic_arguments.len(), 1);
            let Some(argument_id) = dynamic_arguments.first().copied() else {
                debug_assert!(false, "single inline layout requires one argument");
                return Ok(());
            };
            write_single_call_argument_inline_wrapped(f, argument_id)
        }
        CallArgumentLayoutDecision::CommentExpanded(_comment_profile) => {
            format_comment_expanded_call_argument_list(f, dynamic_arguments)
        }
        CallArgumentLayoutDecision::ListDefault {
            force_expand,
            has_line_comment_annotations,
        } => format_default_call_argument_list(
            f,
            render_state.group_id,
            dynamic_arguments,
            force_expand,
            has_line_comment_annotations,
            render_state.has_any_argument_annotation,
            render_state.all_plain_call_arguments,
        ),
    }
}

/// Format call arguments with list-group awareness.
pub(crate) fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS);

    // set up the list group id for conditional formatting
    let group_id = f.group_id("call_args");
    let previous_group_id = f.context().current_argument_group_id;
    f.context_mut().current_argument_group_id = Some(group_id);
    let result = format_call_arguments_with_group(f, call_node_id, dynamic_arguments, group_id);
    f.context_mut().current_argument_group_id = previous_group_id;

    result
}

/// Return whether call arguments can use the no-annotation multi-argument fast path.
fn call_arguments_use_no_annotation_multi_argument_fast_path(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_class: CachedCallArgumentLayoutClass,
    planner_base_state: CallArgumentPlannerBaseState,
    has_boundary_comments: bool,
) -> bool {
    if dynamic_arguments.len() <= 1 {
        return false;
    }

    if has_boundary_comments {
        return false;
    }

    if !call_argument_layout_class_is_simple_multi_unannotated(layout_class) {
        return false;
    }

    if !planner_base_state
        .argument_shape
        .all_single_line_and_unannotated
        || planner_base_state.argument_shape.is_multiline_in_source
    {
        return false;
    }

    if !layout_class.trailing_collection_argument {
        return true;
    }

    !trailing_collection_argument_has_comment_signal(context, dynamic_arguments)
}

/// Format call arguments with the no-annotation multi-argument fast path.
fn format_no_annotation_multi_argument_fast_path<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
    line_width: usize,
    all_plain_call_arguments: bool,
) -> FormatResult<()> {
    if expression_source_len(f.context(), call_node_id) <= line_width {
        f.context()
            .increment_counter("profile.call.arguments.path.no_annotation_inline_fast", 1);
        write_inline_call_argument_list(f, dynamic_arguments, all_plain_call_arguments)
    } else {
        f.context()
            .increment_counter("profile.call.arguments.path.no_annotation_grouped_fast", 1);
        let mut list = list_like("(", ")", ",", dynamic_arguments);
        list.with_group_id(Some(group_id)).should_expand(false);
        write!(f, [list])
    }
}

/// Return whether call arguments can use the forced hug-last fast path.
fn call_arguments_use_forced_hug_last_inline_fast_path(
    dynamic_arguments: &[LocalNodeId<Argument>],
    layout_class: CachedCallArgumentLayoutClass,
    has_boundary_comments: bool,
) -> bool {
    dynamic_arguments.len() > 1 && !has_boundary_comments && layout_class.force_hug_last_inline
}

/// Return whether empty call infix annotations should expand across lines.
fn empty_call_infix_requires_multiline(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .with_annotations(call_node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.get_annotation(*annotation_id);
                if annotation.position() != AnnotationPosition::BlockInfix {
                    return false;
                }

                let annotation_span = context.get_annotation_span(*annotation_id);
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

/// Format call arguments with an active list group id.
fn format_single_call_argument_with_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    argument_id: LocalNodeId<Argument>,
    group_id: GroupId,
) -> FormatResult<()> {
    let single_argument = [argument_id];
    let has_boundary_comments =
        call_arguments_have_boundary_comments(f.context(), call_node_id, &single_argument);

    // shared single argument planner base state
    let has_call_infix_annotations = call_has_non_blank_infix_annotation(f.context(), call_node_id);
    let argument_annotation_profile = f.context().argument_annotation_profile(argument_id);
    let has_any_argument_annotation = argument_annotation_profile.has_prefix_annotation
        || argument_annotation_profile.has_comment
        || argument_annotation_profile.has_line_comment
        || argument_annotation_profile.has_prefix_line_comment;
    let is_multiline_in_source = f.context().node_has_newline(argument_id);
    let argument_shape = CallArgumentShape {
        has_any_argument_annotation,
        is_multiline_in_source,
        all_single_line_and_unannotated: !has_any_argument_annotation && !is_multiline_in_source,
    };
    let planner_base_state = CallArgumentPlannerBaseState {
        line_width: usize::from(f.context().options.line_width),
        call_has_static_arguments: call_has_static_arguments(f.context(), call_node_id),
        has_call_infix_annotations,
        argument_shape,
    };
    let single_argument_force_expand =
        single_argument_requires_expanded_list(f.context(), &single_argument);
    let inline_call_len_without_static_arguments = if planner_base_state.call_has_static_arguments {
        None
    } else {
        resolve_inline_call_len_without_static_arguments(
            f.context(),
            call_node_id,
            &single_argument,
            planner_base_state.call_has_static_arguments,
        )
    };

    // keep very common single simple arguments wrapped inline
    let use_single_simple_argument_fast_path = call_arguments_use_single_simple_argument_fast_path(
        f.context(),
        &single_argument,
        SingleSimpleArgumentFastPathOptions {
            line_width: planner_base_state.line_width,
            call_has_static_arguments: planner_base_state.call_has_static_arguments,
            has_call_infix_annotations: planner_base_state.has_call_infix_annotations,
            single_argument_force_expand,
        },
        planner_base_state.argument_shape,
        inline_call_len_without_static_arguments,
    ) && !has_boundary_comments;
    if use_single_simple_argument_fast_path {
        f.context()
            .increment_counter("profile.call.arguments.single_simple.fast_path", 1);
        f.context()
            .increment_counter("profile.call.arguments.path.single_simple_fast_path", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // keep closure-cast object arguments inline in call-then-chain no-semi seams
    let use_inline_closure_cast_object_argument =
        !has_boundary_comments && argument_is_inline_closure_cast_object(f.context(), argument_id);
    if use_inline_closure_cast_object_argument {
        f.context()
            .increment_counter("profile.call.arguments.path.inline_closure_cast_object", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // preserve inline callback single argument rendering
    let force_expand_single_long_with_static_arguments =
        call_force_expand_single_long_with_static_arguments(
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
    let use_single_callback_argument_inline = call_arguments_use_single_callback_argument_inline(
        f.context(),
        &single_argument,
        planner_base_state.has_call_infix_annotations,
        force_expand_single_long_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
    ) && !has_boundary_comments;
    if use_single_callback_argument_inline {
        f.context()
            .increment_counter("profile.call.arguments.path.single_callback_inline", 1);
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // keep single collection arguments on hugged path when allowed
    let force_hugged_expand =
        call_should_force_hugged_expand(force_expand_single_collection_for_type_binary_callee);
    let has_hug_blocking_comment_annotation =
        argument_has_callback_blocking_comment_annotation(f.context(), argument_id);
    let can_use_hugged = !has_boundary_comments
        && !has_any_argument_annotation
        && !argument_has_multiline_prefix_annotation(f.context(), argument_id)
        && !has_hug_blocking_comment_annotation
        && !planner_base_state.has_call_infix_annotations
        && !force_expand_single_long_with_static_arguments
        && !argument_is_lambda_expression(f.context(), argument_id)
        && !argument_is_function_expression(f.context(), argument_id)
        && !argument_is_interpolated_template_literal(f.context(), argument_id);
    if can_use_hugged {
        let used_hugged = format_hugged(
            f,
            &single_argument,
            HugOptions::CALL,
            Some(group_id),
            force_hugged_expand,
        )?;
        if used_hugged {
            f.context()
                .increment_counter("profile.call.arguments.path.hugged", 1);
            return Ok(());
        }
    }

    // run profiled single argument planner decision
    let planner_state = build_call_argument_planner_state(
        f.context(),
        call_node_id,
        &single_argument,
        planner_base_state,
        single_argument_force_expand,
        force_expand_single_long_with_static_arguments,
        inline_call_len_without_static_arguments,
    );
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED);
    let layout_decision = {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_DECIDE);
        decide_post_hugged_call_argument_layout(
            f.context(),
            call_node_id,
            &single_argument,
            planner_state,
            has_boundary_comments,
        )
    };

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_RENDER);
    let render_state = CallArgumentLayoutRenderState {
        group_id,
        has_any_argument_annotation: planner_base_state
            .argument_shape
            .has_any_argument_annotation,
        all_plain_call_arguments: argument_is_plain_call_argument(f.context(), argument_id),
    };
    format_call_argument_layout_decision(f, &single_argument, layout_decision, render_state)
}

/// Format call arguments with an active list group id.
fn format_call_arguments_with_group<'ast>(
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
        return Ok(());
    }

    // ignore ranges: route through list_like so raw span preservation stays consistent
    if f.context().has_ignore_directive_markers() {
        let comment_tokens = f.context().comment_tokens();
        let has_ignore_ranges =
            any_ignore_range_for_nodes(f.context(), dynamic_arguments, comment_tokens);
        if has_ignore_ranges {
            f.context()
                .increment_counter("profile.call.arguments.path.ignore_ranges_list_like", 1);
            let mut list = list_like("(", ")", ",", dynamic_arguments);
            list.with_group_id(Some(group_id)).force_expand();
            write!(f, [list])?;
            return Ok(());
        }
    }

    if dynamic_arguments.len() == 1 {
        return format_single_call_argument_with_group(
            f,
            call_node_id,
            dynamic_arguments[0],
            group_id,
        );
    }

    let layout_class =
        resolve_call_argument_layout_class(f.context(), call_node_id, dynamic_arguments);
    let planner_base_state =
        build_call_argument_planner_base_state(f.context(), call_node_id, layout_class);
    let all_plain_call_arguments = layout_class.all_plain_call_arguments;
    let has_boundary_comments =
        call_arguments_have_boundary_comments(f.context(), call_node_id, dynamic_arguments);

    // keep compact, no-annotation lists on a cheap inline or grouped path
    let use_no_annotation_multi_argument_fast_path =
        call_arguments_use_no_annotation_multi_argument_fast_path(
            f.context(),
            dynamic_arguments,
            layout_class,
            planner_base_state,
            has_boundary_comments,
        );
    if use_no_annotation_multi_argument_fast_path {
        format_no_annotation_multi_argument_fast_path(
            f,
            call_node_id,
            dynamic_arguments,
            group_id,
            planner_base_state.line_width,
            all_plain_call_arguments,
        )?;
        return Ok(());
    }

    // frequent hug-last callback and collection tails can skip profiled layout
    let use_forced_hug_last_inline_fast_path = call_arguments_use_forced_hug_last_inline_fast_path(
        dynamic_arguments,
        layout_class,
        has_boundary_comments,
    );
    if use_forced_hug_last_inline_fast_path {
        f.context()
            .increment_counter("profile.call.arguments.path.hug_last_forced", 1);
        write_inline_call_argument_list(f, dynamic_arguments, all_plain_call_arguments)?;
        return Ok(());
    }

    let inline_call_len_without_static_arguments = None;
    let planner_state = build_call_argument_planner_state(
        f.context(),
        call_node_id,
        dynamic_arguments,
        planner_base_state,
        false,
        false,
        inline_call_len_without_static_arguments,
    );
    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED);
    let layout_decision = {
        let _timing = f
            .context()
            .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_DECIDE);
        decide_post_hugged_call_argument_layout(
            f.context(),
            call_node_id,
            dynamic_arguments,
            planner_state,
            has_boundary_comments,
        )
    };

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_ARGUMENTS_PROFILED_RENDER);
    let render_state = CallArgumentLayoutRenderState {
        group_id,
        has_any_argument_annotation: layout_class.has_any_argument_annotation,
        all_plain_call_arguments,
    };
    format_call_argument_layout_decision(f, dynamic_arguments, layout_decision, render_state)
}

/// Return whether a call should expand its argument list when formatted in a chain.
pub(crate) fn call_arguments_force_expand_for_chain(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    if let Some(force_expand) = context.cached_call_argument_chain_force_expand(call_node_id) {
        context.increment_counter("profile.call_arguments.chain.fast_cache.hits", 1);
        context.increment_counter(
            if force_expand {
                "profile.call_arguments.chain.force_expand.true"
            } else {
                "profile.call_arguments.chain.force_expand.false"
            },
            1,
        );
        return force_expand;
    }
    context.increment_counter("profile.call_arguments.chain.fast_cache.misses", 1);

    if dynamic_arguments.is_empty() {
        context.cache_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("profile.call_arguments.chain.fast_false.empty", 1);
        context.increment_counter("profile.call_arguments.chain.force_expand.false", 1);
        return false;
    }

    let layout_class = resolve_call_argument_layout_class(context, call_node_id, dynamic_arguments);
    let should_bypass_simple_fast_false = dynamic_arguments.len() == 1
        && single_argument_requires_expanded_list(context, dynamic_arguments);
    let can_use_simple_fast_false =
        call_argument_layout_class_is_simple_multi_unannotated(layout_class)
            && !should_bypass_simple_fast_false;
    if can_use_simple_fast_false {
        context.cache_call_argument_chain_force_expand(call_node_id, false);
        context.increment_counter("profile.call_arguments.chain.fast_false.simple", 1);
        context.increment_counter("profile.call_arguments.chain.force_expand.false", 1);
        return false;
    }

    let force_expand =
        resolve_chain_call_argument_force_expand(context, call_node_id, dynamic_arguments);
    context.cache_call_argument_chain_force_expand(call_node_id, force_expand);
    force_expand
}
