use crate::format::analysis::{
    argument_has_line_comment_annotation, argument_is_collection_literal,
    argument_is_compact_inline_callback, argument_is_inline_closure_cast_object,
    argument_is_interpolated_template_literal, argument_is_trivial_unannotated_non_lambda_value,
    call_has_static_arguments, next_non_whitespace_token_after_annotation,
    previous_non_whitespace_token_before_annotation, previous_non_whitespace_token_before_span,
};
use crate::format::call::layout::{
    argument_has_callback_blocking_comment_annotation, call_argument_layout_facts,
    call_force_expand_single_collection_for_type_binary_callee,
    call_force_expand_single_multiline_with_static_arguments, chain_call_argument_force_expand,
    single_argument_requires_expanded_list,
};
use crate::format::chain::{argument_value_id_if_present, transparent_inner_expression};
use crate::format::collection::list_like;
use crate::format::collection::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::format::directive::{any_ignore_range_for_nodes, node_has_ignore_directive};
use crate::format::expression::{
    format_expression, format_static_argument_list, is_trivial_expression,
};
use crate::format::tree::{
    argument_is_block_callback, argument_is_template_literal, has_multiline_jsx_argument,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Argument, Comment, CommentStyle, Declaration, Expression, FunctionKind,
    LocalNodeId, NodeType, PostfixPosition, TokenType, TypeBinaryOperator,
};
use destack_fir::format::{Buffer, FormatResult, GroupId};
use destack_fir::prelude::{block_indent, hard_line_break, space, token};
use destack_fir::write;

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

    let (
        has_call_infix_annotations,
        _has_boundary_comments,
        _has_any_argument_annotation,
        _all_single_line_and_unannotated,
        all_compact_simple_unannotated,
        _has_line_comment_annotations,
        _arrow_argument_count,
        _function_argument_count,
        _has_complex_non_callback_argument,
        _trailing_collection_argument,
    ) = call_argument_layout_facts(ctx, call_node_id, dynamic_arguments);
    let should_bypass_simple_false = dynamic_arguments.len() == 1
        && single_argument_requires_expanded_list(ctx, dynamic_arguments);
    let can_use_simple_false = !has_call_infix_annotations
        && all_compact_simple_unannotated
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
    let (
        has_call_infix_annotations,
        has_boundary_comments,
        has_any_argument_annotation,
        all_single_line_and_unannotated,
        all_compact_simple_unannotated,
        has_line_comment_annotations,
        arrow_argument_count,
        function_argument_count,
        has_complex_non_callback_argument,
        trailing_collection_argument,
    ) = call_argument_layout_facts(f.context(), call_node_id, &single_argument);
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
    // simple short-circuit path
    let use_single_simple_short_circuit = !has_boundary_comments
        && !call_has_static_arguments
        && !has_call_infix_annotations
        && !single_argument_force_expand
        && all_single_line_and_unannotated
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
    if !has_boundary_comments && argument_is_inline_closure_cast_object(f.context(), argument_id) {
        write_single_call_argument_inline_wrapped(f, argument_id)?;
        return Ok(());
    }

    // use full single argument layout selection and rendering
    format_decided_call_argument_list(
        f,
        call_node_id,
        &single_argument,
        group_id,
        has_call_infix_annotations,
        has_any_argument_annotation,
        all_compact_simple_unannotated,
        has_line_comment_annotations,
        arrow_argument_count,
        function_argument_count,
        has_complex_non_callback_argument,
        trailing_collection_argument,
        single_argument_force_expand,
        force_expand_single_multiline_with_static_arguments,
        force_expand_single_collection_for_type_binary_callee,
        has_boundary_comments,
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
            let mut list = list_like("(", ")", ",", dynamic_arguments);
            list.with_group_id(Some(group_id)).force_expand();
            write!(f, [list])?;
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
    let (
        has_call_infix_annotations,
        has_boundary_comments,
        has_any_argument_annotation,
        _all_single_line_and_unannotated,
        all_compact_simple_unannotated,
        has_line_comment_annotations,
        arrow_argument_count,
        function_argument_count,
        has_complex_non_callback_argument,
        trailing_collection_argument,
    ) = call_argument_layout_facts(f.context(), call_node_id, dynamic_arguments);
    format_decided_call_argument_list(
        f,
        call_node_id,
        dynamic_arguments,
        group_id,
        has_call_infix_annotations,
        has_any_argument_annotation,
        all_compact_simple_unannotated,
        has_line_comment_annotations,
        arrow_argument_count,
        function_argument_count,
        has_complex_non_callback_argument,
        trailing_collection_argument,
        false,
        false,
        false,
        has_boundary_comments,
    )
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
    let mut list = list_like("(", ")", ",", dynamic_arguments);
    list.with_group_id(Some(group_id))
        .should_expand(force_expand);

    if disallow_trailing_separator {
        list.disallow_trailing_separator();
    }

    write!(f, [list])
}

