use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{TypeOperand, TypeTerm, WalkState};

impl WalkState<'_, '_> {
    /// Output one node checked type under the active guard.
    pub(in crate::check) fn output_node_type<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        term: TypeTerm,
    ) {
        let condition = self.active_static_guard();

        self.check
            .output_node_type_guarded(module, id, term, condition);
    }

    /// Output one node checked type operand under the active guard.
    pub(in crate::check) fn output_node_type_operand<T: dir::Node + Clone>(
        &mut self,
        module: ModuleId,
        id: dir::LocalNodeId<T>,
        operand: TypeOperand,
    ) {
        let condition = self.active_static_guard();

        self.check
            .output_node_type_operand_guarded(module, id, operand, condition);
    }
}
