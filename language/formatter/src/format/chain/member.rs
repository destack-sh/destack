use super::groups::{build_tail_chain_lines, chain_head_operation_count};
use crate::format::expression::should_unwrap_parenthesized_member_object;
use crate::format::operator::{
    is_chain_expression, needs_parens_in_postfix_position, write_postfix_base_expression,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, Argument, Declarator, Doc, DocStyle, Expression, LocalNodeId, NodeTree,
    NodeType, PostfixPosition, ScalarLiteral, TokenType,
};
use destack_core::StringId;
use destack_fir::format::{Buffer, FormatError, FormatResult};
use destack_fir::prelude::token;
use destack_fir::write;
use destack_source::Span;
use smallvec::SmallVec;

/// Extract a parenthesized base with a direct index chain.
pub(crate) fn extract_parenthesized_index_chain(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> Option<(LocalNodeId<Expression>, Vec<LocalNodeId<Expression>>)> {
    // collect direct index operations from the outside in
    let mut indices: Vec<LocalNodeId<Expression>> = Vec::new();
    let mut current = expression_id;

    while let Expression::Index {
        position: PostfixPosition::Direct,
        left,
        index: Some(index),
    } = tree.get(current)
    {
        indices.push(*index);
        current = *left;
    }

    if indices.is_empty() {
        return None;
    }

    let Expression::Parenthesized { expression } = tree.get(current) else {
        return None;
    };

    if needs_parens_in_postfix_position(tree, *expression) {
        return None;
    }

    indices.reverse();
    Some((*expression, indices))
}

/// Format a maybe expression without considering chaining.
pub(crate) fn format_maybe_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Maybe { left, position } = f.context().tree.get(node_id) {
        write_postfix_base_expression(f, *left)?;
        match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => write!(f, [token("."), token("?")])?,
        }
    } else {
        debug_assert!(false, "unexpected expression kind for maybe formatter");
    }
    Ok(())
}

/// The head of a chain before any postfix operations.
#[derive(Clone)]
pub(crate) enum ChainExpressionBaseHead {
    Path {
        node_id: LocalNodeId<Expression>,
        segment: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        emit_postfix_annotations: bool,
    },
    Expression(LocalNodeId<Expression>),
}

/// The initial portion of the chain including direct postfix ops.
#[derive(Clone)]
pub(crate) struct ChainExpressionBase {
    pub(crate) head: ChainExpressionBaseHead,
    pub(crate) body: Vec<ChainExpression>,
}

