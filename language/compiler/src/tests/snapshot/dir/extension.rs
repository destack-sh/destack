use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::SnapshotRow;

impl SnapshotTable for dir::ExtensionTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (symbol_id, target) in &self.target_by_symbol {
            let anchor = builder.anchor_symbol(symbol_id.into_global(self.module_id));
            let row = SnapshotRow::new(anchor, "extension", "target")
                .field("symbol", builder.local_symbol_label(*symbol_id))
                .field("target", builder.symbol_path_label(*target));
            builder.push(row);
        }
    }
}
