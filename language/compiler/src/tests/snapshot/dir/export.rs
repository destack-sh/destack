use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, format};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ExportTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        if let Some(node_id) = self.export_assignment {
            let node_id = node_id.into_global(builder.tree.module_id).into_any();
            let row = SnapshotRow::new(builder.anchor_node(node_id), "export", "assignment")
                .field("node", builder.node_label(node_id));
            builder.push(row);
        }

        for namespace_export in &self.namespace_exports {
            let row = SnapshotRow::new(SnapshotAnchor::End, "export", "namespace")
                .field("value", format!("{namespace_export:?}"));
            builder.push(row);
        }

        for ((space, key), export) in &self.export_by_key {
            let anchor = builder.anchor_export(export);
            let row = SnapshotRow::new(anchor, "export", "entry")
                .field("space", format::debug(*space))
                .field("key", builder.static_key(*key))
                .field("target", builder.symbol_label(export.target));
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "export", "summary")
            .field("entries", self.export_by_key.len().to_string())
            .field("namespaces", self.namespace_exports.len().to_string())
            .field(
                "assignment",
                if self.export_assignment.is_some() {
                    "yes"
                } else {
                    "no"
                },
            );
        builder.push(row);
    }
}