/// One operation in an expression chain.
#[derive(Clone)]
pub(crate) enum ChainExpression {
    /// Member expression.
    Member {
        node_id: LocalNodeId<Expression>,
        segment: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        emit_prefix_annotations: bool,
        emit_postfix_annotations: bool,
    },
    /// Instantiation expression.
    Instantiation {
        node_id: LocalNodeId<Expression>,
        static_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Call expression.
    Call {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Index expression.
    Index {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
        index: Option<LocalNodeId<Expression>>,
    },
    /// Maybe expression.
    Maybe {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    },
    /// Must expression.
    Must {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    },
}

/// Return the expression node id carried by one chain operation.
pub(crate) fn chain_operation_node_id(operation: &ChainExpression) -> LocalNodeId<Expression> {
    match operation {
        ChainExpression::Member { node_id, .. }
        | ChainExpression::Instantiation { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Maybe { node_id, .. }
        | ChainExpression::Must { node_id, .. } => *node_id,
    }
}

/// Return whether one chain operation is an index access.
pub(crate) fn chain_operation_is_index(operation: &ChainExpression) -> bool {
    matches!(operation, ChainExpression::Index { .. })
}

/// Return the first operation of the first grouped line.
pub(crate) fn first_grouped_line_operation(
    lines: &[SmallVec<[ChainExpression; 2]>],
) -> Option<&ChainExpression> {
    lines.first().and_then(|line| line.first())
}

/// Return the trailing node of one chain base.
pub(crate) fn chain_base_trailing_node_id(
    base: &ChainExpressionBase,
) -> Option<LocalNodeId<Expression>> {
    if let Some(last_operation) = base.body.last() {
        return Some(chain_operation_node_id(last_operation));
    }

    let ChainExpressionBaseHead::Path { node_id, .. } = base.head else {
        return None;
    };
    Some(node_id)
}

/// Return the left operand for one chain node.
pub(crate) fn chain_node_left_id(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(node_id) {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => Some(*left),
        _ => None,
    }
}

/// Collect all chain nodes from root to leaf.
pub(crate) fn chain_nodes(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> Vec<LocalNodeId<Expression>> {
    // walk from leaf to root through chain links
    let mut chain = Vec::new();
    let mut current = node_id;
    loop {
        chain.push(current);

        let Some(next_id) = chain_node_left_id(tree, current) else {
            break;
        };
        current = next_id;
    }
    chain.reverse();

    chain
}

/// Return the root expression id that should back one normalized chain base.
pub(crate) fn chain_base_root_expression_id(
    context: &DestackFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let Expression::Parenthesized { expression } = context.tree.get(root_id) else {
        return root_id;
    };

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
        return root_id;
    }

    if should_unwrap_parenthesized_member_object(context, root_id, *expression) {
        *expression
    } else {
        root_id
    }
}

/// Return whether one expression has a ternary expression ancestor.
pub(crate) fn expression_has_ternary_ancestor(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;

    while let Some((parent_id, parent_type)) = context.parent(current_id) {
        if parent_type != NodeType::Expression {
            return false;
        }

        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        if matches!(
            context.tree.get(parent_expression_id),
            Expression::If {
                kind: destack_ast::IfKind::Ternary,
                ..
            }
        ) {
            return true;
        }

        current_id = parent_expression_id;
    }

    false
}

/// Return whether a path chain has optional or must tail operators.
fn path_chain_has_optional_or_must_tail(
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

/// Build the normalized chain, base, and tail lines for one chain expression.
pub(crate) fn build_member_chain_parts(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    is_call_like_argument: bool,
) -> FormatResult<(
    Vec<LocalNodeId<Expression>>,
    ChainExpressionBase,
    Vec<SmallVec<[ChainExpression; 2]>>,
)> {
    let tree = context.tree;
    let chain = chain_nodes(tree, node_id);
    let root_id = chain[0];
    let base_root_id = chain_base_root_expression_id(context, root_id);

    let mut base_head = ChainExpressionBaseHead::Expression(base_root_id);
    let mut operations = Vec::new();

    // chain-member root decomposition
    if let Some((path_base_head, path_operations)) = split_path_chain_root(
        context,
        base_root_id,
        is_call_like_argument,
        expression_has_ternary_ancestor(context, base_root_id),
        path_chain_has_optional_or_must_tail(context, base_root_id),
    )? {
        base_head = path_base_head;
        operations.extend(path_operations);
    }

    let mut base = ChainExpressionBase {
        head: base_head,
        body: Vec::new(),
    };
    for expression_id in chain.iter().skip(1).copied() {
        operations.push(chain_expression_from_node(tree, expression_id)?);
    }

    let head_operation_count = chain_head_operation_count(context, &base, &operations);
    let tail_operations = operations.split_off(head_operation_count);
    base.body = operations;

    let lines = build_tail_chain_lines(context, tail_operations);

    Ok((chain, base, lines))
}

/// Return whether a root path segment looks factory-like.
fn path_root_segment_looks_factory_like(segment: &str) -> bool {
    let mut bytes = segment.bytes();

    match bytes.next() {
        Some(b'_' | b'$') => bytes.all(|byte| matches!(byte, b'_' | b'$')),
        Some(byte) => byte.is_ascii_uppercase(),
        None => false,
    }
}

/// Try to split one path root into a chain base head plus member operations.
pub(crate) fn split_path_chain_root(
    context: &DestackFormatContext<'_>,
    base_root_id: LocalNodeId<Expression>,
    is_call_like_argument: bool,
    has_ternary_ancestor: bool,
    has_optional_or_must_tail: bool,
) -> FormatResult<Option<(ChainExpressionBaseHead, Vec<ChainExpression>)>> {
    let Expression::Path {
        path,
        static_arguments,
    } = context.tree.get(base_root_id)
    else {
        return Ok(None);
    };

    let has_boundary_comments = context
        .annotation_ids(base_root_id)
        .iter()
        .any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id),
                Annotation::Doc {
                    position: AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            )
        });

    let first_segment = context.strings.get(path.segments[0]);
    let should_split_root_path_segments = path.segments.len() > 1
        && !is_call_like_argument
        && !has_ternary_ancestor
        && (!path_root_segment_looks_factory_like(first_segment) || has_boundary_comments)
        && (first_segment != "this" || has_optional_or_must_tail || has_boundary_comments);
    if !should_split_root_path_segments {
        return Ok(None);
    }

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
    let base_head = ChainExpressionBaseHead::Path {
        node_id: base_root_id,
        segment: first_segment,
        static_arguments: base_static_arguments,
        emit_postfix_annotations: tail_len == 0 || !emit_postfix_on_tail,
    };

    let mut operations = Vec::with_capacity(tail_len);
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

    Ok(Some((base_head, operations)))
}

/// Check whether source contains a comment between two expression nodes.
pub(crate) fn has_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };

    !context
        .comments_in_range(between_span.start, between_span.end)
        .is_empty()
}

