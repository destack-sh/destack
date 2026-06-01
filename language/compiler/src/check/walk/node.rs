use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{TypeOperand, TypeTerm, WalkState};

impl WalkState<'_, '_> {
    /// Bind one checked type term to a node.
    pub(in crate::check) fn bind_node_type<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
    ) {
        self.check.bind_node_type(module, id, term);
    }

    /// Bind one checked type operand to a node.
    pub(in crate::check) fn bind_node_type_operand<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        operand: TypeOperand,
    ) {
        self.check.bind_node_type_operand(module, id, operand);
    }
}
