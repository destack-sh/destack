use ast::{
    AnnotationPosition, Block, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
};
use destack_ast as ast;
use destack_source::{EnclosingSpan, Span};
use smallvec::SmallVec;

use super::index::FormatterTriviaOwnerIndex;

/// Return whether one node kind is excluded from trivia owner indexing.
pub(super) fn is_trivia_excluded_owner_node_id(tree: &NodeTree, node_id: u32) -> bool {
    matches!(
        tree.get_node_type(node_id),
        NodeType::Annotation
            | NodeType::Doc
            | NodeType::Comment
            | NodeType::Blank
            | NodeType::Decorator
    )
}

/// Normalize one trivia owner id to the canonical structural owner.
pub(super) fn normalize_formatter_trivia_target_owner(tree: &NodeTree, owner_id: u32) -> u32 {
    let mut current_id = owner_id;

    loop {
        if tree.get_node_type(current_id) != NodeType::Expression {
            return current_id;
        }

        let expression_id = LocalNodeId::<Expression>::new(current_id);
        match tree.get(expression_id) {
            // statement wrappers are transparent for trivia ownership
            Expression::Statement(expression) => {
                current_id = expression.id;
            }
            // parenthesized wrappers around declaration expressions are transparent
            Expression::Parenthesized { expression }
                if matches!(tree.get(*expression), Expression::Declaration(_)) =>
            {
                current_id = expression.id;
            }
            // declaration expression trivia belongs to the declaration node
            Expression::Declaration(declaration) => {
                return declaration.id;
            }
            _ => {
                return current_id;
            }
        }
    }
}

