use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, label};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::InstanceSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // node instances
        for (node_id, instance_id) in self.node_instances() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "instance", "node")
                .optional_field("source", builder.node_source(node_id))
                .field("instance", label::instance_label(instance_id));

            builder.push(row);
        }

        // instances
        for (instance_id, instance) in self.iter_instances() {
            let row = SnapshotRow::new(SnapshotAnchor::End, "instance", "entry")
                .field("instance", label::instance_label(instance_id))
                .field("key", builder.symbol_path_label(instance.symbol))
                .list_field(
                    "arguments",
                    instance
                        .arguments
                        .iter()
                        .map(|argument| label::static_argument_label(builder, argument)),
                );

            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "instance", "summary")
            .field("instances", self.instance_count().to_string())
            .field("nodes", self.node_instance_count().to_string());
        builder.push(row);
    }
}