/// Return whether a `//` comment exists between two expression nodes.
pub(crate) fn has_line_comment_between_expressions(
    context: &DestackFormatContext<'_>,
    left_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left_id);
    let right_span = context.span(right_id);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };

    context
        .comments_in_range(between_span.start, between_span.end)
        .iter()
        .copied()
        .any(|comment| context.comment_is_line(comment))
}

/// Get the root head expression of a chain.
pub(crate) fn chain_head_id(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current = node_id;

    loop {
        let next = chain_node_left_id(tree, current);

        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(next_id) => return next_id,
            None => return current,
        }
    }
}

/// Check whether an optional index is numeric-simple.
pub(crate) fn is_numeric_index(
    context: &DestackFormatContext<'_>,
    index: &Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(index_id) = *index else {
        return false;
    };
    if context.has_annotation(index_id) {
        return false;
    }

    matches!(
        context.tree.get(index_id),
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Bigint(_) | ScalarLiteral::Float(_)
        )
    )
}

/// Return an expression end anchor used for chain trivia checks.
pub(crate) fn expression_trivia_anchor_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression = context.tree.get(expression_id);
    let span = context.span(expression_id);

    // member like nodes often include trailing boundary comments in their full spans
    // so anchor at the property token to inspect the comment gap before parent operators
    match expression {
        Expression::Member { .. } | Expression::PrivateMember { .. } => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |member_span| member_span.end),
        Expression::Path { path, .. } if path.segments.len() == 1 => context
            .tree
            .get_main_span(expression_id)
            .map_or(span.end, |path_span| path_span.end),
        _ => span.end,
    }
}

/// Check if a member access has an intervening comment between receiver and property.
pub(crate) fn member_has_intervening_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((left, property_span)) = (match context.tree.get(node_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => context
            .tree
            .get_main_span(node_id)
            .map(|property_span| (*left, property_span)),
        _ => None,
    }) else {
        return false;
    };
    let left_span = context.span(left);
    let left_anchor_end = expression_trivia_anchor_end(context, left);
    if property_span.start <= left_anchor_end {
        return false;
    }

    let span = Span::new(left_span.file, left_anchor_end, property_span.start);
    !context.comments_in_range(span.start, span.end).is_empty()
}

/// Find the first non-trivia parent operator token after one chain node.
fn chain_parent_operator_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parent_id: LocalNodeId<Expression>,
) -> Option<u32> {
    let node_span = context.span(node_id);
    let parent_span = context.span(parent_id);
    if node_span.file != parent_span.file {
        return None;
    }

    let node_anchor_end = expression_trivia_anchor_end(context, node_id);
    if parent_span.end <= node_anchor_end {
        return None;
    }

    context
        .first_non_trivia_token_between(node_anchor_end, parent_span.end)
        .map(|token| token.span.start)
}

