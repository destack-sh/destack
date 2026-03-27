use super::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead, chain_expression_from_node,
    chain_has_breaking_annotations, chain_has_intervening_comment,
    chain_has_optional_call_boundary_trivia, chain_head_id, chain_node_has_breaking_annotation,
    chain_node_has_non_inline_annotation, chain_nodes, chain_operation_node_id,
    chain_overflows_in_type_binary_left, is_numeric_index, is_simple_chain_static_arguments,
    path_postfix_annotations_emit_on_tail, should_split_chain_root_path_segments,
};
use crate::format::analysis::argument_is_inline_closure_cast_object;
use crate::format::call::call_arguments_force_expand_for_chain;
use crate::format::chain::transparent_inner_expression;
use crate::format::expression::should_unwrap_parenthesized_member_object;
use crate::{Annotation, DestackFormatContext};
use destack_ast::{Expression, LocalNodeId, NodeTree, NodeType, PostfixPosition};
use destack_fir::format::{FormatError, FormatResult};
use smallvec::{SmallVec, smallvec};

/// Return whether one chain operation carries static instantiation arguments.
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

/// Return whether one chain base head expression carries trailing static instantiation arguments.
fn chain_expression_has_trailing_static_instantiation_arguments(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Instantiation {
            static_arguments, ..
        } => !static_arguments.is_empty(),
        Expression::Path {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Parenthesized { expression } => {
            chain_expression_has_trailing_static_instantiation_arguments(tree, *expression)
        }
        _ => false,
    }
}

/// Return whether one chain node carries any comment annotation.
fn chain_node_has_any_comment_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Comment { .. }
                )
            })
        })
        .unwrap_or(false)
}

