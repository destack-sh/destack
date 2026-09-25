use tspp_dir as dir;
use tspp_source::Span;

use crate::QueryResult;
use crate::cursor::Cursor;

/// Documentation attached to one source node at a position.
pub(crate) struct DocumentationOccurrence<'a> {
    /// The visible node that owns the documentation.
    pub(crate) node_id: dir::GlobalNodeIdAny,
    /// The authored documentation.
    pub(crate) documentation: &'a dir::Documentation,
    /// The source owner span.
    pub(crate) span: Span,
}

impl Cursor<'_, '_> {
    /// Return documentation owned by the innermost source node at a position.
    pub(crate) fn documentation(&self) -> QueryResult<Option<DocumentationOccurrence<'_>>> {
        // select the innermost source owner with documentation
        let view = self.module.view()?;
        for enclosing in self.enclosing() {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            let Some(documentation) = view.get_documentation_any(node_id) else {
                continue;
            };

            return Ok(Some(DocumentationOccurrence {
                node_id: node_id.into_global(self.module.module_id()),
                documentation,
                span: enclosing.span,
            }));
        }

        Ok(None)
    }
}
