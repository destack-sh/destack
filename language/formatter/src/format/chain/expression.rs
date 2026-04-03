use super::member::member_intervening_comment_nodes;
use super::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead, assignment_like_parent,
    build_member_chain_parts, chain_base_trailing_node_id,
    chain_instantiation_prefix_wrap_body_ops, chain_operation_is_call_like,
    chain_operation_is_index, chain_operation_node_id, chain_should_break,
    expression_has_ternary_ancestor, first_grouped_line_operation, is_nested_lambda_expression,
    member_is_private_hash, transparent_inner_expression,
};
use crate::format::call::format_call_arguments;
use crate::format::expression::{
    format_static_argument_list, format_static_argument_list_with_relational_spacing,
};
use crate::format::operator::write_postfix_base_expression;
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, Doc, DocStyle, Expression, LocalNodeId, NodeType, PostfixPosition,
    TokenType, TypeBinaryOperator,
};
use destack_fir::format::{BestFittingMode, Buffer, FormatResult};
use destack_fir::prelude::{
    expand_parent, format_with, group, hard_line_break, indent, space, token,
};
use destack_fir::{best_fitting, write};
use smallvec::SmallVec;

/// Return whether an expression appears in call-like argument position.
pub(crate) fn is_call_like_argument(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((argument_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }

    let Some((expression_id, expression_type)) = context.parent_by_id(argument_id) else {
        return false;
    };
    if expression_type != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. }
    )
}

/// Return whether one expression is await-like.
pub(crate) fn expression_is_await_like(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Await { .. } | Expression::AwaitMaybe { .. } | Expression::Comptime { .. }
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
    expression_is_await_like(context.tree.get(*expression))
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

    if context.has_annotation(parent_expression_id) {
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

    context.has_annotation(great_grandparent_expression_id)
}

/// Check whether an assignment chain ends in a nested lambda expression.
pub(crate) fn is_assignment_chain_tail_lambda(
    context: &DestackFormatContext<'_>,
    assignment_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    if assignment_like_parent(context, assignment_id).is_none() {
        return false;
    }

    if matches!(context.tree.get(right_id), Expression::Assign { .. }) {
        return false;
    }

    is_nested_lambda_expression(context, right_id)
}

/// Check whether a chain node has annotations that prevent head grouping.
pub(crate) fn chain_node_has_non_inline_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    chain_node_has_forcing_annotation(context, node_id, false)
}

