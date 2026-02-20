use super::{DestackFormatContext, Expression, LocalNodeId, is_trivial_expression};

/// Return whether an expression is trivial and inline-safe without annotations.
pub(super) fn expression_is_trivial_inline_without_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    !context.has_annotation(expression_id)
        && !context.node_has_newline(expression_id)
        && is_trivial_expression(context.tree, context.tree.get(expression_id))
}
