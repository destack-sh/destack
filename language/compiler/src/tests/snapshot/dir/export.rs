use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, label};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ExportTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for export in self.export_by_key.values() {
            match export {
                dir::ExportEntry::Local(export) => {
                    let symbol_id = export.source.into_global(builder.tree.module_id);
                    let anchor = if let Some(item) = export.item {
                        let node_id = item.into_global(builder.tree.module_id).into_any();
                        builder.anchor_node(node_id)
                    } else {
                        builder.anchor_symbol(symbol_id)
                    };
                    let row = SnapshotRow::new(anchor, "export", "local")
                        .field("key", label::export_key_label(builder, export.key))
                        .field("source", builder.local_symbol_label(export.source));
                    builder.push(row);
                }
                dir::ExportEntry::Indirect(export) => {
                    let node_id = export.item.into_global(builder.tree.module_id).into_any();
                    let row = SnapshotRow::new(builder.anchor_node(node_id), "export", "indirect")
                        .field("key", label::export_key_label(builder, export.key))
                        .field(
                            "import",
                            label::export_selector_label(builder, export.imported),
                        );
                    let (target_key, target_value) =
                        label::dependency_target_field(builder, export.target);
                    let row = row.field(target_key, target_value);
                    builder.push(row);
                }
            }
        }

        for export in &self.star_exports {
            let node_id = export.item.into_global(builder.tree.module_id).into_any();
            let row = SnapshotRow::new(builder.anchor_node(node_id), "export", "star");
            let (target_key, target_value) = label::dependency_target_field(builder, export.target);
            let row = row.field(target_key, target_value);
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "export", "summary")
            .field("exports", self.export_by_key.len().to_string())
            .field("stars", self.star_exports.len().to_string());
        builder.push(row);
    }
}
