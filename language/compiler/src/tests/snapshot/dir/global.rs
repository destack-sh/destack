use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GlobalTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (key, entries) in self.entries() {
            for entry in entries {
                match entry {
                    dir::GlobalEntry::Local(entry) => {
                        let symbol = entry.source.into_global(self.module_id);
                        let row =
                            SnapshotRow::new(builder.anchor_symbol(symbol), "global", "local")
                                .field("key", builder.static_key(*key))
                                .field("source", builder.local_symbol_label(entry.source));
                        builder.push(row);
                    }

                    dir::GlobalEntry::Indirect(entry) => {
                        let node_id = entry.item.into_global(self.module_id).into_any();
                        let row =
                            SnapshotRow::new(builder.anchor_node(node_id), "global", "indirect")
                                .field("key", builder.static_key(*key))
                                .field("imported", builder.export_selector_label(entry.imported));
                        let (target_key, target_value) =
                            builder.dependency_target_field(entry.target);
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
