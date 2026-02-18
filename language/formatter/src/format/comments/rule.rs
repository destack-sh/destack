use ast::{NodeParentIndex, NodeTree, NodeType};
use destack_ast as ast;
use destack_source::Span;

use super::owner::{
    normalize_formatter_trivia_target_owner, promote_owner_by_shared_end,
    promote_owner_by_shared_start, promote_owner_to_node_type_ancestor,
};

/// Normalize one owner and optionally promote across shared seam end.
pub(super) fn normalize_owner_with_shared_end(
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
pub(super) fn promote_rhs_expression_owner(
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
