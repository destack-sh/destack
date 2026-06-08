use crate::{LowerError, LowerResult};
use destack_dir as dir;

use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Resolve the symbol declared by one local DIR node.
    pub(crate) fn symbol_for_node<T: dir::Node>(
        &self,
        node_id: dir::LocalNodeId<T>,
    ) -> Option<dir::GlobalSymbolId> {
        let node_id = node_id.into_global_any(self.module_id);
        let symbol_id = self.symbols.symbol_for_declaration(node_id)?;

        Some(symbol_id.into_global(self.module_id))
    }

    /// Resolve the symbol declared by one local DIR node or fail loudly.
    pub(crate) fn require_symbol_for_node<T: dir::Node>(
        &self,
        node_id: dir::LocalNodeId<T>,
    ) -> LowerResult<dir::GlobalSymbolId> {
        let global_node_id = node_id.into_global_any(self.module_id);
        let symbol_id = self.symbols.symbol_for_declaration(global_node_id);

        symbol_id
            .map(|symbol_id| symbol_id.into_global(self.module_id))
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(global_node_id.into_anchored(Some(self.profile))),
                message: "node missing bound symbol".to_string(),
            })
    }

    /// Read one symbol record from local or bound DIR.
    pub(crate) fn symbol(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::Symbol> {
        if symbol_id.module_id == self.module_id {
            Some(self.symbols.get_symbol(symbol_id.local_id).clone())
        } else {
            let bound = self.dir_bound_if_present(symbol_id.module_id)?;

            Some(bound.bindings.get_symbol(symbol_id.local_id).clone())
        }
    }

    /// Return the declaration kind for one symbol.
    pub(crate) fn symbol_kind(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::SymbolKind> {
        Some(self.symbol(symbol_id)?.kind)
    }

    /// Return whether one symbol has the given declaration kind.
    pub(crate) fn symbol_kind_matches(
        &self,
        symbol_id: dir::GlobalSymbolId,
        kind: dir::SymbolKind,
    ) -> bool {
        self.symbol_kind(symbol_id)
            .is_some_and(|actual| actual == kind)
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
