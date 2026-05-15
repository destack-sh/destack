use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, format};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::DependencyTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for dependency in self.iter() {
            let row =
                SnapshotRow::new(builder.anchor_node(dependency.source), "dependency", "edge")
                    .field("specifier", builder.strings.get(dependency.specifier))
                    .field("relation", format::debug(dependency.relation))
                    .field(
                        "target",
                        format::dependency_target(builder, dependency.target),
                    );
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "dependency", "summary")
            .field("edges", self.dependencies.len().to_string());
        builder.push(row);
    }
}
