use destack_dir as dir;
use destack_dir::{Expression, GlobalSymbolId};

use super::expression_symbol_target;
use crate::core::DirQueryContext;

/// Return the recorded namespace receiver symbol for a member access.
pub(crate) fn namespace_receiver_symbol_target(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = dir.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    match expression {
        Expression::Parenthesized { expression } => {
            namespace_receiver_symbol_target(dir, *expression)
        }
        _ => expression_symbol_target(dir, expression_id),
    }
}

/// Return the recorded symbol target for one plain path segment.
pub(crate) fn path_segment_symbol_target(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    segment_index: u16,
) -> Option<GlobalSymbolId> {
    if segment_index == 0 {
        return expression_symbol_target(dir, expression_id);
    }

    None
}
