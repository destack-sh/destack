use super::super::{
    Annotation, AnnotationPosition, Declaration, DestackFormatContext, Expression, LocalNodeId,
    TokenType, TypeBinaryOperator, expression_has_static_type_arguments,
    transparent_inner_expression,
};
use crate::analysis::scan::previous_non_whitespace_token_before_annotation;
use destack_ast::{Comment, CommentStyle};

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

/// Return whether one declarator value has an inline prefix comment on the `=` seam.
pub(super) fn declarator_value_has_inline_assignment_seam_prefix_comment(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(value_id, |annotation_ids| {
            annotation_ids.iter().any(|annotation_id| {
                let Annotation::Comment { node, position } = context.annotation(*annotation_id)
                else {
                    return false;
                };
                if !matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return false;
                }

                if !previous_non_whitespace_token_before_annotation(context, *annotation_id)
                    .is_some_and(|token| token.token.ty == TokenType::Assign)
                {
                    return false;
                }

                let comment = context.tree.get::<Comment>(node);
                let annotation_span = context.annotation_span(*annotation_id);
                let comment_is_inline = !context.has_newline(annotation_span)
                    && annotation_next_token_is_on_same_line(context, *annotation_id);

                match comment.style {
                    CommentStyle::Slash => false,
                    CommentStyle::Star => comment_is_inline,
                }
            })
        })
        .unwrap_or(false)
}

/// Return whether a declaration heritage clause contains static type arguments.
fn declaration_has_generic_heritage(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let heritage = match context.tree.get(declaration_id) {
        Declaration::Struct { heritage, .. }
        | Declaration::Class { heritage, .. }
        | Declaration::Enum { heritage, .. }
        | Declaration::Interface { heritage, .. }
        | Declaration::Extension { heritage, .. } => heritage,
        _ => return false,
    };

    heritage.extends_types.as_ref().is_some_and(|types| {
        types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(context, type_id))
    }) || heritage.implements_types.as_ref().is_some_and(|types| {
        types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(context, type_id))
    })
}

/// Return whether a value expression wraps a class declaration with generic heritage.
pub(super) fn value_has_generic_class_heritage(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            matches!(context.tree.get(*declaration_id), Declaration::Class { .. })
                && declaration_has_generic_heritage(context, *declaration_id)
        }
        Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. } => {
            value_has_generic_class_heritage(context, *left)
        }
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            value_has_generic_class_heritage(context, *expression)
        }
        _ => false,
    }
}

/// Return whether a value is a single-line closure-cast style type binary.
pub(super) fn value_is_inline_closure_cast_type_binary(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    if context.node_has_newline(value_id) {
        return false;
    }

    if !matches!(
        context.tree.get(value_id),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    ) {
        return false;
    }

    let mut current_id = value_id;
    loop {
        let current_span = context.span(current_id);
        let has_prefix = context
            .visit_annotations(current_id, |annotations| {
                !annotations.is_empty()
                    && annotations.iter().all(|annotation_id| {
                        let annotation_is_prefix = matches!(
                            context.annotation(*annotation_id),
                            Annotation::Comment {
                                position: AnnotationPosition::LinePrefix
                                    | AnnotationPosition::BlockPrefix,
                                ..
                            } | Annotation::Doc {
                                position: AnnotationPosition::LinePrefix
                                    | AnnotationPosition::BlockPrefix,
                                ..
                            }
                        );
                        if !annotation_is_prefix {
                            return false;
                        }

                        let annotation_span = context.annotation_span(*annotation_id);
                        annotation_span.start < current_span.start
                    })
            })
            .unwrap_or(false);
        if has_prefix {
            return true;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                Some(*expression)
            }
            Expression::TypeBinary { left, .. }
            | Expression::Binary { left, .. }
            | Expression::Call { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => Some(*left),
            _ => None,
        };
        let Some(next_id) = next_id else {
            return false;
        };
        current_id = next_id;
    }
}
