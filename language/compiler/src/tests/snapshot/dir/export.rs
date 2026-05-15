use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ExportTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for export in self.export_by_name.values() {
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
                        .field("name", value::export_name(builder, export.name))
                        .field("source", builder.local_symbol_label(export.source));
                    builder.push(row);
                }
                dir::ExportEntry::Indirect(export) => {
                    let node_id = export.item.into_global(builder.tree.module_id).into_any();
                    let row = SnapshotRow::new(builder.anchor_node(node_id), "export", "indirect")
                        .field("name", value::export_name(builder, export.name))
                        .field("import", value::export_selector(builder, export.imported));
                    let row = value::add_dependency_target(row, builder, export.target);
                    builder.push(row);
                }
            }
        }

        for export in &self.star_exports {
            let node_id = export.item.into_global(builder.tree.module_id).into_any();
            let row = SnapshotRow::new(builder.anchor_node(node_id), "export", "star");
            let row = value::add_dependency_target(row, builder, export.target);
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "export", "summary")
            .field("exports", self.export_by_name.len().to_string())
            .field("stars", self.star_exports.len().to_string());
        builder.push(row);
    }
}
