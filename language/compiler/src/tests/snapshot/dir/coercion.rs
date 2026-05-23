use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::CoercionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node_id, coercion) in self.coercions() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "coercion", "node")
                .optional_field("source", builder.node_source(node_id))
                .type_field("from", builder.type_label(coercion.source))
                .type_field("to", builder.type_label(coercion.target));

            builder.push(row);
        }

        let count = self.coercions().count();
        if count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "coercion", "summary")
            .count_field("nodes", count);
        builder.push(row);
    }
}
