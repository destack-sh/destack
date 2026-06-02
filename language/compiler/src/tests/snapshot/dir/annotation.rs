use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::AnnotationSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render annotation invocations in source order
        for (_, invocation) in self.iter_invocations() {
            let arguments = invocation
                .arguments
                .iter()
                .map(|argument| builder.annotation_argument_label(argument));
            let row =
                SnapshotRow::new(builder.anchor_node(invocation.source), "annotation", "node")
                    .optional_field("source", builder.node_source(invocation.source))
                    .optional_field("owner", builder.node_source(invocation.owner))
                    .optional_field("callee", builder.node_source(invocation.callee))
                    .field("target", builder.annotation_target_label(invocation.target))
                    .optional_list_field("arguments", arguments);

            builder.push(row);
        }

        // summarize visible annotations
        let count = self.invocation_count() as usize;
        if count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "annotation", "summary")
            .count_field("nodes", count);
        builder.push(row);
    }
}

impl DirSnapshotBuilder<'_> {
    /// Return the snapshot label for one annotation argument.
    pub(crate) fn annotation_argument_label(&self, argument: &dir::AnnotationArgument) -> String {
        let source = self
            .node_source(argument.source)
            .unwrap_or_else(|| "?".to_string());

        match argument.value {
            Some(value) => {
                let value = self.global_static_label(value);

                format!("{source} = {value}")
            }
            None => source,
        }
    }

    /// Return the snapshot label for one annotation target.
    pub(crate) fn annotation_target_label(&self, target: dir::AnnotationTarget) -> String {
        match target {
            dir::AnnotationTarget::LanguageItem(item) => item.key(),
            dir::AnnotationTarget::Symbol(symbol) => self.symbol_path_label(symbol),
            dir::AnnotationTarget::Unknown => "unknown".to_string(),
        }
    }
}
