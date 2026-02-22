use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{Extension, GlobalSymbolId, LocalExtensionId};

use super::TypeTable;

/// Extension ownership for extension declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionTable {
    /// The next extension id to allocate.
    pub(crate) next_extension_id: u32,
    /// The extensions.
    pub(crate) extensions: crate::Arena<Extension>,
    /// Extensions by their declaration symbol (for lookup by extension symbol).
    pub(crate) extension_by_symbol: IndexMap<GlobalSymbolId, LocalExtensionId>,
    /// Extensions indexed by target symbol (for member lookup on types).
    pub(crate) extensions_by_target: IndexMap<GlobalSymbolId, Vec<LocalExtensionId>>,
}

impl ExtensionTable {
    /// Create an empty extension table.
    pub fn new() -> Self {
        Self {
            next_extension_id: 0,
            extensions: crate::Arena::new(),
            extension_by_symbol: IndexMap::new(),
            extensions_by_target: IndexMap::new(),
        }
    }
}

impl TypeTable {
    /// Insert a new extension.
    pub fn insert_extension(&mut self, extension: Extension) -> LocalExtensionId {
        let extension_id = LocalExtensionId::new(self.extension.next_extension_id);
        self.extension.next_extension_id += 1;

        // index by target symbol for member lookup
        let target = extension.target;
        self.extension
            .extensions_by_target
            .entry(target)
            .or_default()
            .push(extension_id);

        // index by extension symbol
        self.extension
            .extension_by_symbol
            .insert(extension.symbol, extension_id);

        self.extension.extensions.allocate(extension);
        extension_id
    }

    /// Get an extension by its id.
    pub fn get_extension(&self, extension_id: LocalExtensionId) -> &Extension {
        self.extension.extensions.get(extension_id.0)
    }

    /// Get an extension id by its symbol.
    pub fn get_extension_id_for_symbol(
        &self,
        extension_symbol: GlobalSymbolId,
    ) -> Option<LocalExtensionId> {
        self.extension
            .extension_by_symbol
            .get(&extension_symbol)
            .copied()
    }

    /// Get a mutable extension by its id.
    pub fn get_extension_mut(&mut self, extension_id: LocalExtensionId) -> &mut Extension {
        self.extension.extensions.get_mut(extension_id.0)
    }

    /// Get all extensions targeting a specific type symbol.
    pub fn get_extensions_for_target(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Option<&Vec<LocalExtensionId>> {
        self.extension.extensions_by_target.get(&target_symbol)
    }

    /// Iterate over all extensions.
    pub fn iter_extensions(&self) -> impl Iterator<Item = (LocalExtensionId, &Extension)> {
        (0..self.extension.next_extension_id).map(|i| {
            let id = LocalExtensionId::new(i);
            (id, self.extension.extensions.get(i))
        })
    }
}
