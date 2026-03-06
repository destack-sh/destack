use ast::{
    AnnotationPosition, Block, Expression, LocalNodeId, NodeParentIndex, NodeTree, NodeType,
};
use destack_ast as ast;
use destack_source::{EnclosingSpan, Span};
use smallvec::SmallVec;

use super::attachment::FormatterTriviaOwnerIndex;

/// Return whether one node kind is excluded from trivia owner indexing.
pub(crate) fn is_trivia_excluded_owner_node_id(tree: &NodeTree, node_id: u32) -> bool {
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
pub(crate) fn normalize_formatter_trivia_target_owner(tree: &NodeTree, owner_id: u32) -> u32 {
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

/// Return one resolved owner in one ancestor chain.
pub(crate) fn find_owner_in_ancestor_chain(
    parents: &NodeParentIndex,
    start_owner: u32,
    mut resolve_owner: impl FnMut(u32) -> Option<u32>,
) -> Option<u32> {
    let mut current_owner = Some(start_owner);
    while let Some(owner_id) = current_owner {
        if let Some(resolved_owner) = resolve_owner(owner_id) {
            return Some(resolved_owner);
        }

        current_owner = parents.get_by_id(owner_id);
    }

    None
}

/// Return one resolved owner across one ordered candidate-owner list.
pub(crate) fn find_owner_in_candidate_ancestry<const N: usize>(
    parents: &NodeParentIndex,
    candidate_owners: [Option<u32>; N],
    mut resolve_owner: impl FnMut(u32) -> Option<u32>,
) -> Option<u32> {
    candidate_owners
        .into_iter()
        .flatten()
        .find_map(|owner_id| find_owner_in_ancestor_chain(parents, owner_id, &mut resolve_owner))
}

/// Return whether one owner is one block node or block expression wrapper.
pub(crate) fn is_block_like_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) == NodeType::Block {
        return true;
    }

    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    matches!(
        tree.get(LocalNodeId::<Expression>::new(owner_id)),
        Expression::Block(_)
    )
}

/// Return one preferred owner that starts at one token span.
pub(crate) fn find_preferred_owner_starting_at(tree: &NodeTree, span: Span) -> Option<u32> {
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

            let should_replace = best_owner.is_none_or(|current| {
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
            });

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Return one smallest owner that encloses one token span.
pub(crate) fn find_smallest_owner_enclosing_token(tree: &NodeTree, span: Span) -> Option<u32> {
    let mut best_owner: Option<EnclosingSpan> = None;
    tree.source_map
        .visit_enclosing_spans(span.start, span.end.saturating_sub(1), |candidate| {
            if is_trivia_excluded_owner_node_id(tree, candidate.idx) {
                return;
            }

            let should_replace = best_owner.is_none_or(|current| {
                candidate.length < current.length
                    || (candidate.length == current.length && candidate.idx < current.idx)
            });

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Return one smallest owner that encloses one seam range.
pub(crate) fn find_smallest_owner_enclosing_range(
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

            let should_replace = best_owner.is_none_or(|current| {
                candidate.length < current.length
                    || (candidate.length == current.length && candidate.idx < current.idx)
            });

            if should_replace {
                best_owner = Some(candidate);
            }
        });

    best_owner.map(|owner| owner.idx)
}

/// Find one owner at or after one semantic token index.
pub(crate) fn find_owner_at_or_after_token(
    tree: &NodeTree,
    semantic_tokens: &[ast::TokenSpan],
    token_index: usize,
) -> Option<u32> {
    semantic_tokens
        .iter()
        .skip(token_index)
        .find_map(|token| find_smallest_owner_enclosing_token(tree, token.span))
}

/// Find one owner of one node type at or after one semantic token index.
pub(crate) fn find_owner_at_or_after_token_with_node_type(
    tree: &NodeTree,
    owner_index: &FormatterTriviaOwnerIndex,
    token_index: usize,
    node_type: NodeType,
) -> Option<u32> {
    let token_count = owner_index.owner_start_by_token.len();
    if token_index >= token_count {
        return None;
    }

    for current_index in token_index..token_count {
        let candidate_owner = owner_index.owner_start_by_token[current_index]
            .or(owner_index.nearest_owner_start_by_token[current_index]);
        let Some(candidate_owner) = candidate_owner else {
            continue;
        };

        let candidate_owner = normalize_formatter_trivia_target_owner(tree, candidate_owner);
        if tree.get_node_type(candidate_owner) == node_type {
            return Some(candidate_owner);
        }
    }

    None
}

/// Promote one owner while ancestor spans share the same seam end.
pub(crate) fn promote_owner_by_shared_end(
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
pub(crate) fn promote_owner_by_shared_start(
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

/// Normalize one owner and optionally promote across shared seam end.
pub(crate) fn normalize_owner_with_shared_end(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    token_before_span: Option<Span>,
) -> u32 {
    let target_node = normalize_formatter_trivia_target_owner(tree, owner_id);
    token_before_span.map_or(target_node, |span| {
        promote_owner_by_shared_end(tree, parents, target_node, span.end)
    })
}

/// Promote one owner across shared seam start and normalize to expression when available.
pub(crate) fn promote_rhs_expression_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    token_after_span: Option<Span>,
) -> u32 {
    let mut target_node = token_after_span.map_or(owner_id, |span| {
        promote_owner_by_shared_start(tree, parents, owner_id, span.start)
    });

    if tree.get_node_type(target_node) != NodeType::Expression
        && let Some(expression_target) =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Expression)
    {
        target_node = expression_target;
    }

    target_node
}

/// Return one block-interior placement target for boundary comments.
pub(crate) fn block_leading_comment_target(
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
pub(crate) fn promote_owner_to_declaration_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        (tree.get_node_type(node_id) == NodeType::Declaration).then_some(node_id)
    })
}

