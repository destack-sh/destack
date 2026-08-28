use destack_dir as dir;
use destack_source::{FileId, Span};

use crate::{Formatter, ModuleQueryContext, ProgramQueryContext, QueryResult};

/// Documentation attached to one source node at a position.
pub(crate) struct DocumentationOccurrence {
    /// The visible node that owns the documentation.
    pub(crate) node_id: dir::GlobalNodeIdAny,
    /// The rendered Markdown.
    pub(crate) markdown: String,
    /// The source owner span.
    pub(crate) span: Span,
}

impl ModuleQueryContext<'_> {
    /// Return documentation owned by the innermost source node at a position.
    pub(crate) fn documentation_at_offset(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<DocumentationOccurrence>> {
        // select the innermost source owner with documentation
        let view = self.view()?;
        for enclosing in self.enclosing_spans_at_cursor(file_id, offset)? {
            let Some(node_id) = view.resolve_node(enclosing.source_id) else {
                continue;
            };
            let Some(documentation) = view.get_documentation_any(node_id) else {
                continue;
            };
            let markdown = Formatter::new(self, program).documentation(documentation)?;

            return Ok(Some(DocumentationOccurrence {
                node_id: node_id.into_global(self.module_id()),
                markdown,
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

        let documentation = Formatter::new(&module, self).documentation(documentation)?;

        Ok(Some(documentation))
    }

    /// Return callable documentation for one symbol declaration.
    pub(crate) fn symbol_callable_documentation(
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

        let documentation = Formatter::new(&module, self).callable_documentation(documentation)?;

        Ok(Some(documentation))
    }
}
