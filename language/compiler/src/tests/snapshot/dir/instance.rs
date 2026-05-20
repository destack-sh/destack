use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::InstanceSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render instance application sites
        for (node_id, instance_id) in self.node_instances() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "instance", "application")
                .optional_field("source", builder.node_source(node_id))
                .field("id", builder.instance_label(instance_id));

            builder.push(row);
        }

        // instances
        for (instance_id, instance) in self.iter_instances() {
            let row = SnapshotRow::new(SnapshotAnchor::End, "instance", "entry")
                .field("id", builder.instance_label(instance_id))
                .field("symbol", builder.symbol_path_label(instance.symbol))
                .list_field(
                    "arguments",
                    instance
                        .arguments
                        .iter()
                        .map(|argument| builder.static_argument_label(argument)),
                );

            builder.push(row);
        }

        let instance_count = self.instance_count();
        let application_count = self.node_instance_count();
        if instance_count == 0 && application_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "instance", "summary")
            .count_field("instances", instance_count)
            .count_field("applications", application_count);
        builder.push(row);
    }
}
