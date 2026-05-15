use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::CaptureTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (symbol_id, capture) in &self.capture_by_function {
            let row = SnapshotRow::new(builder.anchor_symbol(*symbol_id), "capture", "function")
                .field("name", builder.symbol_label(*symbol_id))
                .field("bindings", capture.captures.len().to_string())
                .optional_field(
                    "this",
                    value::optional_capture_binding(builder, capture.this),
                )
                .optional_field(
                    "directive",
                    capture.directive.as_ref().map(value::capture_directive),
                );
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "capture", "summary")
            .field("functions", self.capture_by_function.len().to_string());
        builder.push(row);
    }
}
