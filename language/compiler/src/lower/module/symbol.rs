use crate::{LowerError, LowerResult};
use destack_dir as dir;

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Read one symbol record from local or declared DIR.
    pub(crate) fn symbol(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::Symbol> {
        if symbol_id.module_id == self.module_id {
            Some(self.symbols.get_symbol(symbol_id.local_id).clone())
        } else {
            let declared = self.artifact_dir_data_if_present(symbol_id.module_id)?;

            Some(declared.symbols.get_symbol(symbol_id.local_id).clone())
        }
    }

    /// Return the declaration form for one symbol.
    pub(crate) fn symbol_form(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::SymbolForm> {
        Some(self.symbol(symbol_id)?.form)
    }

    /// Return whether one symbol has the given declaration form.
    pub(crate) fn symbol_is(&self, symbol_id: dir::GlobalSymbolId, form: dir::SymbolForm) -> bool {
        self.symbol_form(symbol_id)
            .is_some_and(|actual| actual == form)
    }

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
