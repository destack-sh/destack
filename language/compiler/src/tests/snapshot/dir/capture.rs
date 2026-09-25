use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::CaptureSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        let mut frame_field_count = 0usize;
        let mut binding_count = 0usize;
        let mut directive_count = 0usize;
        let mut rule_count = 0usize;

        // render shared frames before the closures that reference them
        for (frame_id, frame) in self.iter_frames() {
            let anchor = self
                .capture_by_function
                .iter()
                .find(|(_, capture)| capture.frames.contains(&frame_id))
                .map(|(symbol_id, _)| builder.anchor_symbol(*symbol_id))
                .unwrap_or(SnapshotAnchor::End);

            let fields = frame.fields.iter().map(|field| {
                let symbol = builder.symbol_label(field.symbol);
                let ty = builder.global_type_label(field.ty);

                format!("{symbol}: {ty}")
            });
            let fields = fields.collect::<Vec<_>>().join(", ");
            let fields = if fields.is_empty() {
                "{}".to_string()
            } else {
                format!("{{ {fields} }}")
            };

            let row = SnapshotRow::new(anchor, "capture", "frame")
                .field("frame", builder.capture_frame_label(frame_id))
                .field("scope", builder.global_scope_label(frame.scope))
                .type_field("type", builder.global_type_label(frame.ty))
                .object_field("fields", fields);
            builder.push(row);
            frame_field_count += frame.fields.len();
        }

        for (symbol_id, capture) in &self.capture_by_function {
            let anchor = builder.anchor_symbol(*symbol_id);
            let frames = capture
                .frames
                .iter()
                .map(|frame_id| builder.capture_frame_label(*frame_id));
            let row = SnapshotRow::new(anchor, "capture", "function")
                .field("function", builder.symbol_path_label(*symbol_id))
                .field("bindings", capture.captures.len().to_string())
                .optional_tuple_field("frames", frames);
            builder.push(row);

            // render explicit binding captures
            for binding in &capture.captures {
                let row = SnapshotRow::new(anchor, "capture", "binding")
                    .field("function", builder.symbol_path_label(*symbol_id))
                    .field("symbol", builder.symbol_label(binding.symbol()))
                    .field("mode", DirSnapshotBuilder::variant_label(binding.mode()))
                    .type_field("type", builder.global_type_label(binding.ty()))
                    .optional_field(
                        "frame",
                        binding
                            .frame()
                            .map(|frame| builder.capture_frame_label(frame)),
                    );
                builder.push(row);
            }
            binding_count += capture.captures.len();

            // render lexical receiver capture separately
            if let Some(receiver) = capture.this {
                let row = SnapshotRow::new(anchor, "capture", "receiver")
                    .field("function", builder.symbol_path_label(*symbol_id))
                    .field("symbol", builder.symbol_label(receiver.symbol))
                    .field("mode", DirSnapshotBuilder::variant_label(receiver.mode))
                    .type_field("type", builder.global_type_label(receiver.ty));
                builder.push(row);
            }

            // render directive state as explicit rows
            if let Some(directive) = self.capture_directive(*symbol_id) {
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
        let frame_count = self.frame_count();
        if frame_count == 0
            && frame_field_count == 0
            && function_count == 0
            && binding_count == 0
            && directive_count == 0
            && rule_count == 0
        {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "capture", "summary")
            .count_field("frames", frame_count)
            .count_field("fields", frame_field_count)
            .count_field("functions", function_count)
            .count_field("bindings", binding_count)
            .count_field("directives", directive_count)
            .count_field("rules", rule_count);
        builder.push(row);
    }
}
