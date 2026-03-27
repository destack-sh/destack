use super::{
    ChainExpression, assignment_like_parent, chain_has_parent_intervening_break_or_comment,
    chain_member_has_promotable_boundary_comment, has_comment_between_expressions,
    is_nested_lambda_expression, member_has_intervening_comment,
};
use crate::format::analysis::is_call_like_argument;
use crate::format::operator::is_chain_expression;
use crate::{Annotation, DestackFormatContext};
use destack_ast::{
    AnnotationPosition, Comment, CommentStyle, Doc, DocStyle, Expression, IfKind, LocalNodeId,
    NodeTree, NodeType, PostfixPosition, TokenType, TypeBinaryOperator,
};

/// Treat blanks as unconditional blockers during chain annotation scans.
const CHAIN_SCAN_FORCE_BLANKS: bool = true;

/// Treat blanks as blockers only at breaking positions during chain annotation scans.
const CHAIN_SCAN_POSITIONAL_BLANKS: bool = false;

/// Return whether chain annotations should force breaking.
pub(crate) fn chain_has_breaking_annotations(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
    chain_head: LocalNodeId<Expression>,
) -> bool {
    let has_chain_root_annotations = chain
        .first()
        .is_some_and(|expression_id| chain_node_has_breaking_annotation(context, *expression_id));

    // root prefix annotations are statement-level concerns:
    // root non-prefix annotation handling stays in `chain_head_has_non_prefix_breaking_annotation`
    let has_chain_node_annotations = chain
        .iter()
        .copied()
        .skip(1)
        .any(|expression_id| chain_node_has_breaking_annotation(context, expression_id));
    let chain_head_has_non_prefix_breaking_annotation =
        chain_head_has_non_prefix_breaking_annotation(context, chain_head);

    has_chain_root_annotations
        || has_chain_node_annotations
        || chain_head_has_non_prefix_breaking_annotation
}

/// Return whether a chain head has non-prefix breaking annotation positions.
fn chain_head_has_non_prefix_breaking_annotation(
    context: &DestackFormatContext<'_>,
    chain_head: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(chain_head, |annotations| {
            annotations.iter().any(|annotation_id| {
                if chain_annotation_is_inline_non_breaking(context, *annotation_id)
                    || chain_annotation_is_internal_call_argument_infix(
                        context,
                        chain_head,
                        *annotation_id,
                    )
                {
                    return false;
                }

                let position = context.annotation(*annotation_id).position();
                if matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return false;
                }

                matches!(
                    position,
                    AnnotationPosition::LinePostfixBoundary
                        | AnnotationPosition::BlockInfix
                        | AnnotationPosition::BlockPostfix
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether one node annotation should count for chain formatting.
fn chain_node_has_forcing_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    is_breaking_scan: bool,
    blanks_force_regardless_of_position: bool,
) -> bool {
    let is_chain_link = is_chain_expression(context.tree.get(node_id));
    let is_statement_wrapped_chain_link =
        is_chain_link && chain_node_is_statement_wrapped(context, node_id);
    let has_optional_tail_boundary_comment =
        is_breaking_scan && chain_member_boundary_comment_precedes_optional_tail(context, node_id);

    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                let position = annotation.position();
                let is_optional_tail_boundary_comment = has_optional_tail_boundary_comment
                    && matches!(
                        annotation,
                        Annotation::Comment {
                            position: AnnotationPosition::LinePostfixBoundary,
                            ..
                        }
                    );

                if !is_optional_tail_boundary_comment
                    && (chain_annotation_is_inline_non_breaking(context, *annotation_id)
                        || chain_annotation_is_internal_call_argument_infix(
                            context,
                            node_id,
                            *annotation_id,
                        ))
                {
                    return false;
                }

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
                    Annotation::Blank { .. } => {
                        blanks_force_regardless_of_position
                            || matches!(
                                position,
                                AnnotationPosition::LinePrefix
                                    | AnnotationPosition::LinePostfixBoundary
                                    | AnnotationPosition::BlockPrefix
                                    | AnnotationPosition::BlockInfix
                                    | AnnotationPosition::BlockPostfix
                            )
                    }
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

/// Return whether one optional call seam has boundary trivia before its operator.
pub(crate) fn chain_has_optional_call_boundary_trivia(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.windows(2).any(|pair| {
        let left = pair[0];
        let right = pair[1];

        let right_is_optional_call = matches!(
            context.tree.get(right),
            Expression::Call {
                position: PostfixPosition::Indirect,
                ..
            }
        );
        if !right_is_optional_call {
            return false;
        }

        chain_has_parent_intervening_break_or_comment(context, left)
            || has_comment_between_expressions(context, left, right)
            || matches!(
                context.tree.get(left),
                Expression::Member { .. } | Expression::PrivateMember { .. }
            ) && chain_member_has_promotable_boundary_comment(context, left)
    })
}

/// Return whether one annotation should not force multiline chain breaking.
pub(crate) fn chain_annotation_is_inline_non_breaking(
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
        return context.annotation_next_token_is_on_same_line(annotation_id);
    }

    if position == AnnotationPosition::LinePostfix
        && context.annotation_next_non_whitespace_token_type(annotation_id)
            == Some(TokenType::Maybe)
    {
        return true;
    }

    context.annotation_next_token_is_on_same_line(annotation_id)
}

/// Return whether one annotation is an internal call argument infix marker.
pub(crate) fn chain_annotation_is_internal_call_argument_infix(
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

/// Return whether a member boundary comment sits before an optional tail operator.
fn chain_member_boundary_comment_precedes_optional_tail(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if !matches!(
        context.tree.get(node_id),
        Expression::Member { .. } | Expression::PrivateMember { .. }
    ) {
        return false;
    }

    let has_boundary_comment = context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false);
    if !has_boundary_comment {
        return false;
    }

    context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::Maybe { left, .. } if *left == node_id
            ) || matches!(
                context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                Expression::Call {
                    left,
                    position: PostfixPosition::Indirect,
                    ..
                } if *left == node_id
            )
        })
}

/// Check whether a chain node has an annotation that should force breaking.
pub(crate) fn chain_node_has_breaking_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    chain_node_has_forcing_annotation(context, node_id, true, CHAIN_SCAN_POSITIONAL_BLANKS)
}