/// Decide and render one call argument list directly.
pub(crate) fn format_decided_call_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
    group_id: GroupId,
    has_call_infix_annotations: bool,
    has_any_argument_annotation: bool,
    all_compact_simple_unannotated: bool,
    has_line_comment_annotations: bool,
    arrow_argument_count: usize,
    function_argument_count: usize,
    has_complex_non_callback_argument: bool,
    trailing_collection_argument: bool,
    single_argument_force_expand: bool,
    force_expand_single_multiline_with_static_arguments: bool,
    force_expand_single_collection_for_type_binary_callee: bool,
    has_boundary_comments: bool,
) -> FormatResult<()> {
    if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];

        if !force_expand_single_multiline_with_static_arguments
            && !force_expand_single_collection_for_type_binary_callee
            && !single_argument_force_expand
            && !has_any_argument_annotation
            && argument_is_trivial_unannotated_non_lambda_value(f.context(), argument_id)
        {
            return write_single_call_argument_inline_wrapped(f, argument_id);
        }

        if !has_call_infix_annotations
            && !force_expand_single_multiline_with_static_arguments
            && !force_expand_single_collection_for_type_binary_callee
            && !has_boundary_comments
            && !has_any_argument_annotation
            && argument_is_compact_inline_callback(f.context(), argument_id)
        {
            return write_single_call_argument_inline_wrapped(f, argument_id);
        }
    }
    let (force_expand_regular, trailing_collection_argument) = if dynamic_arguments.is_empty() {
        (false, false)
    } else if dynamic_arguments.len() == 1 {
        let argument_id = dynamic_arguments[0];
        let has_line_comment_annotations = f
            .context()
            .argument_has_line_comment_annotation(argument_id);
        let trailing_collection_argument = argument_is_collection_literal(f.context(), argument_id);
        let has_collection_source_comment =
            trailing_collection_argument && f.context().has_comment(f.context().span(argument_id));
        let has_line_comment_annotations =
            has_line_comment_annotations || has_collection_source_comment;

        let force_expand_jsx = has_multiline_jsx_argument(f.context().tree, dynamic_arguments);
        let force_expand_single_commented_callback =
            argument_is_block_callback(f.context(), argument_id)
                && (argument_has_callback_blocking_comment_annotation(f.context(), argument_id)
                    || has_call_infix_annotations);
        let force_expand_single_prefix_line_commented_argument = f
            .context()
            .argument_has_prefix_line_comment_annotation(argument_id);

        (
            force_expand_jsx
                || has_line_comment_annotations
                || force_expand_single_commented_callback
                || force_expand_single_multiline_with_static_arguments
                || single_argument_force_expand
                || force_expand_single_collection_for_type_binary_callee
                || force_expand_single_prefix_line_commented_argument
                || has_call_infix_annotations,
            trailing_collection_argument,
        )
    } else if !has_call_infix_annotations && all_compact_simple_unannotated {
        (false, trailing_collection_argument)
    } else {
        let force_expand_jsx = has_multiline_jsx_argument(f.context().tree, dynamic_arguments);
        let force_expand_callback_with_collection_tail = trailing_collection_argument
            && dynamic_arguments[..dynamic_arguments.len().saturating_sub(1)]
                .iter()
                .copied()
                .any(|argument_id| argument_is_block_callback(f.context(), argument_id));

        let has_multiple_function_arguments =
            arrow_argument_count >= 2 || function_argument_count >= 2;
        if has_multiple_function_arguments {
            (true, trailing_collection_argument)
        } else {
            (
                force_expand_jsx
                    || has_complex_non_callback_argument
                    || has_line_comment_annotations
                    || force_expand_callback_with_collection_tail
                    || has_multiple_function_arguments
                    || has_call_infix_annotations,
                trailing_collection_argument,
            )
        }
    };
    let trailing_collection_comment_force_expand = dynamic_arguments.len() > 1
        && trailing_collection_argument
        && dynamic_arguments
            .last()
            .copied()
            .is_some_and(|last_argument_id| {
                let has_last_line_comment_annotation = has_line_comment_annotations
                    && argument_has_line_comment_annotation(f.context(), last_argument_id);
                let has_last_source_comment =
                    f.context().has_comment(f.context().span(last_argument_id));
                has_last_line_comment_annotation || has_last_source_comment
            });
    let force_expand =
        force_expand_regular || has_boundary_comments || trailing_collection_comment_force_expand;
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

/// Return whether an argument should emit its prefix annotations.
pub(crate) fn argument_should_emit_prefix_annotations(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let call_or_new_parent = ctx
        .parent(argument_id)
        .and_then(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return None;
            }

            let expression_id = LocalNodeId::<Expression>::new(parent_id);
            match ctx.tree.get(expression_id) {
                Expression::Call {
                    dynamic_arguments, ..
                }
                | Expression::New {
                    dynamic_arguments, ..
                } => Some((
                    expression_id,
                    dynamic_arguments
                        .first()
                        .is_some_and(|first| *first == argument_id),
                )),
                _ => None,
            }
        });
    let is_in_call_or_new = call_or_new_parent.is_some();
    let is_first_in_call_or_new = call_or_new_parent.is_some_and(|(_, is_first)| is_first);

    if argument_satisfies_static_seam_comment_annotation_id(ctx, argument_id).is_some() {
        return false;
    }

    if is_in_call_or_new
        && argument_has_only_separator_prefix_comment_cluster(ctx, argument_id)
        && !is_first_in_call_or_new
    {
        return false;
    }

    if !is_in_call_or_new {
        return true;
    }

    if argument_has_non_blank_prefix_annotation(ctx, argument_id) {
        return true;
    }

    if argument_has_blank_prefix_annotation_before_separator(ctx, argument_id) {
        return false;
    }

    if !is_first_in_call_or_new {
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
    let Some((path_expression_id, path_parent_type)) = ctx.parent(argument_id) else {
        return false;
    };
    if path_parent_type != NodeType::Expression {
        return false;
    }

    let path_expression_id = LocalNodeId::<Expression>::new(path_expression_id);
    let Expression::Path {
        path,
        static_arguments,
    } = ctx.tree.get(path_expression_id)
    else {
        return false;
    };
    if path.segments.len() != 1 {
        return false;
    }

    let Some(static_arguments) = static_arguments.as_ref() else {
        return false;
    };
    if static_arguments.len() <= 1
        || !static_arguments
            .first()
            .is_some_and(|first| *first == argument_id)
    {
        return false;
    }

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
