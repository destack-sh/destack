use tspp_dir as dir;

use crate::{Formatter, ProgramQueryContext, QueryResult};

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
