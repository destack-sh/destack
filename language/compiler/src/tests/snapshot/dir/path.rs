use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ReferenceTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node, reference) in &self.target_by_node {
            let anchor = builder.anchor_node(*node);
            let source = builder.reference_source_label(*node);
            let row = match reference {
                dir::Reference::Bound(symbols) => {
                    let targets = symbols
                        .iter()
                        .map(|symbol| builder.symbol_path_label(*symbol));

                    SnapshotRow::new(anchor, "reference", "bound")
                        .field("source", source)
                        .list_field("targets", targets)
                }
                dir::Reference::Namespace(module) => {
                    SnapshotRow::new(anchor, "reference", "namespace")
                        .field("source", source)
                        .field("module", builder.module_path(*module))
                }
                dir::Reference::Projected { base, from } => {
                    let base = match base {
                        dir::ImportTarget::Symbol(symbol) => builder.symbol_path_label(*symbol),
                        dir::ImportTarget::Namespace(module) => builder.module_path(*module),
                    };

                    SnapshotRow::new(anchor, "reference", "projected")
                        .field("source", source)
                        .field("base", base)
                        .field("from", from.to_string())
                }
                dir::Reference::Ambiguous(ambiguous) => {
                    let targets = ambiguous.iter().map(|target| match target {
                        dir::ImportTarget::Symbol(symbol) => builder.symbol_path_label(*symbol),
                        dir::ImportTarget::Namespace(module) => builder.module_path(*module),
                    });

                    SnapshotRow::new(anchor, "reference", "ambiguous")
                        .field("source", source)
                        .list_field("targets", targets)
                }
                dir::Reference::Missing => {
                    SnapshotRow::new(anchor, "reference", "missing").field("source", source)
                }
            };
            builder.push(row);
        }

        for (node, declarations) in &self.declarations_by_node {
            let anchor = builder.anchor_node(*node);
            let source = builder.reference_root_label(*node);
            let declarations = declarations
                .iter()
                .map(|symbol| builder.symbol_path_label(*symbol));
            let row = SnapshotRow::new(anchor, "reference", "declaration")
                .field("source", source)
                .list_field("targets", declarations);
            builder.push(row);
        }

        if self.is_empty() {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "reference", "summary")
            .count_field("references", self.target_by_node.len())
            .count_field("declarations", self.declarations_by_node.len());
        builder.push(row);
    }
}
