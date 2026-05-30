use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::TypeSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render effective checked node types
        for (node_id, type_id) in self.node_types() {
            if !builder.should_render_type_node(node_id) {
                continue;
            }

            let row = SnapshotRow::new(builder.anchor_node(node_id), "type", "node")
                .optional_field("source", builder.node_source(node_id))
                .type_field("type", builder.type_label(type_id));
            builder.push(row);
        }

        // render solved symbol types
        for (symbol_id, type_id) in self.symbol_types() {
            let row = SnapshotRow::new(builder.anchor_symbol(symbol_id), "type", "symbol")
                .field("symbol", builder.symbol_path_label(symbol_id))
                .optional_field("source", builder.symbol_source(symbol_id))
                .type_field("type", builder.type_label(type_id));
            builder.push(row);
        }

        // summarize type table coverage
        let row = SnapshotRow::new(SnapshotAnchor::End, "type", "summary")
            .field("types", self.type_count().to_string())
            .count_field("nodes", self.node_types().count())
            .count_field("symbols", self.symbol_types().count());
        builder.push(row);
    }
}
