use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, label};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::RelationSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (symbol_id, relation) in self.extends_entries() {
            add_relation_row(builder, symbol_id, relation);
        }

        for (symbol_id, relation) in self.implements_entries() {
            add_relation_row(builder, symbol_id, relation);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "relation", "summary")
            .field("extends", self.extends_entries().count().to_string())
            .field("implements", self.implements_entries().count().to_string());
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
        .field("key", builder.symbol_path_label(symbol_id))
        .field("kind", label::variant_label(relation.kind))
        .field("type", builder.type_label(relation.ty));

    builder.push(row);
}
