use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, label};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GenericSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render generic parameter lists
        for (symbol_id, parameter_list) in self.parameter_lists() {
            let row = SnapshotRow::new(builder.anchor_symbol(symbol_id), "generic", "parameters")
                .field("key", builder.symbol_path_label(symbol_id))
                .list_field(
                    "parameters",
                    parameter_list
                        .parameters
                        .iter()
                        .map(|symbol_id| builder.symbol_path_label(*symbol_id)),
                );
            builder.push(row);
        }

        // render generic parameter metadata
        for (symbol_id, parameter) in self.parameters() {
            let row = match parameter {
                dir::GenericParameterShape::Type {
                    constraint,
                    default,
                    variance,
                } => SnapshotRow::new(builder.anchor_symbol(symbol_id), "generic", "parameter")
                    .field("key", builder.symbol_path_label(symbol_id))
                    .field("space", "type")
                    .optional_field("constraint", constraint.map(|ty| builder.type_label(ty)))
                    .optional_field("default", default.map(|ty| builder.type_label(ty)))
                    .optional_field("variance", variance.map(label::variant_label)),
                dir::GenericParameterShape::Static { ty, default } => {
                    SnapshotRow::new(builder.anchor_symbol(symbol_id), "generic", "parameter")
                        .field("key", builder.symbol_path_label(symbol_id))
                        .field("space", "static")
                        .optional_field("type", ty.map(|ty| builder.type_label(ty)))
                        .optional_field(
                            "default",
                            default.map(|term| label::static_term_label(builder, &term)),
                        )
                }
            };
            builder.push(row);
        }

        // summarize generic table coverage
        let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "summary")
            .field("parameters", self.parameters().count().to_string())
            .field("lists", self.parameter_lists().count().to_string());
        builder.push(row);
    }
}
