use super::super::{
    Annotation, AnnotationPosition, AssignOperator, DestackFormatContext, Expression, LocalNodeId,
    NodeType, Span, TokenType,
};
use crate::analysis::scan::{
    previous_non_whitespace_token_before_annotation, previous_non_whitespace_token_before_span,
};
use destack_ast::{Comment, CommentStyle};

/// Return whether one token is an assignment operator token.
#[inline]
fn is_assignment_operator_token(token_type: TokenType) -> bool {
    AssignOperator::from_token(token_type).is_some()
}

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

/// Return whether one expression has an inline prefix comment on an assignment seam.
pub(super) fn expression_has_assignment_seam_inline_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotation_ids| {
            annotation_ids.iter().any(|annotation_id| {
                let Annotation::Comment { node, position } = context.annotation(*annotation_id)
                else {
                    return false;
                };
                if position != AnnotationPosition::LinePrefix {
                    return false;
                }

                let comment = context.tree.get::<Comment>(node);
                if !previous_non_whitespace_token_before_annotation(context, *annotation_id)
                    .is_some_and(|token| is_assignment_operator_token(token.token.ty))
                {
                    return false;
                }

                match comment.style {
                    CommentStyle::Slash => true,
                    CommentStyle::Star => {
                        let annotation_span = context.annotation_span(*annotation_id);
                        !context.has_newline(annotation_span)
                            && annotation_next_token_is_on_same_line(context, *annotation_id)
                    }
                }
            })
        })
        .unwrap_or(false)
}

/// Return whether one assignment seam has a slash line comment between left and right.
pub(super) fn assignment_seam_has_line_comment_between(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<Expression>,
    right: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left);
    let right_span = context.span(right);
    if left_span.file != right_span.file || left_span.end >= right_span.start {
        return false;
    }

    let between_span = Span::new(left_span.file, left_span.end, right_span.start);
    context
        .comment_tokens()
        .iter()
        .copied()
        .any(|comment_token| {
            if !between_span.intersects(comment_token.span) {
                return false;
            }

            if !matches!(
                comment_token.token.ty,
                TokenType::LineComment | TokenType::DocLineComment
            ) {
                return false;
            }

            previous_non_whitespace_token_before_span(context, comment_token.span)
                .is_some_and(|token| is_assignment_operator_token(token.token.ty))
        })
}

/// Walk left-linked assignment parents and return the outermost chain node.
pub(super) fn left_assignment_chain_root(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            break;
        };
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
        let Expression::Assign { left, .. } = parent_expression else {
            break;
        };
        if left.id != current_id.id {
            break;
        }

        current_id = LocalNodeId::<Expression>::new(parent_id);
    }

    current_id
}

/// Walk right-linked assignment parents and return the outermost chain node.
pub(super) fn right_assignment_chain_root(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current_id = node_id;

    loop {
        let Some((parent_id, parent_type)) = context.parent(current_id) else {
            break;
        };
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
        let Expression::Assign { right, .. } = parent_expression else {
            break;
        };
        if right.id != current_id.id {
            break;
        }

        current_id = LocalNodeId::<Expression>::new(parent_id);
    }

    current_id
}

/// Return the direct assignment parent when current is the rhs.
pub(super) fn right_assignment_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let (parent_id, parent_type) = context.parent(node_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_id);
    let Expression::Assign { right, .. } = parent_expression else {
        return None;
    };
    if right.id != node_id.id {
        return None;
    }

    Some(parent_id)
}
