use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ExportTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for export in self.export_by_key.values() {
            match &export.binding {
                dir::ExportBinding::Local { symbols } => {
                    let anchor = if let Some(item) = export.item {
                        let node_id = item.into_global(builder.tree.module_id).into_any();
                        builder.anchor_node(node_id)
                    } else {
                        let symbol = symbols
                            .first()
                            .copied()
                            .expect("local export has no declaration");
                        builder.anchor_symbol(symbol.into_global(builder.tree.module_id))
                    };
                    let row = SnapshotRow::new(anchor, "export", "local")
                        .field("key", builder.export_key_label(export.key))
                        .list_field(
                            "symbols",
                            symbols
                                .iter()
                                .map(|symbol| builder.local_symbol_label(*symbol)),
                        )
                        .optional_field(
                            "declaration",
                            export
                                .declaration
                                .map(|declaration| builder.local_symbol_label(declaration)),
                        );
                    builder.push(row);
                }
                dir::ExportBinding::Import {
                    local,
                    module,
                    selector,
                } => {
                    let item = export.item.expect("import export has no dependency item");
                    let node_id = item.into_global(builder.tree.module_id).into_any();
                    let row = SnapshotRow::new(builder.anchor_node(node_id), "export", "import")
                        .field("key", builder.export_key_label(export.key))
                        .field("imported", builder.export_selector_label(*selector))
                        .field("local", builder.local_symbol_label(*local))
                        .optional_field(
                            "declaration",
                            export
                                .declaration
                                .map(|declaration| builder.local_symbol_label(declaration)),
                        );
                    let (target_key, target_value) = builder.module_target_field(*module);
                    let row = row.field(target_key, target_value);
                    builder.push(row);
                }
                dir::ExportBinding::ReExport { module, selector } => {
                    let item = export.item.expect("re-export has no dependency item");
                    let node_id = item.into_global(builder.tree.module_id).into_any();
                    let row = SnapshotRow::new(builder.anchor_node(node_id), "export", "reexport")
                        .field("key", builder.export_key_label(export.key))
                        .field("imported", builder.export_selector_label(*selector))
                        .optional_field(
                            "declaration",
                            export
                                .declaration
                                .map(|declaration| builder.local_symbol_label(declaration)),
                        );
                    let (target_key, target_value) = builder.module_target_field(*module);
                    let row = row.field(target_key, target_value);
                    builder.push(row);
                }
            }
        }

        for export in &self.star_exports {
            let node_id = export.item.into_global(builder.tree.module_id).into_any();
            let row = SnapshotRow::new(builder.anchor_node(node_id), "export", "star");
            let (target_key, target_value) = builder.module_target_field(export.target);
            let row = row.field(target_key, target_value);
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "export", "summary")
            .count_field("exports", self.export_by_key.len())
            .count_field("stars", self.star_exports.len());
        builder.push(row);
    }
}
