use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::DependencySegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for dependency in self.iter() {
            let row =
                SnapshotRow::new(builder.anchor_node(dependency.source), "dependency", "edge")
                    .field("relation", value::debug_label(dependency.relation))
                    .field("specifier", builder.strings.get(dependency.specifier));
            let row = if let Some(loader) = dependency.loader {
                row.field("loader", loader.as_str())
            } else {
                row
            };
            let (target_key, target_value) =
                value::dependency_target_field(builder, dependency.target);
            let row = row.field(target_key, target_value);
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "dependency", "summary")
            .field("edges", self.edges.len().to_string());
        builder.push(row);
    }
}
