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
                    key,
                    index,
                    constraint,
                    default,
                    ..
                } => SnapshotRow::new(anchor, "generic", "slot")
                    .field("key", builder.generic_slot_key_label(*key))
                    .field("index", index.get().to_string())
                    .field("kind", "type")
                    .optional_type_field("constraint", constraint.map(|id| builder.type_label(id)))
                    .optional_type_field("default", default.map(|id| builder.type_label(id))),
                dir::GenericSlot::VariadicType {
                    key,
                    index,
                    constraint,
                    default,
                    ..
                } => SnapshotRow::new(anchor, "generic", "slot")
                    .field("key", builder.generic_slot_key_label(*key))
                    .field("index", index.get().to_string())
                    .field("kind", "variadicType")
                    .optional_type_field("constraint", constraint.map(|id| builder.type_label(id)))
                    .optional_type_field("default", default.map(|id| builder.type_label(id))),
                dir::GenericSlot::Static {
                    key,
                    index,
                    constraint,
                    default,
                    ..
                } => SnapshotRow::new(anchor, "generic", "slot")
                    .field("key", builder.generic_slot_key_label(*key))
                    .field("index", index.get().to_string())
                    .field("kind", "static")
                    .optional_type_field("constraint", constraint.map(|id| builder.type_label(id)))
                    .optional_field("default", default.map(|id| builder.static_label(id))),
                dir::GenericSlot::VariadicStatic {
                    key,
                    index,
                    constraint,
                    default,
                    ..
                } => SnapshotRow::new(anchor, "generic", "slot")
                    .field("key", builder.generic_slot_key_label(*key))
                    .field("index", index.get().to_string())
                    .field("kind", "variadicStatic")
                    .optional_type_field("constraint", constraint.map(|id| builder.type_label(id)))
                    .optional_field("default", default.map(|id| builder.static_label(id))),
            };

            builder.push(row);
        }

        // render instance application sites
        for (node_id, instance_id) in self.node_instances() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "instance", "application")
                .optional_field("source", builder.node_source(node_id))
                .field("id", builder.generic_instance_label(instance_id));

            builder.push(row);
        }

        // instances
        for (instance_id, instance) in self.iter_instances() {
            let row = SnapshotRow::new(SnapshotAnchor::End, "instance", "entry")
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