/// Return whether one node annotation should count for chain formatting.
pub(crate) fn chain_node_has_forcing_annotation(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    is_breaking_scan: bool,
) -> bool {
    let is_chain_link = crate::format::operator::is_chain_expression(context.tree.get(node_id));
    let has_optional_tail_boundary_comment = is_breaking_scan
        && matches!(
            context.tree.get(node_id),
            Expression::Member { .. } | Expression::PrivateMember { .. }
        )
        && context.annotation_ids(node_id).iter().any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id),
                Annotation::Doc {
                    position: AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            )
        })
        && context
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
            });

    context.annotation_ids(node_id).iter().any(|annotation_id| {
        let annotation = context.annotation(*annotation_id);
        let position = annotation.position();
        let is_optional_tail_boundary_comment = has_optional_tail_boundary_comment
            && matches!(
                annotation,
                Annotation::Doc {
                    position: AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            );

        if !is_optional_tail_boundary_comment
            && (chain_annotation_is_inline_non_breaking(context, *annotation_id)
                || position == AnnotationPosition::BlockInfix
                    && matches!(
                        context.tree.get(node_id),
                        Expression::Call { .. }
                            | Expression::Instantiation { .. }
                            | Expression::New { .. }
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

        matches!(
            annotation,
            Annotation::Doc { .. } | Annotation::Decorator { .. }
        ) && matches!(
            position,
            AnnotationPosition::LinePrefix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockPrefix
                | AnnotationPosition::BlockInfix
                | AnnotationPosition::BlockPostfix
        )
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
        Annotation::Doc { node, .. } => context.tree.get::<Doc>(node).style == DocStyle::Star,
        Annotation::Decorator { .. } => false,
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

/// One normalized chain layout owned by the chain formatter.
struct MemberChain {
    chain: Vec<LocalNodeId<Expression>>,
    base: ChainExpressionBase,
    lines: Vec<SmallVec<[ChainExpression; 2]>>,
    instantiation_prefix_wrap_body_ops: Option<usize>,
}

impl MemberChain {
    /// Build the normalized chain layout for one expression.
    fn from_expression(
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
    ) -> FormatResult<Self> {
        let (chain, base, lines) =
            build_member_chain_parts(context, node_id, is_call_like_argument(context, node_id))?;
        let instantiation_prefix_wrap_body_ops =
            chain_instantiation_prefix_wrap_body_ops(context, &base, &lines);

        Ok(Self {
            chain,
            base,
            lines,
            instantiation_prefix_wrap_body_ops,
        })
    }
}

/// Format a member/call/maybe/index chain with prettier-style breaking.
pub(crate) fn format_expression_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let chain = MemberChain::from_expression(f.context(), node_id)?;
    let base = &chain.base;
    let lines = &chain.lines;
    let chain_should_break = chain_should_break(f.context(), &chain.chain, &chain.base);
    let instantiation_prefix_wrap_body_ops = chain.instantiation_prefix_wrap_body_ops;

    let deferred_base_boundary_owner_node_id = if lines.is_empty() {
        None
    } else {
        let base_trailing_node_id = chain_base_trailing_node_id(base);
        base_trailing_node_id.filter(|node_id| {
            let mut has_boundary_postfix_annotation = false;

            for annotation_id in f.context().annotation_ids(*node_id).iter().copied() {
                let annotation = f.context().annotation(annotation_id);
                if annotation.position() != AnnotationPosition::LinePostfixBoundary {
                    continue;
                }

                has_boundary_postfix_annotation = true;
                if !f
                    .context()
                    .span_starts_on_own_line(f.context().annotation_span(annotation_id))
                {
                    return false;
                }
            }

            has_boundary_postfix_annotation
        })
    };

    let format_one_line_chain = format_with(|f| {
        format_chain_base_content(
            f,
            node_id,
            &base,
            &lines,
            instantiation_prefix_wrap_body_ops,
            deferred_base_boundary_owner_node_id,
        )?;

        for line in lines {
            format_chain_expression_line(
                f,
                node_id,
                line,
                deferred_base_boundary_owner_node_id,
                false,
            )?;
        }

        Ok(())
    });

    let format_expanded_chain = format_with(|f| {
        if chain_should_break {
            write!(f, [expand_parent()])?;
        }

        format_chain_base_content(
            f,
            node_id,
            &base,
            &lines,
            instantiation_prefix_wrap_body_ops,
            deferred_base_boundary_owner_node_id,
        )?;

        if lines.is_empty() {
            return Ok(());
        }

        let skip_first_soft_break_for_conditional_head =
            expression_has_ternary_ancestor(f.context(), node_id)
                && base.body.last().is_some_and(chain_operation_is_call_like)
                && lines.first().is_some_and(|line| {
                    matches!(
                        line.as_slice(),
                        [
                            ChainExpression::Member { .. },
                            operation
                        ]
                        if chain_operation_is_call_like(operation)
                    )
                });
        let format_lines = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            for (line_index, line) in lines.iter().enumerate() {
                let should_skip_first_break =
                    line_index == 0 && skip_first_soft_break_for_conditional_head;
                let should_insert_break = !should_skip_first_break
                    && (line_index == 0
                        || !line.first().is_some_and(|operation| {
                            let node_id = chain_operation_node_id(operation);

                            f.context()
                                .annotation_ids(node_id)
                                .iter()
                                .any(|annotation_id| {
                                    matches!(
                                        f.context().annotation(*annotation_id).position(),
                                        AnnotationPosition::BlockPrefix
                                    )
                                })
                        }));
                if should_insert_break {
                    if let Some(ChainExpression::Member { node_id, .. }) = line.first() {
                        write_member_gap_comments(f, *node_id)?;
                    }

                    write!(f, [hard_line_break()])?;
                }

                if line_index == 0
                    && let Some(owner_node_id) = deferred_base_boundary_owner_node_id
                {
                    write!(
                        f,
                        [
                            crate::format::annotation::line_postfix_boundary_annotations(
                                f.context(),
                                owner_node_id
                            )
                        ]
                    )?;
                }

                format_chain_expression_line(
                    f,
                    node_id,
                    line,
                    deferred_base_boundary_owner_node_id,
                    should_insert_break,
                )?;
            }

            Ok(())
        });

        write!(f, [indent(&format_lines)])?;

        Ok(())
    });

    if chain_should_break {
        return write!(f, [group(&format_expanded_chain).should_expand(true)]);
    }

    if lines.is_empty() {
        return write!(f, [group(&format_one_line_chain)]);
    }

    write!(
        f,
        [
            best_fitting![group(&format_one_line_chain), group(&format_expanded_chain)]
                .with_mode(BestFittingMode::AllLines)
        ]
    )
}

/// Format the unwrapped base segment of a chain.
fn write_member_gap_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let comment_nodes = member_intervening_comment_nodes(f.context(), node_id);
    if comment_nodes.is_empty() {
        return Ok(());
    }

    write!(f, [space()])?;

    for (index, comment_id) in comment_nodes.iter().copied().enumerate() {
        let comment_span = f.context().span(comment_id);
        write!(f, [comment_id])?;

        let is_last = index + 1 == comment_nodes.len();
        if !is_last
            || f.context()
                .span_has_newline_before_next_non_whitespace_token(comment_span)
        {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Format the unwrapped base segment of a chain.
fn format_chain_base_content<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    instantiation_prefix_wrap_body_ops: Option<usize>,
    deferred_base_boundary_owner_node_id: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let root_is_decorator_expression =
        f.context()
            .parent(formatted_root_id)
            .is_some_and(|(_, parent_type)| {
                matches!(parent_type, NodeType::Decorator | NodeType::Annotation)
            });
    let mut has_open_prefix_wrap = false;
    let mut has_closed_prefix_wrap = false;
    if instantiation_prefix_wrap_body_ops.is_some() {
        write!(f, [token("(")])?;
        has_open_prefix_wrap = true;
    }

    match &base.head {
        ChainExpressionBaseHead::Path {
            node_id,
            segment,
            static_arguments,
            emit_postfix_annotations,
        } => {
            if !root_is_decorator_expression {
                write!(
                    f,
                    [crate::format::annotation::prefix_annotations(
                        f.context(),
                        *node_id
                    )]
                )?;
            }
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                let next_operation = base
                    .body
                    .first()
                    .or_else(|| first_grouped_line_operation(lines));
                if next_operation.is_some_and(chain_operation_is_index) {
                    format_static_argument_list_with_relational_spacing(f, arguments)?;
                } else {
                    format_static_argument_list(f, arguments)?;
                }
            }
            if *emit_postfix_annotations {
                let should_defer_boundary_annotations =
                    deferred_base_boundary_owner_node_id == Some(*node_id);
                if should_defer_boundary_annotations {
                    write!(
                        f,
                        [crate::format::annotation::infix_or_postfix_annotations_without_line_postfix_boundary(
                            f.context(),
                            *node_id
                        )]
                    )?;
                } else {
                    write!(
                        f,
                        [crate::format::annotation::infix_or_postfix_annotations(
                            f.context(),
                            *node_id
                        )]
                    )?;
                }
            }
        }
        ChainExpressionBaseHead::Expression(node_id) => {
            // chain normalization can still surface nested chain nodes as a base
            // write the base expression directly instead of panicking in debug mode
            write_postfix_base_expression(f, *node_id)?;
            if deferred_base_boundary_owner_node_id == Some(*node_id) {
                write!(
                    f,
                    [crate::format::annotation::infix_or_postfix_annotations_without_line_postfix_boundary(
                        f.context(),
                        *node_id
                    )]
                )?;
            } else {
                write!(
                    f,
                    [crate::format::annotation::infix_or_postfix_annotations(
                        f.context(),
                        *node_id
                    )]
                )?;
            }
        }
    }

    if instantiation_prefix_wrap_body_ops == Some(0) {
        write!(f, [token(")")])?;
        has_closed_prefix_wrap = true;
    }

    for (index, op) in base.body.iter().enumerate() {
        let next_operation = base
            .body
            .get(index + 1)
            .or_else(|| first_grouped_line_operation(lines));
        format_chain_expression(
            f,
            formatted_root_id,
            op,
            next_operation,
            deferred_base_boundary_owner_node_id,
        )?;
        if !has_closed_prefix_wrap && instantiation_prefix_wrap_body_ops == Some(index + 1) {
            write!(f, [token(")")])?;
            has_closed_prefix_wrap = true;
        }
    }

    if has_open_prefix_wrap && !has_closed_prefix_wrap {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one chained operation.
fn format_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    op: &ChainExpression,
    next_operation: Option<&ChainExpression>,
    deferred_base_boundary_owner_node_id: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // output any line prefix annotations before the operation
    let (node_id, emit_prefix_annotations, emit_postfix_annotations) = match op {
        ChainExpression::Member {
            node_id,
            emit_prefix_annotations,
            emit_postfix_annotations,
            ..
        } => (
            *node_id,
            *emit_prefix_annotations,
            *emit_postfix_annotations,
        ),
        ChainExpression::Instantiation { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Maybe { node_id, .. }
        | ChainExpression::Must { node_id, .. } => {
            let should_emit_prefix_annotations = match op {
                ChainExpression::Call { node_id, .. } => match f.context().tree.get(*node_id) {
                    Expression::Call { left, .. } => {
                        if !f.context().has_prefix_annotation(*node_id) {
                            false
                        } else if !f.context().has_prefix_annotation(*left) {
                            true
                        } else {
                            let left_span = f.context().span(*left);
                            let has_leading_prefix_before_left = f
                                .context()
                                .annotation_ids(*node_id)
                                .iter()
                                .any(|annotation_id| {
                                    let annotation = f.context().annotation(*annotation_id);
                                    if !matches!(
                                        annotation.position(),
                                        AnnotationPosition::BlockPrefix
                                            | AnnotationPosition::LinePrefix
                                    ) {
                                        return false;
                                    }

                                    let annotation_span =
                                        f.context().annotation_span(*annotation_id);
                                    annotation_span.start < left_span.start
                                });

                            !has_leading_prefix_before_left
                        }
                    }
                    _ => f.context().has_prefix_annotation(*node_id),
                },
                _ => true,
            };
            (*node_id, should_emit_prefix_annotations, true)
        }
    };
    let call_or_new_handles_empty_infix = matches!(
        op,
        ChainExpression::Call {
            node_id,
            dynamic_arguments,
            ..
        } if dynamic_arguments.is_empty() && f.context().has_infix_annotation(*node_id)
    );
    let emit_prefix_annotations = emit_prefix_annotations && node_id != formatted_root_id;
    let root_postfix_owned_by_outer_context = node_id == formatted_root_id;
    let emit_postfix_annotations = emit_postfix_annotations && !root_postfix_owned_by_outer_context;
    if emit_prefix_annotations {
        write!(
            f,
            [crate::format::annotation::prefix_annotations(
                f.context(),
                node_id
            )]
        )?;
    }

    match op {
        ChainExpression::Member {
            node_id,
            segment,
            static_arguments,
            ..
        } => {
            let is_private_hash = member_is_private_hash(f.context(), *node_id);
            write!(f, [token(".")])?;
            if is_private_hash {
                write!(f, [token("#")])?;
            }
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                if next_operation.is_some_and(chain_operation_is_index) {
                    format_static_argument_list_with_relational_spacing(f, arguments)?;
                } else {
                    format_static_argument_list(f, arguments)?;
                }
            }
        }
        ChainExpression::Instantiation {
            static_arguments, ..
        } => {
            if next_operation.is_some_and(chain_operation_is_index) {
                format_static_argument_list_with_relational_spacing(f, static_arguments)?;
            } else {
                format_static_argument_list(f, static_arguments)?;
            }
        }
        ChainExpression::Call {
            node_id: call_node_id,
            position,
            static_arguments,
            dynamic_arguments,
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(arguments) = static_arguments {
                format_static_argument_list(f, arguments)?;
            }
            format_call_arguments(f, *call_node_id, dynamic_arguments)?;
        }
        ChainExpression::Index {
            position, index, ..
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(index) = index {
                let should_parenthesize = if matches!(
                    f.context().tree.get(*index),
                    Expression::Parenthesized { .. }
                ) {
                    false
                } else {
                    let inner_index_id = transparent_inner_expression(f.context(), *index);
                    matches!(
                        f.context().tree.get(inner_index_id),
                        Expression::Assign { .. }
                    )
                };
                if should_parenthesize {
                    write!(f, [token("["), token("("), *index, token(")"), token("]")])?;
                } else {
                    write!(f, [token("["), *index, token("]")])?;
                }
            } else {
                write!(f, [token("[]")])?;
            }
        }
        ChainExpression::Maybe { position, .. } => match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("?")])?;
            }
        },
        ChainExpression::Must { position, .. } => match position {
            PostfixPosition::Direct => write!(f, [token("!")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("!")])?;
            }
        },
    }

    // output any line postfix annotations after the operation
    if emit_postfix_annotations {
        let should_defer_boundary_annotations =
            deferred_base_boundary_owner_node_id == Some(node_id);
        if call_or_new_handles_empty_infix {
            write!(
                f,
                [crate::format::annotation::postfix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        } else if should_defer_boundary_annotations {
            write!(
                f,
                [crate::format::annotation::infix_or_postfix_annotations_without_line_postfix_boundary(
                    f.context(),
                    node_id
                )]
            )?;
        } else {
            write!(
                f,
                [crate::format::annotation::infix_or_postfix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        }
    }

    Ok(())
}

/// Format all operations for one chain line.
fn format_chain_expression_line<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    ops: &[ChainExpression],
    deferred_base_boundary_owner_node_id: Option<LocalNodeId<Expression>>,
    skip_first_member_gap_comments: bool,
) -> FormatResult<()> {
    for (index, op) in ops.iter().enumerate() {
        if let ChainExpression::Member { node_id, .. } = op
            && (!skip_first_member_gap_comments || index > 0)
        {
            write_member_gap_comments(f, *node_id)?;
        }

        let next_operation = ops.get(index + 1);
        format_chain_expression(
            f,
            formatted_root_id,
            op,
            next_operation,
            deferred_base_boundary_owner_node_id,
        )?;
    }
    Ok(())
}
