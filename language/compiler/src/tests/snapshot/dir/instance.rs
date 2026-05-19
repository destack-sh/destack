use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::InstanceSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node_id, instantiation_id) in self.node_instantiations() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "instance", "node")
                .field("node", builder.node_label(node_id))
                .field("instance", value::instantiation_label(instantiation_id));

            builder.push(row);
        }

        for (instantiation_id, instantiation) in self.iter_instantiations() {
            let row = SnapshotRow::new(SnapshotAnchor::End, "instance", "entry")
                .field("instance", value::instantiation_label(instantiation_id))
                .field("symbol", value::symbol_label(builder, instantiation.symbol))
                .list_field(
                    "arguments",
                    instantiation
                        .arguments
                        .iter()
                        .map(|argument| value::static_argument_label(builder, argument)),
                );

            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "instance", "summary")
            .field("instances", self.instantiation_count().to_string())
            .field("nodes", self.node_instantiation_count().to_string());
        builder.push(row);
    }
}
