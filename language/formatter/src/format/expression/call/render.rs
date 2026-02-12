use super::super::*;
use super::profile::*;
use crate::timing::tags;
use destack_fir::write;

/// Collect deferred callee boundary comments for empty call argument lists.
pub(crate) fn collect_deferred_empty_call_boundary_comments(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> (Option<String>, Option<String>, Option<String>) {
    let (left, dynamic_arguments) = match context.tree.get(call_node_id) {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => (*left, dynamic_arguments),
        _ => return (None, None, None),
    };

    if !dynamic_arguments.is_empty() {
        return (None, None, None);
    }

    let mut inline_argument_comment: Option<String> = None;
    let mut line_argument_comment: Option<String> = None;
    let mut trailing_optional_comment: Option<String> = None;
    for expression_id in callee_expression_chain_ids(context, left) {
        context.with_annotations(expression_id, |annotations| {
            for annotation_id in annotations {
                let Annotation::Comment {
                    node: comment_id,
                    position: annotation_position,
                } = context.tree.get::<Annotation>(*annotation_id)
                else {
                    continue;
                };
                let comment = context.tree.get::<destack_ast::Comment>(*comment_id);
                let annotation_span = context.get_span::<Annotation>(*annotation_id);
                let annotation_source = context.get_span_str(annotation_span).trim().to_string();
                let previous_character =
                    previous_non_whitespace_before_annotation(context, *annotation_id);
                let next_character = next_non_whitespace_after_annotation(context, *annotation_id);

                if comment.style == destack_ast::CommentStyle::Star
                    && *annotation_position == AnnotationPosition::BlockPostfix
                    && previous_character == Some('(')
                    && next_character == Some(')')
                {
                    inline_argument_comment = Some(annotation_source);
                } else if comment.style == destack_ast::CommentStyle::Slash
                    && matches!(
                        *annotation_position,
                        AnnotationPosition::LinePostfix
                            | AnnotationPosition::LinePostfixBoundary
                            | AnnotationPosition::BlockPostfix
                    )
                    && previous_character == Some('(')
                    && next_character == Some(')')
                {
                    line_argument_comment = Some(annotation_source);
                } else if comment.style == destack_ast::CommentStyle::Slash
                    && matches!(
                        *annotation_position,
                        AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
                    )
                    && next_character == Some('?')
                {
                    trailing_optional_comment = Some(annotation_source);
                }
            }
        });
    }

    (
        inline_argument_comment,
        line_argument_comment,
        trailing_optional_comment,
    )
}

/// Collect callee chain expression ids where boundary comments may be attached.
pub(crate) fn callee_expression_chain_ids(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Expression>> {
    let mut result = vec![left_id];
    let mut current_id = left_id;

    loop {
        let next_id = match context.tree.get(current_id) {
            Expression::Maybe { left, .. } | Expression::Must { left, .. } => Some(*left),
            Expression::Parenthesized { expression } => Some(*expression),
            _ => None,
        };

        let Some(next_id) = next_id else {
            break;
        };

        result.push(next_id);
        current_id = next_id;
    }

    result
}

/// Return the enclosing empty call expression for a callee expression chain.
pub(crate) fn enclosing_empty_call_id_for_callee_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    if matches!(
        context.tree.get(expression_id),
        Expression::Call {
            dynamic_arguments, ..
        } | Expression::New {
            dynamic_arguments, ..
        } if dynamic_arguments.is_empty()
    ) {
        return Some(expression_id);
    }

    let mut current_id = expression_id;
    loop {
        let (parent_id, parent_type) = context.get_parent_by_id(current_id.id)?;
        if parent_type != NodeType::Expression {
            return None;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        match context.tree.get(parent_id) {
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
                if *left != current_id || !dynamic_arguments.is_empty() {
                    return None;
                }

                return Some(parent_id);
            }
            Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
                if *left != current_id {
                    return None;
                }

                current_id = parent_id;
            }
            Expression::Parenthesized { expression } => {
                if *expression != current_id {
                    return None;
                }

                current_id = parent_id;
            }
            _ => return None,
        }
    }
}

/// Return whether an annotation is deferred to call rendering for empty call boundaries.
pub(crate) fn is_deferred_empty_call_boundary_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    annotation_id: LocalNodeId<Annotation>,
    annotation_position: AnnotationPosition,
) -> bool {
    if enclosing_empty_call_id_for_callee_expression(context, expression_id).is_none() {
        return false;
    }

    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };
    let comment = context.tree.get::<destack_ast::Comment>(*node);
    let previous_character = previous_non_whitespace_before_annotation(context, annotation_id);
    let next_character = next_non_whitespace_after_annotation(context, annotation_id);

    if comment.style == destack_ast::CommentStyle::Star
        && matches!(
            annotation_position,
            AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockPostfix
        )
        && ((previous_character == Some('(') && next_character == Some(')'))
            || next_character == Some('?'))
    {
        return true;
    }

    comment.style == destack_ast::CommentStyle::Slash
        && matches!(
            annotation_position,
            AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockPostfix
        )
        && (previous_character == Some('(') && next_character == Some(')')
            || next_character == Some('?'))
}

/// Return whether an expression participates in a deferred empty call boundary comment chain.
pub(crate) fn expression_is_in_deferred_empty_call_boundary_chain(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(call_id) = enclosing_empty_call_id_for_callee_expression(context, expression_id)
    else {
        return false;
    };
    let (inline_argument_comment, line_argument_comment, trailing_optional_comment) =
        collect_deferred_empty_call_boundary_comments(context, call_id);

    inline_argument_comment.is_some()
        || line_argument_comment.is_some()
        || trailing_optional_comment.is_some()
}

/// Format call dynamic arguments while honoring deferred callee boundary comments.
pub(crate) fn format_call_dynamic_arguments_with_deferred_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if !dynamic_arguments.is_empty() {
        return format_call_arguments(f, call_node_id, dynamic_arguments);
    }

    let (inline_argument_comment, line_argument_comment, trailing_optional_comment) =
        collect_deferred_empty_call_boundary_comments(f.context(), call_node_id);

    let _timing = f
        .context()
        .timing_scope(tags::FORMAT_EXPRESSION_CALL_EMPTY_ARGUMENTS);
    if let Some(comment) = line_argument_comment {
        let content = format_with(|f| write!(f, [hard_line_break(), text(comment.as_str())]));
        write!(
            f,
            [token("("), indent(&content), hard_line_break(), token(")")]
        )?;
    } else if let Some(comment) = inline_argument_comment {
        write!(f, [token("("), text(comment.as_str()), token(")")])?;
    } else {
        write!(f, [token("("), token(")")])?;
    }

    if let Some(comment) = trailing_optional_comment {
        let content = format_with(|f| write!(f, [space(), text(comment.as_str())]));
        write!(f, [line_postfix(&content, 0)])?;
    }

    Ok(())
}

/// Format a call expression.
#[inline]
pub(crate) fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let _timing = f.context().timing_scope(tags::FORMAT_EXPRESSION_CALL);

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

        format_call_dynamic_arguments_with_deferred_comments(f, node_id, dynamic_arguments)?;
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
