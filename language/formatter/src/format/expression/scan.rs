use super::{
    Annotation, AnnotationPosition, Argument, DestackFormatContext, Expression, IfCondition,
    IfKind, LocalNodeId, NodeType, Span, TokenType, parenthesized_has_leading_inner_trivia,
    transparent_inner_expression,
};
use crate::analysis::scan::{first_non_trivia_token_in_span, last_non_trivia_token_in_span};
use crate::directive::comment_node_is_ignore_directive;

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

/// Return whether an expression tree contains multiline static type arguments.
pub(crate) fn expression_has_multiline_static_type_argument(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    let static_arguments_have_newline = |arguments: &[LocalNodeId<Argument>]| {
        arguments
            .iter()
            .copied()
            .any(|argument_id| context.node_has_newline(argument_id))
    };

    match context.tree.get(expression_id) {
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| static_arguments_have_newline(arguments)),
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
                .is_some_and(|arguments| static_arguments_have_newline(arguments))
                || expression_has_multiline_static_type_argument(context, *left)
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
                .is_some_and(|arguments| static_arguments_have_newline(arguments))
                || expression_has_multiline_static_type_argument(context, *left)
        }
        Expression::Instantiation {
            left,
            static_arguments,
        } => {
            static_arguments_have_newline(static_arguments)
                || expression_has_multiline_static_type_argument(context, *left)
        }
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            expression_has_multiline_static_type_argument(context, *expression)
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
        .visit_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                if !matches!(annotation.position(), AnnotationPosition::BlockPrefix)
                    || !matches!(annotation, Annotation::Comment { .. })
                {
                    return false;
                }

                let annotation_span = context.annotation_span(*annotation_id);
                context.has_newline(annotation_span)
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
    let span_len = context.span_char_len(span);
    (span_len, span_len)
}

/// Return whether expression source is wrapped in a top-level parenthesis pair.
pub(super) fn expression_source_has_outer_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let span = context.span(node_id);
    let Some(first_token) = first_non_trivia_token_in_span(context, span) else {
        return false;
    };
    let Some(last_token) = last_non_trivia_token_in_span(context, span) else {
        return false;
    };

    first_token.token.ty == TokenType::OpenParenthesis
        && last_token.token.ty == TokenType::CloseParenthesis
}

/// Return whether an expression has a prefix comment annotation.
pub(super) fn expression_has_prefix_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    } | Annotation::Doc {
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
        .visit_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let Annotation::Comment { position, node } = context.annotation(*annotation_id)
                else {
                    return false;
                };

                if !matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return false;
                }

                if comment_node_is_ignore_directive(context, node) {
                    return true;
                }
                false
            })
        })
        .unwrap_or(false)
}

/// Return whether an expression has a leading prefix comment in its left spine.
pub(crate) fn expression_has_leading_prefix_comment(
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
    let is_parent_yield_value = context
        .parent(node_id)
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

    let has_doc_like_prefix_annotation = context
        .visit_annotations(inner_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Doc {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false);

    parenthesized_has_leading_inner_trivia(context, node_id, inner_id)
        && has_doc_like_prefix_annotation
        && !expression_has_prefix_ignore_directive_comment_annotation(context, inner_id)
}

/// Decide whether a sequence expression needs parentheses in its parent context.
pub(super) fn sequence_expression_needs_parens(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
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
