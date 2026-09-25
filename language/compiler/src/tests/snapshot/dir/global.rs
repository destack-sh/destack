use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GlobalTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (key, entries) in self.entries() {
            for entry in entries {
                match &entry.binding {
                    dir::ExportBinding::Local { symbols } => {
                        let symbol = symbols
                            .first()
                            .copied()
                            .expect("local global has no declaration");
                        let anchor = entry.item.map_or_else(
                            || builder.anchor_symbol(symbol.into_global(self.module_id)),
                            |item| builder.anchor_node(item.into_global_any(self.module_id)),
                        );
                        let row = SnapshotRow::new(anchor, "global", "local")
                            .field("key", builder.static_key(*key))
                            .list_field(
                                "symbols",
                                symbols
                                    .iter()
                                    .map(|symbol| builder.local_symbol_label(*symbol)),
                            )
                            .optional_field(
                                "declaration",
                                entry
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
                        let item = entry.item.expect("import global has no dependency item");
                        let node_id = item.into_global(self.module_id).into_any();
                        let row =
                            SnapshotRow::new(builder.anchor_node(node_id), "global", "import")
                                .field("key", builder.static_key(*key))
                                .field("imported", builder.export_selector_label(*selector))
                                .field("local", builder.local_symbol_label(*local))
                                .optional_field(
                                    "declaration",
                                    entry
                                        .declaration
                                        .map(|declaration| builder.local_symbol_label(declaration)),
                                );
                        let (target_key, target_value) = builder.module_target_field(*module);
                        let row = row.field(target_key, target_value);
                        builder.push(row);
                    }

                    dir::ExportBinding::ReExport { module, selector } => {
                        let item = entry.item.expect("re-export global has no dependency item");
                        let node_id = item.into_global(self.module_id).into_any();
                        let row =
                            SnapshotRow::new(builder.anchor_node(node_id), "global", "reexport")
                                .field("key", builder.static_key(*key))
                                .field("imported", builder.export_selector_label(*selector))
                                .optional_field(
                                    "declaration",
                                    entry
                                        .declaration
                                        .map(|declaration| builder.local_symbol_label(declaration)),
                                );
                        let (target_key, target_value) = builder.module_target_field(*module);
                        let row = row.field(target_key, target_value);
                        builder.push(row);
                    }
                }
            }
        }

        if self.is_empty() {
            return;
        }

        let entry_count = self.entries_by_key.values().map(Vec::len).sum::<usize>();
        let row = SnapshotRow::new(SnapshotAnchor::End, "global", "summary")
            .count_field("keys", self.entries_by_key.len())
            .count_field("entries", entry_count);
        builder.push(row);
    }
}