/// Check whether a chain node has annotations that prevent head grouping.
pub(crate) fn chain_node_has_non_inline_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    chain_node_has_forcing_annotation(context, node_id, false, CHAIN_SCAN_FORCE_BLANKS)
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

/// Return whether a chain contains comments between adjacent chain operations.
pub(crate) fn chain_has_intervening_comment(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    // check adjacent chain nodes directly for source comments
    for adjacent in chain.windows(2) {
        let left_id = adjacent[0];
        let right_id = adjacent[1];
        if has_comment_between_expressions(context, left_id, right_id) {
            return true;
        }
    }

    chain.iter().copied().any(|expression_id| {
        member_has_intervening_comment(context, expression_id)
            || chain_has_parent_intervening_break_or_comment(context, expression_id)
    })
}

/// Return whether one expression is await-like.
fn expression_is_await_like(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
    matches!(
        tree.get(expression_id),
        Expression::Await { .. } | Expression::AwaitMaybe { .. }
    )
}

/// Return whether a call has a member receiver wrapped in parenthesized await-like expression.
pub(crate) fn call_has_parenthesized_await_member_receiver(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = context.tree.get(node_id) else {
        return false;
    };

    let member_receiver_id = match context.tree.get(*left) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => *left,
        _ => return false,
    };

    let Expression::Parenthesized { expression } = context.tree.get(member_receiver_id) else {
        return false;
    };
    expression_is_await_like(context.tree, *expression)
}

