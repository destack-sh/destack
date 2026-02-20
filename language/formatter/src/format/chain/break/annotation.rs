use super::super::{
    Annotation, AnnotationPosition, ChainExpression, DestackFormatContext, Expression, LocalNodeId,
    NodeType, is_chain_expression,
};
use destack_ast::{Comment, CommentStyle, Doc, DocStyle, TokenType};

/// Return whether the next non-whitespace token after one annotation starts on the same line.
fn annotation_next_token_is_on_same_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let span = context.annotation_span(annotation_id);
    let tokens = context.tokens;
    let mut index = tokens.partition_point(|token| token.span.start < span.end);

    while let Some(token) = tokens.get(index).copied() {
        match token.token.ty {
            TokenType::Whitespace => {
                index += 1;
                continue;
            }
            TokenType::Newline => return false,
            _ => return true,
        }
    }

    false
}

/// Return whether one annotation should not force multiline chain breaking.
pub(super) fn chain_annotation_is_inline_non_breaking(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = context.annotation(annotation_id);
    let position = annotation.position();
    if !matches!(
        position,
        AnnotationPosition::BlockPrefix
            | AnnotationPosition::BlockPostfix
            | AnnotationPosition::LinePrefix
            | AnnotationPosition::LinePostfix
            | AnnotationPosition::LinePostfixBoundary
    ) {
        return false;
    }

    let is_star_style = match annotation {
        Annotation::Comment { node, .. } => {
            let comment = context.tree.get::<Comment>(node);
            comment.style == CommentStyle::Star
        }
        Annotation::Doc { node, .. } => {
            let doc = context.tree.get::<Doc>(node);
            doc.style == DocStyle::Star
        }
        Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
    };
    if !is_star_style {
        return false;
    }

    let annotation_span = context.annotation_span(annotation_id);
    if context.has_newline(annotation_span) {
        return false;
    }

    if position == AnnotationPosition::LinePostfixBoundary {
        return true;
    }

    annotation_next_token_is_on_same_line(context, annotation_id)
}

/// Return whether one annotation is an internal call argument infix marker.
pub(super) fn chain_annotation_is_internal_call_argument_infix(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    if context.annotation(annotation_id).position() != AnnotationPosition::BlockInfix {
        return false;
    }

    matches!(
        context.tree.get(node_id),
        Expression::Call { .. } | Expression::Instantiation { .. } | Expression::New { .. }
    )
}

/// Return whether one expression is wrapped by a statement expression parent.
fn chain_node_is_statement_wrapped(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::Statement(inner_id) if inner_id.id == node_id.id
            )
        })
}

/// Check whether a chain node has an annotation that should force breaking.
pub(crate) fn chain_node_has_breaking_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let is_chain_link = is_chain_expression(context.tree.get(node_id));
    let is_statement_wrapped_chain_link =
        is_chain_link && chain_node_is_statement_wrapped(context, node_id);
    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                if chain_annotation_is_inline_non_breaking(context, *annotation_id)
                    || chain_annotation_is_internal_call_argument_infix(
                        context,
                        node_id,
                        *annotation_id,
                    )
                {
                    return false;
                }

                let annotation = context.annotation(*annotation_id);
                let position = annotation.position();
                if !is_chain_link
                    && matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    )
                {
                    return false;
                }
                if is_statement_wrapped_chain_link
                    && matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    )
                {
                    return false;
                }

                matches!(
                    position,
                    AnnotationPosition::LinePrefix
                        | AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockPrefix
                        | AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                )
            })
        })
        .unwrap_or(false)
}

/// Check whether a chain node has annotations that prevent head grouping.
pub(crate) fn chain_node_has_non_inline_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let is_chain_link = is_chain_expression(context.tree.get(node_id));
    let is_statement_wrapped_chain_link =
        is_chain_link && chain_node_is_statement_wrapped(context, node_id);
    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                if chain_annotation_is_inline_non_breaking(context, *annotation_id)
                    || chain_annotation_is_internal_call_argument_infix(
                        context,
                        node_id,
                        *annotation_id,
                    )
                {
                    return false;
                }

                let annotation = context.annotation(*annotation_id);
                let position = annotation.position();
                if !is_chain_link
                    && matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    )
                {
                    return false;
                }
                if is_statement_wrapped_chain_link
                    && matches!(
                        position,
                        AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                    )
                {
                    return false;
                }

                match annotation {
                    Annotation::Blank { .. } => true,
                    Annotation::Doc { .. }
                    | Annotation::Comment { .. }
                    | Annotation::Decorator { .. } => matches!(
                        position,
                        AnnotationPosition::LinePrefix
                            | AnnotationPosition::LinePostfixBoundary
                            | AnnotationPosition::BlockPrefix
                            | AnnotationPosition::BlockInfix
                            | AnnotationPosition::BlockPostfix
                    ),
                }
            })
        })
        .unwrap_or(false)
}

/// Check whether a chain line starts with block prefix annotations.
pub(crate) fn chain_line_starts_with_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    line: &[ChainExpression],
) -> bool {
    let Some(first_op) = line.first() else {
        return false;
    };
    let node_id = match first_op {
        ChainExpression::Member { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Instantiation { node_id, .. }
        | ChainExpression::Maybe { node_id, .. }
        | ChainExpression::Must { node_id, .. } => *node_id,
    };

    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id).position(),
                    AnnotationPosition::BlockPrefix
                )
            })
        })
        .unwrap_or(false)
}
