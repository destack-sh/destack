use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GenericSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render declaration slots
        for (owner, slots) in self.declarations() {
            for (index, slot) in slots.slots.iter().enumerate() {
                let symbol = slot.symbol();
                let row = match slot {
                    dir::GenericSlot::Type {
                        constraint,
                        default,
                        variance,
                        ..
                    } => SnapshotRow::new(builder.anchor_symbol(owner), "generic", "slot")
                        .field("symbol", builder.symbol_path_label(symbol))
                        .field("index", index.to_string())
                        .field("kind", "type")
                        .optional_field("constraint", constraint.map(|ty| builder.type_label(ty)))
                        .optional_field("default", default.map(|ty| builder.type_label(ty)))
                        .optional_field(
                            "variance",
                            variance.map(DirSnapshotBuilder::variant_label),
                        ),
                    dir::GenericSlot::Static {
                        constraint,
                        default,
                        ..
                    } => SnapshotRow::new(builder.anchor_symbol(owner), "generic", "slot")
                        .field("symbol", builder.symbol_path_label(symbol))
                        .field("index", index.to_string())
                        .field("kind", "static")
                        .optional_field("constraint", constraint.map(|ty| builder.type_label(ty)))
                        .optional_field("default", default.map(|term| builder.static_label(term))),
                };
                builder.push(row);
            }
        }

        // summarize generic table coverage when present
        let declaration_count = self.declarations().count();
        if declaration_count == 0 {
            return;
        }

        let slot_count = self
            .declarations()
            .map(|(_, slots)| slots.slots.len())
            .sum::<usize>();
        let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "summary")
            .count_field("declarations", declaration_count)
            .count_field("slots", slot_count);
        builder.push(row);
    }
}