/// Return whether one receiver sits under await-like wrapping.
pub(crate) fn receiver_is_await_wrapped(
    context: &DestackFormatContext<'_>,
    receiver_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(receiver_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    if matches!(
        context.tree.get(parent_expression_id),
        Expression::Await { expression } | Expression::AwaitMaybe { expression }
            if *expression == receiver_id
    ) {
        return true;
    }

    let Expression::Parenthesized { expression } = context.tree.get(parent_expression_id) else {
        return false;
    };
    if *expression != receiver_id {
        return false;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_expression_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    matches!(
        context.tree.get(grandparent_expression_id),
        Expression::Await { expression } | Expression::AwaitMaybe { expression }
            if *expression == parent_expression_id
    )
}

/// Return whether one chain tail overflows through cast or satisfies parent context.
pub(crate) fn chain_overflows_in_type_binary_left(
    context: &DestackFormatContext<'_>,
    chain_tail: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(chain_tail) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::TypeBinary { left, operator, .. } = context.tree.get(parent_expression_id)
    else {
        return false;
    };
    if *left != chain_tail {
        return false;
    }
    if !matches!(
        operator,
        TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
    ) {
        return false;
    }

    if context.has_non_blank_annotation(parent_expression_id) {
        return true;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_expression_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    let Expression::Parenthesized { expression } = context.tree.get(grandparent_expression_id)
    else {
        return false;
    };
    if *expression != parent_expression_id {
        return false;
    }

    let Some((great_grandparent_id, great_grandparent_type)) =
        context.parent(grandparent_expression_id)
    else {
        return false;
    };
    if great_grandparent_type != NodeType::Expression {
        return false;
    }

    let great_grandparent_expression_id = LocalNodeId::<Expression>::new(great_grandparent_id);
    let Expression::New { left, .. } = context.tree.get(great_grandparent_expression_id) else {
        return false;
    };
    if *left != grandparent_expression_id {
        return false;
    }

    context.has_non_blank_annotation(great_grandparent_expression_id)
}

/// Return whether a path root should be split into synthetic chain segments.
pub(crate) fn should_split_chain_root_path_segments(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Path { path, .. } = context.tree.get(root_id) else {
        return false;
    };
    if path.segments.len() <= 1 {
        return false;
    }

    // preserve compact callee-style heads inside argument positions
    if is_call_like_argument(context, root_id) {
        return false;
    }

    let has_optional_or_must_tail = path_chain_has_optional_or_must_tail(context, root_id);
    let has_root_line_postfix_boundary_comment = context
        .visit_annotations(root_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false);
    let has_boundary_comments = has_root_line_postfix_boundary_comment;

    // keep factory style roots merged by default, unless boundary comments need a seam
    let first_segment = context.strings.get(path.segments[0]);
    if is_factory_like_path_head(first_segment) && !has_boundary_comments {
        return false;
    }
    if first_segment == "this" && !has_optional_or_must_tail && !has_boundary_comments {
        return false;
    }

    // conditional branches keep compact path heads
    if expression_is_in_conditional_branch(context, root_id) {
        return false;
    }

    true
}

/// Return whether a path chain has optional or must tail operators.
pub(crate) fn path_chain_has_optional_or_must_tail(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> bool {
    let mut current = root_id;

    while let Some((parent_id, parent_type)) = context.parent(current) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        let parent = context.tree.get(parent_id);
        let parent_uses_left = match parent {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => *left == current,
            _ => false,
        };
        if !parent_uses_left {
            break;
        }

        if matches!(
            parent,
            Expression::Maybe { .. }
                | Expression::Must { .. }
                | Expression::Call {
                    position: PostfixPosition::Indirect,
                    ..
                }
                | Expression::Index {
                    position: PostfixPosition::Indirect,
                    ..
                }
        ) {
            return true;
        }

        current = parent_id;
    }

    false
}

/// Return whether a path head looks like a factory identifier.
pub(crate) fn is_factory_like_path_head(name: &str) -> bool {
    let mut bytes = name.bytes();
    match bytes.next() {
        Some(b'_' | b'$') => bytes.all(|byte| matches!(byte, b'_' | b'$')),
        Some(byte) => byte.is_ascii_uppercase(),
        None => false,
    }
}

/// Return whether an expression is inside a ternary branch.
pub(crate) fn expression_is_in_conditional_branch(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context.any_ancestor(expression_id, |ancestor_id, node_type| {
        node_type == NodeType::Expression
            && matches!(
                context
                    .tree
                    .get(LocalNodeId::<Expression>::new(ancestor_id)),
                Expression::If {
                    kind: IfKind::Ternary,
                    ..
                }
            )
    })
}

/// Check whether an assignment chain ends in a nested lambda expression.
pub(crate) fn is_assignment_chain_tail_lambda(
    context: &DestackFormatContext<'_>,
    assignment_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    // only assignment-like rhs positions participate in assignment chains
    if assignment_like_parent(context, assignment_id).is_none() {
        return false;
    }

    // intermediate assignments are not chain tails
    if matches!(context.tree.get(right_id), Expression::Assign { .. }) {
        return false;
    }

    is_nested_lambda_expression(context, right_id)
}
