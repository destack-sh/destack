use crate::{LowerError, LowerResult};
use destack_dir as dir;

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Get the name of a symbol as a String.
    pub(crate) fn get_symbol_name(&self, symbol_id: dir::LocalSymbolId) -> Option<String> {
        // read the symbol entry
        let symbol = self.symbols.get_symbol(symbol_id);

        // resolve the symbol name
        symbol.name().map(|name| self.strings.get(name).to_string())
    }

    /// Get the name of a symbol, returning an error if it has no name.
    pub(crate) fn symbol_name(
        &self,
        symbol_id: dir::LocalSymbolId,
        node: dir::GlobalNodeIdAny,
    ) -> LowerResult<String> {
        // resolve the symbol name
        let name =
            self.get_symbol_name(symbol_id)
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node.into_anchored(Some(self.profile))),
                    message: "symbol must have a name".to_string(),
                })?;

        // return the resolved name
        Ok(name)
    }
}
