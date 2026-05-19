use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::TypeSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render node level attachments where type checking placed information
        for (node_id, entry) in self.node_entries() {
            let mut row = SnapshotRow::new(builder.anchor_node(node_id), "type", "node")
                .field("node", builder.node_label(node_id));

            row = row
                .optional_field("declared", value::optional_type_label(entry.declared))
                .optional_field("inferred", value::optional_type_label(entry.inferred))
                .optional_field("receiver", value::optional_type_label(entry.receiver))
                .optional_field("contextual", value::optional_type_label(entry.contextual))
                .optional_field("signature", value::optional_type_label(entry.signature));

            if let Some(addressability) = entry.addressability {
                row = row.field("addressability", value::debug_label(addressability));
            }

            builder.push(row);
        }

        // render symbol level type attachments and generic metadata
        for (symbol_id, entry) in self.symbol_entries() {
            let mut row = SnapshotRow::new(builder.anchor_symbol(symbol_id), "type", "symbol")
                .field("name", builder.symbol_label(symbol_id));

            if let Some(parameter) = entry.generic_parameter {
                row = row
                    .field("parameter", value::debug_label(parameter.space))
                    .optional_field(
                        "constraint",
                        value::optional_type_label(parameter.constraint),
                    )
                    .optional_field("variance", value::optional_debug_label(parameter.variance));
            }

            if let Some(parameters) = &entry.generic_parameter_symbols {
                row = row.field(
                    "parameters",
                    parameters
                        .iter()
                        .map(|symbol_id| builder.symbol_label(*symbol_id))
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }

            row = row
                .optional_field("instance", value::optional_type_label(entry.instance_type))
                .optional_field("value", value::optional_type_label(entry.value_type))
                .optional_field("alias", value::optional_type_label(entry.alias_target_type));

            if let Some(lineage_id) = entry.lineage_id {
                row = row.field("lineage", lineage_id.to_string());
            }

            if let Some(extension_id) = entry.extension_id {
                row = row.field("extension", extension_id.to_string());
            }

            builder.push(row);
        }

        // summarize larger stores that would drown out the source overlay
        let row = SnapshotRow::new(SnapshotAnchor::End, "type", "summary")
            .field("types", self.type_count().to_string())
            .field("nodes", self.node_entries().count().to_string())
            .field("symbols", self.symbol_entries().count().to_string())
            .field("lineages", self.lineage_count().to_string())
            .field("extensions", self.extension_count().to_string());
        builder.push(row);
    }
}
