use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ImportTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (symbol_id, resolution) in &self.resolution_by_symbol {
            let anchor = builder.anchor_symbol(symbol_id.into_global(self.module_id));
            let row = match resolution {
                dir::ImportResolution::Resolved(resolution) => {
                    SnapshotRow::new(anchor, "import", "resolved")
                        .field("symbol", builder.local_symbol_label(*symbol_id))
                        .list_field(
                            "declarations",
                            builder.export_target_labels(&resolution.declaration),
                        )
                        .list_field("targets", builder.export_target_labels(&resolution.target))
                }
                dir::ImportResolution::Ambiguous(resolutions) => {
                    let declarations = resolutions.iter().flat_map(|resolution| {
                        builder.export_target_labels(&resolution.declaration)
                    });
                    let targets = resolutions
                        .iter()
                        .flat_map(|resolution| builder.export_target_labels(&resolution.target));

                    SnapshotRow::new(anchor, "import", "ambiguous")
                        .field("symbol", builder.local_symbol_label(*symbol_id))
                        .list_field("declarations", declarations)
                        .list_field("targets", targets)
                }
                dir::ImportResolution::Missing => SnapshotRow::new(anchor, "import", "missing")
                    .field("symbol", builder.local_symbol_label(*symbol_id)),
            };
            builder.push(row);
        }

        for (key, resolutions) in &self.global_resolution_by_key {
            let declarations = resolutions
                .iter()
                .flat_map(|resolution| builder.export_target_labels(&resolution.declaration));
            let targets = resolutions
                .iter()
                .flat_map(|resolution| builder.export_target_labels(&resolution.target));
            let row = SnapshotRow::new(SnapshotAnchor::End, "import", "global")
                .field("key", builder.static_key(*key))
                .list_field("declarations", declarations)
                .list_field("targets", targets);
            builder.push(row);
        }

        for (item, symbol) in &self.language_symbol_by_item {
            let row = SnapshotRow::new(SnapshotAnchor::End, "import", "language")
                .field("item", item.to_string())
                .field("symbol", builder.symbol_path_label(*symbol));
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "import", "summary")
            .count_field("symbols", self.resolution_by_symbol.len())
            .count_field("globals", self.global_resolution_by_key.len())
            .count_field("language", self.language_symbol_by_item.len());
        builder.push(row);
    }
}
