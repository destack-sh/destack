use destack_dir as dir;
use destack_dir::{Expression, GlobalSymbolId};

use super::resolve_expression_symbol;
use crate::core::DirQueryContext;

/// Resolve the namespace receiver symbol for a member access.
pub(crate) fn resolve_namespace_receiver_symbol(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = dir.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    match expression {
        Expression::Parenthesized { expression } => {
            resolve_namespace_receiver_symbol(dir, *expression)
        }
        _ => resolve_expression_symbol(dir, expression_id),
    }
}

/// Resolve one plain path segment symbol for a qualified path expression.
pub(crate) fn resolve_path_segment_symbol(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    segment_index: u16,
) -> Option<GlobalSymbolId> {
    if segment_index == 0 {
        return resolve_expression_symbol(dir, expression_id);
    }

    None
}
