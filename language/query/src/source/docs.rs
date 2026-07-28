use destack_dir as dir;

use crate::{ProgramQueryContext, QueryResult};

impl ProgramQueryContext<'_> {
    /// Return the checked documentation for one symbol declaration.
    pub(crate) fn symbol_doc_text(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<String>> {
        let module = self.module(symbol_id.module_id)?;
        let symbols = module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return Ok(None);
        };
        let Some(documentation) = module.view().get_documentation_any(declaration.local_id) else {
            return Ok(None);
        };

        Ok(Some(module.strings().get(documentation.text).to_string()))
    }
}
