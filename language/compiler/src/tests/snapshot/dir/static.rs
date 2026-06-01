use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::StaticSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        let mut symbol_static_ids = Vec::new();

        // render static values attached to symbols
        for (symbol_id, static_id) in self.symbol_statics() {
            symbol_static_ids.push(static_id);

            let row = SnapshotRow::new(builder.anchor_symbol(symbol_id), "static", "symbol")
                .field("symbol", builder.symbol_path_label(symbol_id))
                .optional_field("source", builder.symbol_source(symbol_id))
                .field("value", builder.global_static_label(static_id));

            builder.push(row);
        }

        // render static values
        for static_id in self.iter_static_ids() {
            let global_static_id = static_id.into_global(self.module_id);
            if symbol_static_ids.contains(&global_static_id) {
                continue;
            }

            let term = self.get_static(static_id);
            let row = SnapshotRow::new(SnapshotAnchor::End, "static", "entry")
                .field("value", builder.static_term_label(term));

            builder.push(row);
        }

        // summarize static table coverage when present
        let static_count = self.static_count();
        if static_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "static", "summary")
            .field("statics", static_count.to_string());
        builder.push(row);
    }
}
