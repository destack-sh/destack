use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::DecoratorSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render decorator applications in source order
        for (_, application) in self.iter_applications() {
            let arguments = application
                .arguments
                .iter()
                .map(|argument| builder.decorator_argument_label(argument));
            let row =
                SnapshotRow::new(builder.anchor_node(application.source), "decorator", "node")
                    .optional_field("source", builder.node_source(application.source))
                    .optional_field("owner", builder.node_source(application.owner))
                    .optional_field("target", builder.node_source(application.target))
                    .field(
                        "resolution",
                        builder.decorator_target_label(application.resolution),
                    )
                    .optional_tuple_field("arguments", arguments);

            builder.push(row);
        }

        // summarize visible decorators
        let count = self.application_count() as usize;
        if count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "decorator", "summary")
            .count_field("nodes", count);
        builder.push(row);
    }
}

impl DirSnapshotBuilder<'_> {
    /// Return the snapshot label for one decorator argument.
    pub(crate) fn decorator_argument_label(&self, argument: &dir::DecoratorArgument) -> String {
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

    /// Return the snapshot label for one decorator target.
    pub(crate) fn decorator_target_label(&self, target: dir::DecoratorResolution) -> String {
        match target {
            dir::DecoratorResolution::LanguageItem(item) => item.key(),
            dir::DecoratorResolution::Symbol(symbol) => self.symbol_path_label(symbol),
            dir::DecoratorResolution::Unresolved => "unresolved".to_string(),
        }
    }
}
