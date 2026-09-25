use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::DecoratorSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render decorator applications in source order
        for (_, application) in self.iter_applications() {
            let resolution = &application.resolution;
            let row = SnapshotRow::new(
                builder.anchor_node(application.source.into_any()),
                "decorator",
                "node",
            )
            .optional_field("source", builder.node_source(application.source.into_any()))
            .optional_field("owner", builder.node_source(application.owner))
            .optional_field(
                "expression",
                builder.node_source(application.expression.into_any()),
            )
            .field("target", builder.decorator_target_label(resolution.target))
            .type_field("type", builder.global_type_label(resolution.ty));
            let row = match &resolution.selection {
                dir::DecoratorSelection::Newtype {
                    key,
                    backing,
                    arguments,
                } => {
                    let row = row
                        .field("kind", "newtype")
                        .type_tuple_field(
                            "parameters",
                            arguments
                                .iter()
                                .map(|argument| builder.global_type_label(argument.parameter_type)),
                        )
                        .optional_field("arguments", builder.argument_bindings_label(arguments));

                    builder.add_newtype_selection(row, key, *backing)
                }
                dir::DecoratorSelection::Derive { interfaces } => {
                    row.field("kind", "derive").list_field(
                        "interfaces",
                        interfaces
                            .iter()
                            .map(|interface| interface.name().to_string()),
                    )
                }
            }
            .field("value", builder.global_static_label(application.value));

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
    /// Return the snapshot label for one decorator target.
    pub(crate) fn decorator_target_label(&self, target: dir::DecoratorTarget) -> String {
        match target {
            dir::DecoratorTarget::LanguageItem { item, .. } => item.key(),
            dir::DecoratorTarget::Symbol { symbol } => self.symbol_path_label(symbol),
        }
    }

    /// Add one exact newtype selection to a decorator snapshot row.
    fn add_newtype_selection(
        &self,
        row: SnapshotRow,
        key: &dir::InstanceKey,
        backing: dir::GlobalTypeId,
    ) -> SnapshotRow {
        row.field("newtype", self.symbol_path_label(key.symbol))
            .type_field("backing", self.global_type_label(backing))
            .optional_field(
                "generic_arguments",
                self.generic_arguments_label(&key.arguments),
            )
    }
}