/// Return one preferred owner that starts at one token span.
pub(super) fn find_preferred_owner_starting_at(tree: &NodeTree, span: Span) -> Option<u32> {
    let mut best_owner: Option<EnclosingSpan> = None;
    tree.source_map
        .visit_enclosing_spans(span.start, span.end.saturating_sub(1), |candidate| {
            if candidate.span.start != span.start
                || is_trivia_excluded_owner_node_id(tree, candidate.idx)
            {
                return;
            }

            let candidate_kind_rank = if tree.get_node_type(candidate.idx) == NodeType::Expression {
                1
            } else {
                0
            };

            let should_replace = if let Some(current) = best_owner {
                let current_kind_rank = if tree.get_node_type(current.idx) == NodeType::Expression {
                    1
                } else {
                    0
                };
                candidate.length < current.length
                    || (candidate.length == current.length
                        && candidate_kind_rank < current_kind_rank)
                    || (candidate.length == current.length
                        && candidate_kind_rank == current_kind_rank
                        && candidate.idx < current.idx)
            } else {
                true
            };

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Return one smallest owner that encloses one token span.
pub(super) fn find_smallest_owner_enclosing_token(tree: &NodeTree, span: Span) -> Option<u32> {
    let mut best_owner: Option<EnclosingSpan> = None;
    tree.source_map
        .visit_enclosing_spans(span.start, span.end.saturating_sub(1), |candidate| {
            if is_trivia_excluded_owner_node_id(tree, candidate.idx) {
                return;
            }

            let should_replace = if let Some(current) = best_owner {
                candidate.length < current.length
                    || (candidate.length == current.length && candidate.idx < current.idx)
            } else {
                true
            };

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Return one smallest owner that encloses one seam range.
pub(super) fn find_smallest_owner_enclosing_range(
    tree: &NodeTree,
    start: u32,
    end: u32,
) -> Option<u32> {
    if start >= end {
        return None;
    }

    let mut best_owner: Option<EnclosingSpan> = None;
    tree.source_map
        .visit_enclosing_spans(start, end.saturating_sub(1), |candidate| {
            if is_trivia_excluded_owner_node_id(tree, candidate.idx) {
                return;
            }

            let should_replace = if let Some(current) = best_owner {
                candidate.length < current.length
                    || (candidate.length == current.length && candidate.idx < current.idx)
            } else {
                true
            };

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Promote one owner while ancestor spans share the same seam end.
pub(super) fn promote_owner_by_shared_end(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    seam_end: u32,
) -> u32 {
    let mut current_id = owner_id;
    let mut best_id = owner_id;

    while let Some(parent_id) = parents.get_by_id(current_id) {
        let parent_span = tree.get_span_by_id(parent_id);
        if parent_span.end != seam_end {
            break;
        }
        if is_trivia_excluded_owner_node_id(tree, parent_id) {
            break;
        }

        best_id = parent_id;
        current_id = parent_id;
    }

    best_id
}

/// Promote one owner while ancestor spans share the same seam start.
pub(super) fn promote_owner_by_shared_start(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    seam_start: u32,
) -> u32 {
    let mut current_id = owner_id;
    let mut best_id = owner_id;

    while let Some(parent_id) = parents.get_by_id(current_id) {
        let parent_span = tree.get_span_by_id(parent_id);
        if parent_span.start != seam_start {
            break;
        }
        if is_trivia_excluded_owner_node_id(tree, parent_id) {
            break;
        }

        best_id = parent_id;
        current_id = parent_id;
    }

    best_id
}

/// Return one block-interior placement target for boundary comments.
pub(super) fn resolve_block_leading_comment_target(
    tree: &NodeTree,
    block_id: LocalNodeId<Block>,
) -> (u32, AnnotationPosition) {
    let block = tree.get(block_id);
    if let Some(first_expression) = block.expressions.first().copied() {
        return (first_expression.id, AnnotationPosition::BlockPrefix);
    }

    if block.format == ast::BlockFormat::Explicit {
        return (block_id.id, AnnotationPosition::BlockInfix);
    }

    (block_id.id, AnnotationPosition::BlockPrefix)
}

/// Promote one owner to the nearest declaration ancestor.
pub(super) fn promote_owner_to_declaration_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Declaration {
            return Some(node_id);
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Promote one owner to the nearest ancestor of one node type.
pub(super) fn promote_owner_to_node_type_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    node_type: NodeType,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == node_type {
            return Some(node_id);
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Promote one owner to the nearest `satisfies` expression ancestor.
pub(super) fn promote_owner_to_satisfies_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if matches!(
                tree.get(expression_id),
                Expression::TypeBinary {
                    operator: ast::TypeBinaryOperator::Satisfies,
                    ..
                }
            ) {
                return Some(node_id);
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Promote one owner to the nearest parenthesized expression ancestor.
pub(super) fn promote_owner_to_parenthesized_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if matches!(tree.get(expression_id), Expression::Parenthesized { .. }) {
                return Some(node_id);
            }
        }

        current_id = parents.get_by_id(node_id);
    }

    None
}

/// Find the next declaration owner at or after one semantic token index.
pub(super) fn find_next_declaration_owner_from_token(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: usize,
) -> Option<u32> {
    let token_count = owner_index.owner_start_by_token.len();
    if token_index >= token_count {
        return None;
    }

    let search_end = (token_index + 96).min(token_count);
    for current_index in token_index..search_end {
        let candidate_owner = owner_index.owner_start_by_token[current_index]
            .or(owner_index.nearest_owner_start_by_token[current_index]);
        let Some(candidate_owner) = candidate_owner else {
            continue;
        };

        let candidate_owner = normalize_formatter_trivia_target_owner(tree, candidate_owner);
        if tree.get_node_type(candidate_owner) == NodeType::Declaration {
            return Some(candidate_owner);
        }
    }

    None
}

/// Find the next member owner at or after one semantic token index.
pub(super) fn find_next_member_owner_from_token(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: usize,
) -> Option<u32> {
    let token_count = owner_index.owner_start_by_token.len();
    if token_index >= token_count {
        return None;
    }

    let search_end = (token_index + 96).min(token_count);
    for current_index in token_index..search_end {
        let candidate_owner = owner_index.owner_start_by_token[current_index]
            .or(owner_index.nearest_owner_start_by_token[current_index]);
        let Some(candidate_owner) = candidate_owner else {
            continue;
        };

        if tree.get_node_type(candidate_owner) == NodeType::Member {
            return Some(candidate_owner);
        }
    }

    None
}

/// Return one lowest common ancestor for two owners.
pub(super) fn lowest_common_owner_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    left_owner: u32,
    right_owner: u32,
) -> Option<u32> {
    let mut left_chain = SmallVec::<[u32; 24]>::new();
    let mut current_left = Some(left_owner);
    while let Some(owner_id) = current_left {
        left_chain.push(owner_id);
        current_left = parents.get_by_id(owner_id);
    }

    let mut current_right = Some(right_owner);
    while let Some(owner_id) = current_right {
        if left_chain.contains(&owner_id) && !is_trivia_excluded_owner_node_id(tree, owner_id) {
            return Some(owner_id);
        }
        current_right = parents.get_by_id(owner_id);
    }

    None
}
