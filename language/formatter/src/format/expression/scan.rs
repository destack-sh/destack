use super::*;
use crate::directive::is_ignore_directive_comment;
use crate::scan::previous_non_whitespace_before_span as previous_non_whitespace_before_source_span;
use destack_ast::Comment;

/// Return a compact lower bound for one-line width from source text.
#[inline]
pub(crate) fn source_min_inline_char_len(source: &str) -> usize {
    if source.is_ascii() {
        source
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .count()
    } else {
        source.chars().filter(|ch| !ch.is_whitespace()).count()
    }
}

/// Return whether an expression tree contains static type arguments.
pub(crate) fn expression_has_static_type_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || expression_has_static_type_arguments(context, *left)
        }
        Expression::Call {
            left,
            static_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            ..
        } => {
            static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || expression_has_static_type_arguments(context, *left)
        }
        Expression::Instantiation {
            left,
            static_arguments,
        } => !static_arguments.is_empty() || expression_has_static_type_arguments(context, *left),
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            expression_has_static_type_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether an expression has a non-doc multiline block prefix comment annotation.
pub(crate) fn expression_has_non_doc_multiline_block_prefix_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .with_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.get_annotation(*annotation_id);
                if !matches!(annotation.position(), AnnotationPosition::BlockPrefix)
                    || !matches!(annotation, Annotation::Comment { .. })
                {
                    return false;
                }

                let annotation_span = context.get_annotation_span(*annotation_id);
                let annotation_source = context.get_span_str(annotation_span);
                let trimmed = annotation_source.trim_start();

                annotation_source.contains('\n') && !trimmed.starts_with("/**")
            })
        })
        .unwrap_or(false)
}

/// Return compact and source-width inline character bounds for one span.
#[inline]
pub(crate) fn span_inline_char_bounds(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> (usize, usize) {
    let source = context.get_span_str(span);
    let min_len = source_min_inline_char_len(source);
    let max_len = context.span_char_len(span);
    (min_len, max_len)
}

/// Return the previous non-whitespace character before a span.
pub(super) fn previous_non_whitespace_before_span(
    context: &DestackFormatContext<'_>,
    span: Span,
) -> Option<char> {
    previous_non_whitespace_before_source_span(context, span)
}

/// Return whether expression source is wrapped in a top-level parenthesis pair.
pub(super) fn expression_source_has_outer_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let source = context.get_span_str(context.get_span(node_id));
    let source = source.trim();
    source.starts_with('(') && source.ends_with(')')
}

/// Return whether an expression has a prefix comment annotation.
pub(super) fn expression_has_prefix_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .with_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.get_annotation(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether an expression has a prefix ignore-directive comment annotation.
pub(super) fn expression_has_prefix_ignore_directive_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .with_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let Annotation::Comment { position, node } = context.get_annotation(*annotation_id)
                else {
                    return false;
                };

                if !matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return false;
                }

                let comment = context.tree.get::<Comment>(node);
                let comment_source = context.strings.get(comment.string);
                if is_ignore_directive_comment(comment_source) {
                    return true;
                }

                let comment_span = context.get_span::<Comment>(node);
                let raw_comment = context.get_span_str(comment_span);
                is_ignore_directive_comment(raw_comment)
            })
        })
        .unwrap_or(false)
}

/// Return whether an expression has a leading prefix comment in its left spine.
pub(super) fn expression_has_leading_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current = expression_id;

    loop {
        if expression_has_prefix_comment_annotation(context, current) {
            return true;
        }

        let next = match context.tree.get(current) {
            Expression::Parenthesized { expression } => Some(*expression),
            Expression::Binary { left, .. } | Expression::TypeBinary { left, .. } => Some(*left),
            Expression::If {
                kind: IfKind::Ternary,
                condition:
                    IfCondition::Expression {
                        condition: expression_id,
                    },
                ..
            } => Some(*expression_id),
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => Some(*left),
            Expression::TaggedTemplateExpression { tag, .. } => Some(*tag),
            _ => None,
        };

        let Some(next) = next else {
            break;
        };
        current = next;
    }

    false
}

/// Return whether a yield value has leading prefix comments on its left side.
pub(super) fn yield_value_has_leading_prefix_comment(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    expression_has_leading_prefix_comment(context, value_id)
}

/// Return whether parenthesized cast comments should be hoisted before `(`.
pub(super) fn should_hoist_parenthesized_inner_cast_prefix_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let is_parent_yield_value =
        context
            .get_parent(node_id)
            .is_some_and(|(parent_id, parent_type)| {
                parent_type == NodeType::Expression
                    && matches!(
                        context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                        Expression::Yield { value: Some(value_id), .. } if *value_id == node_id
                    )
            });
    if is_parent_yield_value {
        return false;
    }

    matches!(
        context.tree.get(inner_id),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    ) && parenthesized_has_leading_inner_trivia(context, node_id, inner_id)
        && expression_has_prefix_comment_annotation(context, inner_id)
        && !expression_has_prefix_ignore_directive_comment_annotation(context, inner_id)
}

/// Decide whether a sequence expression needs parentheses in its parent context.
pub(super) fn sequence_expression_needs_parens(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Expression {
        return true;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Statement(inner_id) => *inner_id != node_id,
        Expression::Return { value } => value.is_some_and(|value_id| value_id != node_id),
        Expression::Throw { value } => *value != node_id,
        Expression::Parenthesized { expression } => *expression != node_id,
        Expression::For {
            initialization,
            increment,
            ..
        } => {
            !initialization.is_some_and(|value_id| value_id == node_id)
                && !increment.is_some_and(|value_id| value_id == node_id)
        }
        // preserve explicit nested grouping: `(1, (2, 3), 4)`
        Expression::SequenceExpression { .. } => {
            expression_source_has_outer_parentheses(context, node_id)
        }
        _ => true,
    }
}
