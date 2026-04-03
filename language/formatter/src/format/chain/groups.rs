use super::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead,
    chain_annotation_is_inline_non_breaking, chain_has_parent_intervening_break_or_comment,
    chain_head_id, chain_member_has_promotable_boundary_comment, chain_node_has_forcing_annotation,
    chain_node_has_non_inline_annotation, chain_operation_is_call_like, chain_operation_node_id,
    chain_overflows_in_type_binary_left, has_comment_between_expressions, is_numeric_index,
    is_simple_chain_static_arguments, member_has_intervening_comment, transparent_inner_expression,
};
use crate::DestackFormatContext;
use crate::format::call::{
    argument_is_inline_closure_cast_object, call_arguments_force_expand_for_chain,
};
use crate::format::operator::expression_has_static_type_arguments;
use destack_ast::{AnnotationPosition, Expression, LocalNodeId, NodeType, PostfixPosition};
use smallvec::SmallVec;

/// The grouping role of one chain operation.
#[derive(Clone, Copy, Eq, PartialEq)]
enum ChainOperationGroupingKind {
    NumericDirectIndex,
    MemberLike,
    CallLike,
    AttachedTail,
    Standalone,
}

/// Return the number of operations that stay in the head group.
pub(crate) fn chain_head_operation_count(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
    operations: &[ChainExpression],
) -> usize {
    let mut head_operation_count = 0usize;

    while head_operation_count < operations.len() {
        let operation = &operations[head_operation_count];
        if !chain_operation_stays_in_head_group(context, operation) {
            break;
        }

        head_operation_count += 1;
    }

    let base_has_leading_call_like = chain_base_has_leading_call_like(context, base);

    if !base_has_leading_call_like {
        while head_operation_count + 1 < operations.len() {
            let operation = &operations[head_operation_count];
            let next_operation = &operations[head_operation_count + 1];
            if chain_node_has_non_inline_annotation(context, chain_operation_node_id(operation))
                || !chain_operation_extends_member_head_group(context, operation)
                || !chain_operation_extends_member_head_group(context, next_operation)
            {
                break;
            }

            head_operation_count += 1;
        }
    }

    head_operation_count
}

/// Build the tail groups after the head.
pub(crate) fn build_tail_chain_lines(
    context: &DestackFormatContext<'_>,
    tail_operations: Vec<ChainExpression>,
) -> Vec<SmallVec<[ChainExpression; 2]>> {
    let mut lines = Vec::new();
    let mut current_line = SmallVec::<[ChainExpression; 2]>::new();
    let mut has_seen_call_like = false;

    for operation in tail_operations {
        match chain_operation_grouping_kind(context, &operation) {
            ChainOperationGroupingKind::NumericDirectIndex => {
                current_line.push(operation);
            }
            ChainOperationGroupingKind::MemberLike => {
                if has_seen_call_like {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = SmallVec::new();
                    }

                    current_line.push(operation);
                    has_seen_call_like = false;
                } else {
                    current_line.push(operation);
                }
            }
            ChainOperationGroupingKind::CallLike => {
                current_line.push(operation);
                has_seen_call_like = true;
            }
            ChainOperationGroupingKind::AttachedTail => {
                current_line.push(operation);
            }
            ChainOperationGroupingKind::Standalone => {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = SmallVec::new();
                }

                current_line.push(operation);
                has_seen_call_like = false;
            }
        }

        if chain_operation_has_trailing_annotations(context, current_line.last().unwrap()) {
            lines.push(current_line);
            current_line = SmallVec::new();
            has_seen_call_like = false;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Return the body-op index where one static-instantiation prefix wrap begins.
pub(crate) fn chain_instantiation_prefix_wrap_body_ops(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
) -> Option<usize> {
    let head_has_static_instantiation_prefix = match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => {
            expression_has_static_type_arguments(context, *expression_id)
        }
        ChainExpressionBaseHead::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
    };
    let static_instantiation_body_index = base
        .body
        .iter()
        .position(chain_operation_has_static_instantiation_arguments);
    let prefix_body_ops = if head_has_static_instantiation_prefix {
        Some(0usize)
    } else {
        static_instantiation_body_index.map(|index| index + 1)
    };
    if let Some(prefix_body_ops) = prefix_body_ops {
        let has_member_tail_in_base = base
            .body
            .iter()
            .skip(prefix_body_ops)
            .any(|operation| matches!(operation, ChainExpression::Member { .. }));
        let has_member_tail_in_lines = lines
            .first()
            .and_then(|line| line.first())
            .is_some_and(|operation| matches!(operation, ChainExpression::Member { .. }));
        (has_member_tail_in_base || has_member_tail_in_lines).then_some(prefix_body_ops)
    } else {
        None
    }
}

