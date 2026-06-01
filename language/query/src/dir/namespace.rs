use destack_dir as dir;

use crate::core::DirQueryContext;

impl DirQueryContext<'_> {
    /// Return the recorded namespace receiver symbol for a member access.
    pub(crate) fn namespace_receiver_symbol_target(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<dir::GlobalSymbolId> {
        let dir_tree = self.view();
        let expression = dir_tree.get::<dir::Expression>(expression_id);

        match expression {
            dir::Expression::Parenthesized { expression } => {
                self.namespace_receiver_symbol_target(*expression)
            }
            _ => self.expression_symbol_target(expression_id),
        }
    }

    /// Return the recorded symbol target for one plain path segment.
    pub(crate) fn path_segment_symbol_target(
        self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        segment_index: u16,
    ) -> Option<dir::GlobalSymbolId> {
        if segment_index == 0 {
            return self.expression_symbol_target(expression_id);
        }

        None
    }
}
