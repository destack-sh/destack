use destack_dir as dir;

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
                dir::DecoratorSelection::Newtype { newtype, arguments } => {
                    let row = row
                        .field("kind", "newtype")
                        .type_tuple_field(
                            "parameters",
                            arguments
                                .iter()
                                .map(|argument| builder.global_type_label(argument.parameter_type)),
                        )
                        .optional_field("arguments", builder.argument_bindings_label(arguments));

                    builder.add_newtype_selection(row, newtype)
                }
                dir::DecoratorSelection::Derive { providers } => {
                    row.field("kind", "derive").list_field(
                        "providers",
                        providers
                            .iter()
                            .map(|provider| builder.derive_provider_label(provider)),
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
        selection: &dir::NewtypeSelection,
    ) -> SnapshotRow {
        row.field("newtype", self.symbol_path_label(selection.symbol))
            .type_field("backing", self.global_type_label(selection.backing))
            .optional_field(
                "generic_arguments",
                self.generic_arguments_label(&selection.generic_arguments),
            )
    }

    /// Return the snapshot label for one selected derive provider.
    fn derive_provider_label(&self, provider: &dir::DeriveProvider) -> String {
        let symbol = self.symbol_path_label(provider.newtype.symbol);
        let backing = self.global_type_label(provider.newtype.backing);
        let ty = self.global_type_label(provider.ty);

        format!("{symbol} backing={backing} type={ty}")
    }
}
