use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::PathTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (key, resolution) in &self.entries {
            let anchor = builder.anchor_node(key.source);
            let source = builder.source_path_prefix_label(*key);
            let row = match resolution {
                dir::PathResolution::Found(target) => {
                    path_target_row(builder, anchor, source, *target)
                }
                dir::PathResolution::Missing => {
                    SnapshotRow::new(anchor, "path", "missing").field("source", source)
                }
                dir::PathResolution::Ambiguous(targets) => {
                    let targets = targets
                        .iter()
                        .map(|target| path_target_label(builder, *target));

                    SnapshotRow::new(anchor, "path", "ambiguous")
                        .field("source", source)
                        .list_field("targets", targets)
                }
            };
            builder.push(row);
        }

        if self.entries.is_empty() {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "path", "summary")
            .count_field("paths", self.entries.len());
        builder.push(row);
    }
}

/// Return one path target row.
fn path_target_row(
    builder: &DirSnapshotBuilder<'_>,
    anchor: SnapshotAnchor,
    source: String,
    target: dir::PathTarget,
) -> SnapshotRow {
    match target {
        dir::PathTarget::Symbol(symbol) => SnapshotRow::new(anchor, "path", "symbol")
            .field("source", source)
            .field("target", builder.symbol_path_label(symbol)),
        dir::PathTarget::Namespace(module) => SnapshotRow::new(anchor, "path", "namespace")
            .field("source", source)
            .field("module", builder.module_path(module)),
    }
}

/// Return one path target label.
fn path_target_label(builder: &DirSnapshotBuilder<'_>, target: dir::PathTarget) -> String {
    match target {
        dir::PathTarget::Symbol(symbol) => builder.symbol_path_label(symbol),
        dir::PathTarget::Namespace(module) => builder.module_path(module),
    }
}
