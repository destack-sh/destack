use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GenericSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // slots
        for (_, slot) in self.iter_slots() {
            let anchor = builder.anchor_symbol(slot.owner());
            let row = match slot {
                dir::GenericSlot::Type {
                    owner,
                    key,
                    index,
                    constraint,
                    default,
                    ..
                } => generic_slot_key_field(
                    SnapshotRow::new(anchor, "generic", "slot"),
                    builder,
                    *owner,
                    *key,
                )
                .field("index", index.get().to_string())
                .field("kind", "type")
                .optional_type_field("constraint", constraint.map(|id| builder.type_label(id)))
                .optional_type_field("default", default.map(|id| builder.type_label(id))),
                dir::GenericSlot::VariadicType {
                    owner,
                    key,
                    index,
                    constraint,
                    default,
                    ..
                } => generic_slot_key_field(
                    SnapshotRow::new(anchor, "generic", "slot"),
                    builder,
                    *owner,
                    *key,
                )
                .field("index", index.get().to_string())
                .field("kind", "variadicType")
                .optional_type_field("constraint", constraint.map(|id| builder.type_label(id)))
                .optional_type_field("default", default.map(|id| builder.type_label(id))),
                dir::GenericSlot::Static {
                    owner,
                    key,
                    index,
                    constraint,
                    default,
                    ..
                } => generic_slot_key_field(
                    SnapshotRow::new(anchor, "generic", "slot"),
                    builder,
                    *owner,
                    *key,
                )
                .field("index", index.get().to_string())
                .field("kind", "static")
                .optional_type_field("constraint", constraint.map(|id| builder.type_label(id)))
                .optional_field("default", default.map(|id| builder.static_label(id))),
                dir::GenericSlot::VariadicStatic {
                    owner,
                    key,
                    index,
                    constraint,
                    default,
                    ..
                } => generic_slot_key_field(
                    SnapshotRow::new(anchor, "generic", "slot"),
                    builder,
                    *owner,
                    *key,
                )
                .field("index", index.get().to_string())
                .field("kind", "variadicStatic")
                .optional_type_field("constraint", constraint.map(|id| builder.type_label(id)))
                .optional_field("default", default.map(|id| builder.static_label(id))),
            };

            builder.push(row);
        }

        // render generic application sites
        for (node_id, instance_id) in self.node_instances() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "generic", "application")
                .optional_field("source", builder.node_source(node_id))
                .field("id", builder.generic_instance_label(instance_id));

            builder.push(row);
        }

        // render generic instances
        for (instance_id, instance) in self.iter_instances() {
            let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "instance")
                .field("id", builder.generic_instance_label(instance_id))
                .field("symbol", builder.symbol_path_label(instance.symbol))
                .list_field(
                    "arguments",
                    instance
                        .arguments
                        .iter()
                        .map(|argument| builder.static_argument_label(argument)),
                );

            builder.push(row);
        }

        let slot_count = self.slot_count();
        let instance_count = self.instance_count();
        let application_count = self.node_instance_count();
        if slot_count == 0 && instance_count == 0 && application_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "summary")
            .count_field("slots", slot_count)
            .count_field("instances", instance_count)
            .count_field("applications", application_count);
        builder.push(row);
    }
}

/// Add one generic slot key field.
fn generic_slot_key_field(
    row: SnapshotRow,
    builder: &DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    key: dir::GenericSlotKey,
) -> SnapshotRow {
    match key {
        dir::GenericSlotKey::Symbol(symbol) => row.field(
            "symbol",
            format!(
                "{}.{}",
                builder.symbol_path_label(owner),
                builder.symbol_label(symbol)
            ),
        ),
        dir::GenericSlotKey::Generated(_) => row.field("key", builder.generic_slot_key_label(key)),
    }
}
