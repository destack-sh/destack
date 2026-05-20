use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::CaptureSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        let mut binding_count = 0usize;
        let mut directive_count = 0usize;
        let mut rule_count = 0usize;

        for (symbol_id, capture) in &self.capture_by_function {
            let anchor = builder.anchor_symbol(*symbol_id);
            let row = SnapshotRow::new(anchor, "capture", "function")
                .field("function", builder.symbol_path_label(*symbol_id))
                .field("bindings", capture.captures.len().to_string())
                .optional_field(
                    "this",
                    capture
                        .this
                        .map(|binding| builder.capture_binding_label(binding)),
                );
            builder.push(row);

            for binding in &capture.captures {
                let row = SnapshotRow::new(anchor, "capture", "binding")
                    .field("function", builder.symbol_path_label(*symbol_id))
                    .field("symbol", builder.symbol_label(binding.symbol))
                    .field("mode", DirSnapshotBuilder::variant_label(binding.mode));
                builder.push(row);
            }
            binding_count += capture.captures.len();

            // render directive state as explicit rows
            if let Some(directive) = &capture.directive {
                let row = SnapshotRow::new(anchor, "capture", "directive")
                    .field("function", builder.symbol_path_label(*symbol_id))
                    .field(
                        "default",
                        DirSnapshotBuilder::variant_label(directive.default),
                    )
                    .field("rules", directive.rules.len().to_string());
                builder.push(row);
                directive_count += 1;

                // render per binding capture overrides
                for rule in &directive.rules {
                    let row = SnapshotRow::new(anchor, "capture", "rule")
                        .field("function", builder.symbol_path_label(*symbol_id))
                        .field("binding", builder.strings.get(rule.name))
                        .field("mode", DirSnapshotBuilder::variant_label(rule.mode));
                    builder.push(row);
                }
                rule_count += directive.rules.len();
            }
        }

        let function_count = self.capture_by_function.len();
        if function_count == 0 && binding_count == 0 && directive_count == 0 && rule_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "capture", "summary")
            .count_field("functions", function_count)
            .count_field("bindings", binding_count)
            .count_field("directives", directive_count)
            .count_field("rules", rule_count);
        builder.push(row);
    }
}