/// Return whether one base begins with call-like chaining.
fn chain_base_has_leading_call_like(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
) -> bool {
    match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => {
            let expression_id = transparent_inner_expression(context, *expression_id);
            let expression = context.tree.get(expression_id);
            chain_expression_is_call_like_base(context, expression)
                || base.body.first().is_some_and(chain_operation_is_call_like)
        }
        ChainExpressionBaseHead::Path { .. } => {
            base.body.first().is_some_and(chain_operation_is_call_like)
        }
    }
}

/// Return whether one expression behaves like a call-like chain base.
fn chain_expression_is_call_like_base(
    context: &DestackFormatContext<'_>,
    expression: &Expression,
) -> bool {
    matches!(
        expression,
        Expression::Call { .. } | Expression::Instantiation { .. }
    ) || matches!(
        expression,
        Expression::Parenthesized { expression }
            if matches!(
                context.tree.get(*expression),
                Expression::Call { .. } | Expression::Instantiation { .. }
            )
    )
}

/// Return whether one operation stays in the head group.
fn chain_operation_stays_in_head_group(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    matches!(
        chain_operation_grouping_kind(context, operation),
        ChainOperationGroupingKind::NumericDirectIndex
            | ChainOperationGroupingKind::CallLike
            | ChainOperationGroupingKind::AttachedTail
    )
}

/// Return whether one operation extends the member-like head group.
fn chain_operation_extends_member_head_group(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    matches!(
        chain_operation_grouping_kind(context, operation),
        ChainOperationGroupingKind::NumericDirectIndex | ChainOperationGroupingKind::MemberLike
    )
}

/// Return the grouping role of one chain operation.
fn chain_operation_grouping_kind(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> ChainOperationGroupingKind {
    if matches!(
        operation,
        ChainExpression::Index {
            position: PostfixPosition::Direct,
            index,
            ..
        } if is_numeric_index(context, index)
    ) {
        return ChainOperationGroupingKind::NumericDirectIndex;
    }

    if matches!(
        operation,
        ChainExpression::Member { .. } | ChainExpression::Index { .. }
    ) {
        return ChainOperationGroupingKind::MemberLike;
    }

    if chain_operation_is_call_like(operation) {
        return ChainOperationGroupingKind::CallLike;
    }

    if matches!(
        operation,
        ChainExpression::Maybe { .. } | ChainExpression::Must { .. }
    ) {
        return ChainOperationGroupingKind::AttachedTail;
    }

    ChainOperationGroupingKind::Standalone
}

/// Return whether an operation carries static instantiation arguments.
fn chain_operation_has_static_instantiation_arguments(operation: &ChainExpression) -> bool {
    match operation {
        ChainExpression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        ChainExpression::Member {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        _ => false,
    }
}

/// Return whether an operation has postfix annotations that force a split.
fn chain_operation_has_trailing_annotations(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    context
        .annotation_ids(chain_operation_node_id(operation))
        .iter()
        .any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id).position(),
                AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
                    | AnnotationPosition::BlockPostfix
            )
        })
}

/// Return whether one chain should break across groups.
pub(crate) fn chain_should_break(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
    base: &ChainExpressionBase,
) -> bool {
    chain_has_oxc_break_signal(context, chain, base)
        || chain_has_ir_specific_break_signal(context, chain, base)
}

/// Return whether one OXC-style member chain group should break.
fn chain_has_oxc_break_signal(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
    base: &ChainExpressionBase,
) -> bool {
    let has_chain_intervening_comment = chain_has_intervening_comment_or_break(context, chain);
    let has_optional_tail = chain_has_optional_tail(context, chain);
    let has_member_access = chain_has_member_access(context, chain);
    let has_chain_following_operation = chain.len() > 1;

    (has_chain_intervening_comment && has_member_access)
        || (has_chain_intervening_comment
            && (has_optional_tail || chain_has_direct_curried_call_pair(context, chain)))
        || (has_chain_following_operation && head_call_requires_expanded_arguments(context, chain))
        || tail_parent_call_requires_break(context, chain)
        || chain_base_requires_expanded_arguments(context, base)
}

/// Return whether one destack-specific chain signal forces breaking.
fn chain_has_ir_specific_break_signal(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
    base: &ChainExpressionBase,
) -> bool {
    let chain_tail = chain[chain.len() - 1];

    chain_has_forcing_annotations(context, chain)
        || chain_has_optional_call_boundary_trivia(context, chain)
        || chain_has_direct_call_with_inline_closure_cast_object_argument(context, chain)
        || chain_overflows_in_type_binary_left(context, chain_tail)
        || chain_has_doc_annotation(context, chain)
        || chain_body_has_breaking_member_annotation(context, base)
}

