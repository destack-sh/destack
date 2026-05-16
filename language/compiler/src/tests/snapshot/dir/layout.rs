use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::LayoutSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        let row = SnapshotRow::new(SnapshotAnchor::End, "layout", "summary").field("entries", "0");

        builder.push(row);
    }
}
