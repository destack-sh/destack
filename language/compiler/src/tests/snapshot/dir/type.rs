use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, format};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::TypeTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render node level attachments where type checking placed information
        for (node_id, entry) in self.node_entries() {
            let mut row = SnapshotRow::new(builder.anchor_node(node_id), "type", "node")
                .field("node", builder.node_label(node_id));

            format::add_optional_type(&mut row, "declared", entry.declared);
            format::add_optional_type(&mut row, "inferred", entry.inferred);
            format::add_optional_type(&mut row, "receiver", entry.receiver);
            format::add_optional_type(&mut row, "contextual", entry.contextual);
            format::add_optional_type(&mut row, "signature", entry.signature);
            format::add_optional_instantiation(&mut row, "instantiation", entry.instantiation);

            if let Some(addressability) = entry.addressability {
                row = row.field("addressability", format::debug(addressability));
            }

            if let Some(resolution) = &entry.resolution {
                row = row.field("resolution", format::resolution(builder, resolution));
            }

            builder.push(row);
        }

        // render symbol level type attachments and generic metadata
        for (symbol_id, entry) in self.symbol_entries() {
            let mut row = SnapshotRow::new(builder.anchor_symbol(symbol_id), "type", "symbol")
                .field("name", builder.symbol_label(symbol_id));

            if let Some(parameter) = entry.generic_parameter {
                row = row
                    .field("parameter", format::debug(parameter.space))
                    .optional_field("constraint", format::optional_type_id(parameter.constraint))
                    .optional_field("variance", format::optional_debug(parameter.variance));
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

            format::add_optional_type(&mut row, "instance", entry.instance_type);
            format::add_optional_type(&mut row, "value", entry.value_type);
            format::add_optional_type(&mut row, "alias", entry.alias_target_type);

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
            .field("instantiations", self.instantiation_count().to_string())
            .field("lineages", self.lineage_count().to_string())
            .field("extensions", self.extension_count().to_string());
        builder.push(row);
    }
}
