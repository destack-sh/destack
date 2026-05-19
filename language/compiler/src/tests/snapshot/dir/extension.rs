use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, label};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ExtensionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (extension_id, extension) in self.iter_extensions() {
            let row = SnapshotRow::new(
                builder.anchor_symbol(extension.symbol),
                "extension",
                "entry",
            )
            .field("id", extension_id.to_string())
            .field("key", builder.symbol_path_label(extension.symbol))
            .field("form", label::variant_label(extension.form))
            .field("target", builder.symbol_path_label(extension.target));
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "extension", "summary")
            .field("extensions", self.extension_count().to_string());
        builder.push(row);
    }
}