/// Check if a chain node has source breaks or comments before its parent operator.
pub(crate) fn chain_has_parent_intervening_break_or_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent = context.tree.get(parent_id);
    let parent_uses_node_as_left = match parent {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => *left == node_id,
        _ => false,
    };
    if !parent_uses_node_as_left {
        return false;
    }

    let should_check_parent_gap = match parent {
        Expression::Member { .. } | Expression::PrivateMember { .. } => true,
        Expression::Call { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => matches!(
            context.tree.get(node_id),
            Expression::Member { .. } | Expression::PrivateMember { .. } | Expression::Path { .. }
        ),
        _ => false,
    };
    if !should_check_parent_gap {
        return false;
    }

    let node_span = context.span(node_id);
    let node_anchor_end = expression_trivia_anchor_end(context, node_id);
    let Some(parent_operator_start) = chain_parent_operator_start(context, node_id, parent_id)
    else {
        return false;
    };
    if parent_operator_start <= node_anchor_end {
        return false;
    }
    let between_span = Span::new(node_span.file, node_anchor_end, parent_operator_start);

    !context
        .comments_in_range(between_span.start, between_span.end)
        .is_empty()
}

/// Return whether a member has only one promotable boundary comment.
pub(crate) fn chain_member_has_promotable_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let annotations = context.annotation_ids(node_id);
    if annotations.is_empty() {
        return false;
    }

    let mut found_promotable_boundary_comment = false;
    for annotation_id in annotations.iter().copied() {
        match context.annotation(annotation_id) {
            Annotation::Doc {
                node,
                position: AnnotationPosition::LinePostfixBoundary,
            } => {
                let doc = context.tree.get::<Doc>(node);
                if doc.style == DocStyle::Star {
                    let comment_span = context.span(node);
                    if context.has_newline(comment_span) {
                        return false;
                    }
                }
                found_promotable_boundary_comment = true;
            }
            _ => return false,
        }
    }

    found_promotable_boundary_comment
}

/// Check whether a member access uses a private hash (`.#name`).
pub(crate) fn member_is_private_hash(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(context.tree.get(node_id), Expression::PrivateMember { .. }) {
        return true;
    }

    let Some(property_span) = context.tree.get_main_span(node_id) else {
        return false;
    };

    let Some(prev_token) = context.token_before_token_start(property_span.start) else {
        return false;
    };

    prev_token.token.ty == TokenType::Hash
}

/// Decide whether postfix annotations on a path belong after the last segment.
pub(crate) fn path_postfix_annotations_emit_on_tail(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> bool {
    let Some(last_segment_start) = path_last_segment_start(context, node_id, segments_len) else {
        return false;
    };

    let mut has_postfix = false;

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        let annotation = context.annotation(annotation_id);
        let position = annotation.position();
        let is_postfix = matches!(
            position,
            AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockInfix
                | AnnotationPosition::BlockPostfix
        );
        if !is_postfix {
            continue;
        }

        has_postfix = true;
        let span = context.annotation_span(annotation_id);
        if span.start < last_segment_start {
            return false;
        }
    }

    if !has_postfix {
        return true;
    }

    true
}

/// Find the start byte of the last path segment token.
fn path_last_segment_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    segments_len: usize,
) -> Option<u32> {
    if segments_len == 0 {
        return None;
    }

    let span = context.span(node_id);
    context.nth_token_type_start_in_span(span, TokenType::Identifier, segments_len)
}

