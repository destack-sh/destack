use super::*;
use crate::scan::previous_non_whitespace_before_span as previous_non_whitespace_before_source_span;

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
                    context.tree.get::<Annotation>(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
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
