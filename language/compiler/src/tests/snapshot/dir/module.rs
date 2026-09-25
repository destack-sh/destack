use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ModuleSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for module in self.iter() {
            let row = SnapshotRow::new(builder.anchor_node(module.source), "module", "edge")
                .field(
                    "relation",
                    DirSnapshotBuilder::variant_label(module.relation),
                )
                .field("specifier", builder.strings.get(module.specifier));
            let row = if let Some(loader) = module.loader {
                row.field("loader", loader.as_str())
            } else {
                row
            };
            let (target_key, target_value) = builder.module_target_field(module.target);
            let row = row.field(target_key, target_value);
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "module", "summary")
            .field("edges", self.edges.len().to_string());
        builder.push(row);
    }
}