/// Return whether the chain has any doc annotations.
fn chain_has_doc_annotation(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.iter().copied().any(|expression_id| {
        context
            .annotation_ids(expression_id)
            .iter()
            .any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    crate::Annotation::Doc { .. }
                )
            })
    })
}

/// Return whether the chain body contains one breaking member annotation.
fn chain_body_has_breaking_member_annotation(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
) -> bool {
    base.body.iter().any(|operation| {
        let ChainExpression::Member { node_id, .. } = operation else {
            return false;
        };

        chain_node_has_forcing_annotation(context, *node_id, true)
    })
}

/// Return whether the chain has intervening comment or break trivia.
fn chain_has_intervening_comment_or_break(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain
        .windows(2)
        .any(|adjacent| has_comment_between_expressions(context, adjacent[0], adjacent[1]))
        || chain.iter().copied().any(|expression_id| {
            member_has_intervening_comment(context, expression_id)
                || chain_has_parent_intervening_break_or_comment(context, expression_id)
        })
}

/// Return whether the chain has optional-call boundary trivia.
fn chain_has_optional_call_boundary_trivia(
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

/// Return whether the chain has forcing annotations.
fn chain_has_forcing_annotations(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    let chain_root = chain[0];
    let chain_head = chain_head_id(context.tree, chain_root);

    chain.first().is_some_and(|expression_id| {
        chain_node_has_forcing_annotation(context, *expression_id, true)
    }) || chain
        .iter()
        .copied()
        .skip(1)
        .any(|expression_id| chain_node_has_forcing_annotation(context, expression_id, true))
        || context
            .annotation_ids(chain_head)
            .iter()
            .any(|annotation_id| {
                if chain_annotation_is_inline_non_breaking(context, *annotation_id)
                    || context.annotation(*annotation_id).position()
                        == AnnotationPosition::BlockInfix
                        && matches!(
                            context.tree.get(chain_head),
                            Expression::Call { .. }
                                | Expression::Instantiation { .. }
                                | Expression::New { .. }
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
}

/// Return whether the chain contains one optional tail.
fn chain_has_optional_tail(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Maybe { .. }
                | Expression::Call {
                    position: PostfixPosition::Indirect,
                    ..
                }
        )
    })
}

/// Return whether the chain contains one member access.
fn chain_has_member_access(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
    })
}

/// Return whether the chain has one direct curried call pair.
fn chain_has_direct_curried_call_pair(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.windows(2).any(|pair| {
        pair.iter().all(|expression_id| {
            matches!(
                context.tree.get(*expression_id),
                Expression::Call {
                    position: PostfixPosition::Direct,
                    ..
                }
            )
        })
    })
}

/// Return whether the chain has one direct call with one inline closure cast object argument.
fn chain_has_direct_call_with_inline_closure_cast_object_argument(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain.iter().copied().any(|expression_id| {
        let Expression::Call {
            position: PostfixPosition::Direct,
            dynamic_arguments,
            ..
        } = context.tree.get(expression_id)
        else {
            return false;
        };

        dynamic_arguments
            .iter()
            .copied()
            .any(|argument_id| argument_is_inline_closure_cast_object(context, argument_id))
    })
}

/// Return whether the head call requires expanded arguments.
fn head_call_requires_expanded_arguments(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    chain
        .iter()
        .copied()
        .enumerate()
        .find_map(|(index, expression_id)| {
            let Expression::Call {
                dynamic_arguments, ..
            } = context.tree.get(expression_id)
            else {
                return None;
            };

            // only calls that still have one chain tail should force the outer break
            if index + 1 >= chain.len() {
                return Some(false);
            }

            Some(call_arguments_force_expand_for_chain(
                context,
                expression_id,
                dynamic_arguments,
            ))
        })
        .unwrap_or(false)
}

/// Return whether the tail parent call requires the chain to break.
fn tail_parent_call_requires_break(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    let chain_tail = chain[chain.len() - 1];

    context
        .parent(chain_tail)
        .is_some_and(|(parent_id, parent_type)| {
            if parent_type != NodeType::Expression {
                return false;
            }

            let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
            let Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
                ..
            } = context.tree.get(parent_expression_id)
            else {
                return false;
            };
            if *left != chain_tail {
                return false;
            }

            !is_simple_chain_static_arguments(context, static_arguments)
                && !dynamic_arguments.is_empty()
        })
}

/// Return whether the normalized chain base already requires expanded arguments.
fn chain_base_requires_expanded_arguments(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
) -> bool {
    match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => {
            let Expression::Call {
                dynamic_arguments, ..
            } = context.tree.get(*expression_id)
            else {
                return false;
            };

            call_arguments_force_expand_for_chain(context, *expression_id, dynamic_arguments)
        }
        ChainExpressionBaseHead::Path { .. } => false,
    }
}