/// Convert one chain expression node into a chain operation.
pub(crate) fn chain_expression_from_node(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<ChainExpression> {
    let chain_expression = match tree.get(expression_id) {
        Expression::Member {
            name,
            static_arguments,
            ..
        } => {
            let Some(name) = *name else {
                return Err(FormatError::SyntaxError {
                    message: "missing member name in chain expression",
                });
            };
            ChainExpression::Member {
                node_id: expression_id,
                segment: name,
                static_arguments: static_arguments.clone(),
                emit_prefix_annotations: true,
                emit_postfix_annotations: true,
            }
        }
        Expression::PrivateMember {
            name,
            static_arguments,
            ..
        } => {
            let Some(name) = *name else {
                return Err(FormatError::SyntaxError {
                    message: "missing private member name in chain expression",
                });
            };
            ChainExpression::Member {
                node_id: expression_id,
                segment: name,
                static_arguments: static_arguments.clone(),
                emit_prefix_annotations: true,
                emit_postfix_annotations: true,
            }
        }
        Expression::Call {
            position,
            static_arguments,
            dynamic_arguments,
            ..
        } => ChainExpression::Call {
            node_id: expression_id,
            position: *position,
            static_arguments: static_arguments.clone(),
            dynamic_arguments: dynamic_arguments.clone(),
        },
        Expression::Instantiation {
            static_arguments, ..
        } => ChainExpression::Instantiation {
            node_id: expression_id,
            static_arguments: static_arguments.clone(),
        },
        Expression::Index {
            position, index, ..
        } => ChainExpression::Index {
            node_id: expression_id,
            position: *position,
            index: *index,
        },
        Expression::Maybe { position, .. } => ChainExpression::Maybe {
            node_id: expression_id,
            position: *position,
        },
        Expression::Must { position, .. } => ChainExpression::Must {
            node_id: expression_id,
            position: *position,
        },
        _ => {
            return Err(FormatError::SyntaxError {
                message: "unexpected expression kind for chain expression",
            });
        }
    };

    Ok(chain_expression)
}

/// Check whether the expression is part of a member/call/maybe/index chain.
pub(crate) fn is_expression_chain(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
    chain_node_left_id(tree, node_id).is_some_and(|left_id| is_chain_expression(tree.get(left_id)))
}

/// Check whether an expression is the root of a chain.
pub(crate) fn is_chain_root(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
    chain_node_left_id(tree, node_id).is_some_and(|left_id| !is_chain_expression(tree.get(left_id)))
}

/// Check whether this expression is used as the receiver in a chain parent.
pub(crate) fn has_chain_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent_by_id(node_id.id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    match parent_expr {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Call { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => left.id == node_id.id,
        Expression::Index { .. } => false,
        _ => false,
    }
}

/// Walk upward through transparent wrappers to find an assignment-like parent rhs.
pub(crate) fn assignment_like_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<(NodeType, u32)> {
    let mut current_id = node_id.id;

    // walk through transparent wrappers until we reach an assignment-like parent
    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        match parent_type {
            NodeType::Expression => {
                let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));

                // assignment rhs
                if let Expression::Assign { right, .. } = parent_expr
                    && right.id == current_id
                {
                    return Some((NodeType::Expression, parent_id));
                }

                // transparent wrappers around the rhs
                let is_wrapper_parent = matches!(
                    parent_expr,
                    Expression::Await { expression }
                        | Expression::AwaitMaybe { expression }
                        | Expression::Parenthesized { expression }
                        if expression.id == current_id
                );

                if is_wrapper_parent {
                    current_id = parent_id;
                    continue;
                }

                return None;
            }
            NodeType::Declarator => {
                let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
                if declarator.value.is_some_and(|value| value.id == current_id) {
                    return Some((NodeType::Declarator, parent_id));
                }
                return None;
            }
            _ => return None,
        }
    }

    None
}

/// Unwrap transparent wrappers around an expression for classification.
pub(crate) fn transparent_inner_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    context.transparent_inner_expression(node_id)
}

/// Decide whether a nullish coalescing operator should trail on a new line.
pub(crate) fn should_use_trailing_coalesce(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
    if !matches!(parent_expression, Expression::Parenthesized { .. }) {
        return false;
    }

    let left_is_chain = matches!(
        context.tree.get(left),
        Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::Call { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    );
    if !left_is_chain {
        return false;
    }

    let expression_span = context.span(node_id);
    if context.has_newline(expression_span) {
        return true;
    }

    false
}
