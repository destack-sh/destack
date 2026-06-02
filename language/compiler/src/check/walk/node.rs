use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{TypeOperand, TypeTerm, WalkState};

impl WalkState<'_, '_> {
    /// Publish one checked type term for a node.
    pub(in crate::check) fn publish_node_type<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
    ) {
        self.check.publish_node_type(module, id, term);
    }

    /// Publish one checked type operand for a node.
    pub(in crate::check) fn publish_node_type_operand<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        operand: TypeOperand,
    ) {
        self.check.publish_node_type_operand(module, id, operand);
    }
}
