use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ReferenceTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (node, reference) in &self.target_by_node {
            let anchor = builder.anchor_node(*node);
            let source = builder.reference_source_label(*node);
            let row = builder.reference_row(anchor, source, "target", reference);
            builder.push(row);
        }

        for (node, declaration) in &self.declaration_by_node {
            let anchor = builder.anchor_node(*node);
            let source = builder.reference_root_label(*node);
            let row = builder.reference_row(anchor, source, "declaration", declaration);
            builder.push(row);
        }

        if self.is_empty() {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "reference", "summary")
            .count_field("references", self.target_by_node.len())
            .count_field("declarations", self.declaration_by_node.len());
        builder.push(row);
    }
}

impl DirSnapshotBuilder<'_> {
    /// Render one name reference row.
    fn reference_row(
        &self,
        anchor: SnapshotAnchor,
        source: String,
        column: &'static str,
        reference: &dir::Reference,
    ) -> SnapshotRow {
        match reference {
            dir::Reference::Bound(symbols) => {
                let targets = symbols.iter().map(|symbol| self.symbol_path_label(*symbol));

                SnapshotRow::new(anchor, "reference", column)
                    .field("source", source)
                    .field("kind", "bound")
                    .list_field("targets", targets)
            }
            dir::Reference::Namespace(module) => SnapshotRow::new(anchor, "reference", column)
                .field("source", source)
                .field("kind", "namespace")
                .field("module", self.module_path(*module)),
            dir::Reference::Projected { base, from } => {
                let base = match base {
                    dir::ReferenceTarget::Symbol(symbol) => self.symbol_path_label(*symbol),
                    dir::ReferenceTarget::Namespace(module) => self.module_path(*module),
                };

                SnapshotRow::new(anchor, "reference", column)
                    .field("source", source)
                    .field("kind", "projected")
                    .field("base", base)
                    .field("from", from.to_string())
            }
            dir::Reference::Ambiguous(ambiguous) => {
                let targets = ambiguous.iter().map(|target| match target {
                    dir::ReferenceTarget::Symbol(symbol) => self.symbol_path_label(*symbol),
                    dir::ReferenceTarget::Namespace(module) => self.module_path(*module),
                });

                SnapshotRow::new(anchor, "reference", column)
                    .field("source", source)
                    .field("kind", "ambiguous")
                    .list_field("targets", targets)
            }
            dir::Reference::Missing => SnapshotRow::new(anchor, "reference", column)
                .field("source", source)
                .field("kind", "missing"),
        }
    }
}
