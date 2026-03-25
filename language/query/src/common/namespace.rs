use destack_dir as dir;
use destack_dir::{Expression, GlobalSymbolId};
use destack_source::{NodeSpanType, SourcePartKey};

use super::{QueryContext, resolve_expression_symbol};

/// Resolve the namespace receiver symbol for a member access.
pub(crate) fn resolve_namespace_receiver_symbol(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    match expression {
        Expression::Parenthesized { expression } => {
            resolve_namespace_receiver_symbol(ctx, *expression)
        }
        _ => resolve_expression_symbol(ctx, expression_id),
    }
}

/// Resolve one plain path segment symbol for a qualified path expression.
pub(crate) fn resolve_path_segment_symbol(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    segment_index: u16,
) -> Option<GlobalSymbolId> {
    let source_id = ctx.tree().get_source(expression_id.id);
    ctx.types()
        .get_symbol_target_for_source_part(SourcePartKey::new(
            source_id,
            NodeSpanType::Segment(segment_index),
        ))
}