/// Build chain base, lines, break state, and static-instantiation wrap metadata.
pub(crate) fn chain_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<(
    ChainExpressionBase,
    Vec<SmallVec<[ChainExpression; 2]>>,
    bool,
    Option<usize>,
)> {
    let tree = context.tree;
    let chain = chain_nodes(tree, node_id);
    let root_id = chain[0];

    // only parenthesized roots are candidates for unwrap
    let base_root_id = if let Expression::Parenthesized { expression } = context.tree.get(root_id) {
        let root_is_statement_expression =
            context
                .parent(root_id)
                .is_some_and(|(parent_id, parent_type)| {
                    if parent_type != NodeType::Expression {
                        return false;
                    }

                    matches!(
                        context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                        Expression::Statement(expression_id) if expression_id.id == root_id.id
                    )
                });
        if root_is_statement_expression
            && matches!(
                context.tree.get(*expression),
                Expression::ObjectExpression { .. }
            )
        {
            root_id
        } else if should_unwrap_parenthesized_member_object(context, root_id, *expression) {
            *expression
        } else {
            root_id
        }
    } else {
        root_id
    };

    let mut base_head = ChainExpressionBaseHead::Expression(base_root_id);
    let mut operations = Vec::new();

    // split root path segments into explicit member chain operations
    if let Expression::Path {
        path,
        static_arguments,
    } = tree.get(base_root_id)
        && should_split_chain_root_path_segments(context, base_root_id)
    {
        let segments = &path.segments;
        let static_arguments = static_arguments.clone();
        let Some(first_segment) = segments.first().copied() else {
            return Err(FormatError::SyntaxError {
                message: "path chain root must contain at least one segment",
            });
        };

        let tail_segments = &segments[1..];
        let tail_len = tail_segments.len();
        let emit_postfix_on_tail = tail_len > 0
            && path_postfix_annotations_emit_on_tail(context, base_root_id, segments.len());
        let base_static_arguments = if tail_len == 0 {
            static_arguments.clone()
        } else {
            None
        };
        base_head = ChainExpressionBaseHead::Path {
            node_id: base_root_id,
            segment: first_segment,
            static_arguments: base_static_arguments,
            emit_postfix_annotations: tail_len == 0 || !emit_postfix_on_tail,
        };

        for (index, segment) in tail_segments.iter().copied().enumerate() {
            let is_last = index + 1 == tail_len;
            let static_args = if is_last {
                static_arguments.clone()
            } else {
                None
            };
            operations.push(ChainExpression::Member {
                node_id: base_root_id,
                segment,
                static_arguments: static_args,
                emit_prefix_annotations: false,
                emit_postfix_annotations: emit_postfix_on_tail && is_last,
            });
        }
    }

    let mut base = ChainExpressionBase {
        head: base_head,
        body: Vec::new(),
    };
    for expression_id in chain.iter().skip(1).copied() {
        operations.push(chain_expression_from_node(tree, expression_id)?);
    }

    let mut head_operation_count = 0usize;
    while head_operation_count < operations.len() {
        let operation = &operations[head_operation_count];
        let is_numeric_direct_index = matches!(
            operation,
            ChainExpression::Index {
                position: PostfixPosition::Direct,
                index,
                ..
            } if is_numeric_index(context, index)
        );
        if !matches!(
            operation,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            } | ChainExpression::Instantiation { .. }
                | ChainExpression::Maybe { .. }
                | ChainExpression::Must { .. }
        ) && !is_numeric_direct_index
        {
            break;
        }

        head_operation_count += 1;
    }

    let base_has_leading_call_like = match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => {
            let expression_id = transparent_inner_expression(context, *expression_id);
            let expression = context.tree.get(expression_id);
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
            ) || base.body.first().is_some_and(|operation| {
                matches!(
                    operation,
                    ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
                )
            })
        }
        ChainExpressionBaseHead::Path { .. } => base.body.first().is_some_and(|operation| {
            matches!(
                operation,
                ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
            )
        }),
    };
    if !base_has_leading_call_like {
        while head_operation_count + 1 < operations.len() {
            let operation = &operations[head_operation_count];
            let next_operation = &operations[head_operation_count + 1];
            if chain_node_has_non_inline_annotation(context, chain_operation_node_id(operation))
                || !matches!(
                    operation,
                    ChainExpression::Member { .. } | ChainExpression::Index { .. }
                )
                || !matches!(
                    next_operation,
                    ChainExpression::Member { .. } | ChainExpression::Index { .. }
                )
            {
                break;
            }

            head_operation_count += 1;
        }
    }

    let tail_operations = operations.split_off(head_operation_count);
    base.body = operations;

    let lines = tail_operations
        .into_iter()
        .map(|operation| smallvec![operation])
        .collect::<Vec<_>>();

    let chain_has_any_comment_annotation = chain
        .iter()
        .copied()
        .any(|expression_id| chain_node_has_any_comment_annotation(context, expression_id));
    let chain_body_has_breaking_member_annotation = base.body.iter().any(|operation| {
        let ChainExpression::Member { node_id, .. } = operation else {
            return false;
        };

        chain_node_has_breaking_annotation(context, *node_id)
    });
    let chain_root = chain[0];
    let chain_tail = chain[chain.len() - 1];
    let chain_head = chain_head_id(context.tree, chain_root);
    let has_chain_intervening_comment = chain_has_intervening_comment(context, &chain);
    let has_optional_call_boundary_trivia =
        chain_has_optional_call_boundary_trivia(context, &chain);
    let has_optional_tail = chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Maybe { .. }
                | Expression::Call {
                    position: PostfixPosition::Indirect,
                    ..
                }
        )
    });
    let has_member_access = chain.iter().copied().any(|expression_id| {
        matches!(
            context.tree.get(expression_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
    });
    let has_chain_annotations = chain_has_breaking_annotations(context, &chain, chain_head);
    let has_direct_curried_call_pair = chain.windows(2).any(|pair| {
        pair.iter().all(|expression_id| {
            matches!(
                context.tree.get(*expression_id),
                Expression::Call {
                    position: PostfixPosition::Direct,
                    ..
                }
            )
        })
    });
    let has_direct_call_with_inline_closure_cast_object_argument =
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
        });
    let head_call_requires_expanded_arguments = chain
        .iter()
        .copied()
        .find_map(|expression_id| {
            let Expression::Call {
                dynamic_arguments, ..
            } = context.tree.get(expression_id)
            else {
                return None;
            };

            Some(call_arguments_force_expand_for_chain(
                context,
                expression_id,
                dynamic_arguments,
            ))
        })
        .unwrap_or(false);
    let chain_has_following_operation = chain.len() > 1;
    let tail_parent_call_requires_chain_break =
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
            });
    let should_break = has_chain_annotations
        || (has_chain_intervening_comment && has_member_access)
        || has_optional_call_boundary_trivia
        || has_direct_call_with_inline_closure_cast_object_argument
        || (has_chain_intervening_comment && (has_optional_tail || has_direct_curried_call_pair))
        || (chain_has_following_operation && head_call_requires_expanded_arguments)
        || tail_parent_call_requires_chain_break
        || chain_overflows_in_type_binary_left(context, chain_tail)
        || chain_has_any_comment_annotation
        || chain_body_has_breaking_member_annotation;
    let instantiation_prefix_wrap_body_ops = {
        let head_has_static_instantiation_prefix = match &base.head {
            ChainExpressionBaseHead::Expression(expression_id) => {
                chain_expression_has_trailing_static_instantiation_arguments(
                    context.tree,
                    *expression_id,
                )
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
    };

    Ok((
        base,
        lines,
        should_break,
        instantiation_prefix_wrap_body_ops,
    ))
}
