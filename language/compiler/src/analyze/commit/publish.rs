use crate::Compiler;
use destack_dir::{InferTable, TypeTable};

impl Compiler {
    /// Commit infer-owned expression overlays into canonical type ctx.
    pub(super) fn commit_infer_expression_overlays(
        &self,
        infer: &InferTable,
        types: &mut TypeTable,
    ) {
        // commit inferred expression types in deterministic node-id order
        let mut inferred_type_entries = infer.iter_inferred_type_nodes().collect::<Vec<_>>();
        inferred_type_entries.sort_by_key(|(node_id, _)| *node_id);
        for (node_id, type_id) in inferred_type_entries {
            types.set_inferred_type(node_id, type_id);
        }

        // commit expression addressability in deterministic node-id order
        let mut addressability_entries = infer.iter_addressability_nodes().collect::<Vec<_>>();
        addressability_entries.sort_by_key(|(node_id, _)| *node_id);
        for (node_id, addressability) in addressability_entries {
            types.set_addressability_for_node(node_id, addressability);
        }
    }
}
