use destack_dir as dir;
use destack_source::{FileId, Span};

use crate::{ModuleQueryContext, ProgramQueryContext, QueryResult};

/// Documentation attached to one source node at a position.
pub(crate) struct DocumentationOccurrence {
    /// The visible node that owns the documentation.
    pub(crate) node_id: dir::GlobalNodeIdAny,
    /// The normalized documentation text.
    pub(crate) text: String,
    /// The source owner span.
    pub(crate) span: Span,
}

impl ModuleQueryContext<'_> {
    /// Return documentation owned by the innermost source node at a position.
    pub(crate) fn documentation_at_offset(
        &self,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<DocumentationOccurrence>> {
        // select the innermost source owner with documentation
        let view = self.view()?;
        for enclosing in self.enclosing_spans_at_cursor(file_id, offset)? {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            let Some(documentation) = view.get_documentation_any(node_id) else {
                continue;
            };
            let text = self.strings().get(documentation.text).to_string();

            return Ok(Some(DocumentationOccurrence {
                node_id: node_id.into_global(self.module_id()),
                text,
                span: enclosing.span,
            }));
        }

        Ok(None)
    }
}

impl ProgramQueryContext<'_> {
    /// Return the documentation for one symbol declaration.
    pub(crate) fn symbol_documentation(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<String>> {
        let module = self.module(symbol_id.module_id)?;
        let symbols = module.bindings()?;
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return Ok(None);
        };
        let Some(documentation) = module.view()?.get_documentation_any(declaration.local_id) else {
            return Ok(None);
        };

        Ok(Some(module.strings().get(documentation.text).to_string()))
    }
}
