use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GuardTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node_id, entry) in &self.entry_by_node {
            let row = SnapshotRow::new(builder.anchor_node(*node_id), "guard", "entry")
                .field("node", builder.node_label(*node_id))
                .field("kind", value::debug_label(*entry));
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "guard", "summary")
            .field("entries", self.entry_by_node.len().to_string());
        builder.push(row);
    }
}
