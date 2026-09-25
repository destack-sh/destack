use tspp_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ReferenceTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render each target at its exact reference site
        for (site, reference) in &self.targets {
            let row = builder.reference_row(*site, "target", reference);
            builder.push(row);
        }

        // render authored declarations that differ from their targets
        for (site, declaration) in &self.declarations {
            let row = builder.reference_row(*site, "declaration", declaration);
            builder.push(row);
        }

        if self.is_empty() {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "reference", "summary")
            .count_field("references", self.targets.len())
            .count_field("declarations", self.declarations.len());
        builder.push(row);
    }
}

impl DirSnapshotBuilder<'_> {
    /// Render one name reference row.
    fn reference_row(
        &self,
        site: dir::ReferenceSite,
        column: &'static str,
        reference: &dir::Reference,
    ) -> SnapshotRow {
        // identify the complete node or its exact path segment before the target fields
        let anchor = self.anchor_node(site.node());
        let source = self.reference_source_label(site.node());
        let row = SnapshotRow::new(anchor, "reference", column).field("source", source);
        let row = match site {
            dir::ReferenceSite::Node(_) => row,
            dir::ReferenceSite::Path { segment, .. } => row.field("segment", segment.to_string()),
        };

        // describe the resolved declaration or target
        match reference {
            dir::Reference::Bound(symbols) => {
                let targets = symbols.iter().map(|symbol| self.symbol_path_label(*symbol));

                row.field("kind", "bound").list_field("targets", targets)
            }
            dir::Reference::Namespace { module, .. } => row
                .field("kind", "namespace")
                .field("module", self.module_path(*module)),
            dir::Reference::TypeLiteral(literal) => row
                .field("kind", "literal")
                .field("literal", format!("{literal:?}")),
            dir::Reference::Projected { base, from } => {
                let base = match base {
                    dir::ReferenceTarget::Symbol(symbol) => self.symbol_path_label(*symbol),
                    dir::ReferenceTarget::Namespace(module) => self.module_path(*module),
                };

                row.field("kind", "projected")
                    .field("base", base)
                    .field("from", from.to_string())
            }
            dir::Reference::Ambiguous(ambiguous) => {
                let targets = ambiguous.iter().map(|target| match target {
                    dir::ReferenceTarget::Symbol(symbol) => self.symbol_path_label(*symbol),
                    dir::ReferenceTarget::Namespace(module) => self.module_path(*module),
                });

                row.field("kind", "ambiguous")
                    .list_field("targets", targets)
            }
            dir::Reference::Missing => row.field("kind", "missing"),
        }
    }
}
