use super::super::{DestackFormatContext, Expression, LocalNodeId, NodeType, TypeBinaryOperator};

/// Return whether a chain should break because its cast or satisfies parent overflows.
pub(super) fn chain_overflows_in_type_binary_left(
    context: &DestackFormatContext<'_>,
    chain_tail: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(chain_tail) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::TypeBinary { left, operator, .. } = context.tree.get(parent_id) else {
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

    if context.node_has_newline(parent_id) {
        return true;
    }

    let Some((grandparent_id, grandparent_type)) = context.parent(parent_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_id = LocalNodeId::<Expression>::new(grandparent_id);
    let Expression::Parenthesized { expression } = context.tree.get(grandparent_id) else {
        return false;
    };
    if *expression != parent_id {
        return false;
    }

    let Some((great_grandparent_id, great_grandparent_type)) = context.parent(grandparent_id)
    else {
        return false;
    };
    if great_grandparent_type != NodeType::Expression {
        return false;
    }

    let great_grandparent_id = LocalNodeId::<Expression>::new(great_grandparent_id);
    let Expression::New { left, .. } = context.tree.get(great_grandparent_id) else {
        return false;
    };
    if *left != grandparent_id {
        return false;
    }

    context.node_has_newline(great_grandparent_id)
}
