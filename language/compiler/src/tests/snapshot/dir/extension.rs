use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ExtensionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (_, extension) in self.iter_extensions() {
            let row = SnapshotRow::new(
                builder.anchor_symbol(extension.symbol),
                "extension",
                "entry",
            )
            .field("symbol", builder.symbol_path_label(extension.symbol))
            .field("form", DirSnapshotBuilder::variant_label(extension.form))
            .type_field("target", builder.type_label(extension.target_type));
            builder.push(row);
        }

        let extension_count = self.extension_count();
        if extension_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "extension", "summary")
            .field("extensions", extension_count.to_string());
        builder.push(row);
    }
}
