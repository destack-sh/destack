use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::RelationSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (symbol_id, relation) in self.extends_entries() {
            add_relation_row(builder, symbol_id, relation);
        }

        for (symbol_id, relation) in self.implements_entries() {
            add_relation_row(builder, symbol_id, relation);
        }

        let extends_count = self.extends_entries().count();
        let implements_count = self.implements_entries().count();
        if extends_count == 0 && implements_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "relation", "summary")
            .count_field("extends", extends_count)
            .count_field("implements", implements_count);
        builder.push(row);
    }
}

/// Add one solved relation row.
fn add_relation_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol_id: dir::GlobalSymbolId,
    relation: dir::Relation,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(symbol_id), "relation", "entry")
        .field("symbol", builder.symbol_path_label(symbol_id))
        .field("kind", DirSnapshotBuilder::variant_label(relation.kind))
        .type_field("type", builder.global_type_label(relation.ty));

    builder.push(row);
}
