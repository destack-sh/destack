use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::CaptureSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (symbol_id, capture) in &self.capture_by_function {
            let anchor = builder.anchor_symbol(*symbol_id);
            let row = SnapshotRow::new(anchor, "capture", "function")
                .field("name", builder.symbol_label(*symbol_id))
                .field("bindings", capture.captures.len().to_string())
                .optional_field(
                    "this",
                    value::optional_capture_binding_label(builder, capture.this),
                );
            builder.push(row);

            // render directive state as explicit rows
            if let Some(directive) = &capture.directive {
                let row = SnapshotRow::new(anchor, "capture", "directive")
                    .field("name", builder.symbol_label(*symbol_id))
                    .field("default", value::debug_label(directive.default))
                    .field("rules", directive.rules.len().to_string());
                builder.push(row);

                // render per binding capture overrides
                for rule in &directive.rules {
                    let row = SnapshotRow::new(anchor, "capture", "rule")
                        .field("name", builder.symbol_label(*symbol_id))
                        .field("binding", builder.strings.get(rule.name))
                        .field("mode", value::debug_label(rule.mode));
                    builder.push(row);
                }
            }
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "capture", "summary")
            .field("functions", self.capture_by_function.len().to_string());
        builder.push(row);
    }
}