/// Promote one owner to the nearest ancestor of one node type.
pub(crate) fn promote_owner_to_node_type_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
    node_type: NodeType,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        (tree.get_node_type(node_id) == node_type).then_some(node_id)
    })
}

/// Promote one owner to one statement boundary owner.
pub(crate) fn promote_owner_to_statement_boundary(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> u32 {
    if let Some(member_owner) =
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Member)
    {
        return member_owner;
    }

    let mut statement_expression_owner = None;
    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression
            && tree
                .get(LocalNodeId::<Expression>::new(node_id))
                .is_top_level_statement()
        {
            statement_expression_owner = Some(node_id);
        }

        current_id = parents.get_by_id(node_id);
    }
    if let Some(statement_expression_owner) = statement_expression_owner {
        return statement_expression_owner;
    }

    if let Some(declaration_owner) = promote_owner_to_declaration_ancestor(tree, parents, owner_id)
    {
        return declaration_owner;
    }

    if let Some(expression_owner) =
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
    {
        return expression_owner;
    }

    owner_id
}

/// Promote one owner to the nearest enclosing statement boundary owner.
pub(crate) fn promote_owner_to_nearest_statement_boundary(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> u32 {
    if let Some(member_owner) =
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Member)
    {
        return member_owner;
    }

    let mut current_id = Some(owner_id);
    while let Some(node_id) = current_id {
        if tree.get_node_type(node_id) == NodeType::Expression
            && tree
                .get(LocalNodeId::<Expression>::new(node_id))
                .is_top_level_statement()
        {
            return node_id;
        }

        current_id = parents.get_by_id(node_id);
    }

    if let Some(declaration_owner) = promote_owner_to_declaration_ancestor(tree, parents, owner_id)
    {
        return declaration_owner;
    }

    if let Some(expression_owner) =
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
    {
        return expression_owner;
    }

    owner_id
}

/// Promote one owner to the nearest `satisfies` expression ancestor.
pub(crate) fn promote_owner_to_satisfies_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
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

        None
    })
}

/// Promote one owner to the nearest parenthesized expression ancestor.
pub(crate) fn promote_owner_to_parenthesized_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if matches!(tree.get(expression_id), Expression::Parenthesized { .. }) {
                return Some(node_id);
            }
        }

        None
    })
}

/// Return one lowest common ancestor for two owners.
pub(crate) fn lowest_common_owner_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: u32,
    following_owner: u32,
) -> Option<u32> {
    let mut preceding_chain = SmallVec::<[u32; 24]>::new();
    let mut current_preceding = Some(preceding_owner);
    while let Some(owner_id) = current_preceding {
        preceding_chain.push(owner_id);
        current_preceding = parents.get_by_id(owner_id);
    }

    let mut current_following = Some(following_owner);
    while let Some(owner_id) = current_following {
        if preceding_chain.contains(&owner_id) && !is_trivia_excluded_owner_node_id(tree, owner_id)
        {
            return Some(owner_id);
        }
        current_following = parents.get_by_id(owner_id);
    }

    None
}
