use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::CoercionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node_id, coercion) in self.coercions() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "coercion", "node")
                .optional_field("source", builder.node_source(node_id))
                .type_field("from", builder.global_type_label(coercion.source))
                .type_field("to", builder.global_type_label(coercion.target))
                .field("origin", cast_origin_label(coercion.origin));

            builder.push(row);
        }

        let count = self.coercions().count();
        if count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "coercion", "summary")
            .count_field("nodes", count);
        builder.push(row);
    }
}

/// Return the snapshot label for one cast origin.
fn cast_origin_label(origin: dir::CastOrigin) -> &'static str {
    match origin {
        dir::CastOrigin::Explicit => "explicit",
        dir::CastOrigin::Implicit => "implicit",
    }
}
