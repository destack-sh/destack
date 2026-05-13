use destack_dir::{GlobalSymbolId, LanguageItem};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Resolved compiler-known language symbols for a profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageSymbols {
    /// Top-level language symbols by item id.
    pub symbols: IndexMap<LanguageItem, GlobalSymbolId>,
}

impl LanguageSymbols {
    /// Get the symbol for a language item.
    pub fn get(&self, item: LanguageItem) -> Option<GlobalSymbolId> {
        self.symbols.get(&item).copied()
    }
}

/// Resolved compiler-known intrinsic bindings for a profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageIntrinsics {
    /// Intrinsic names keyed by symbol id.
    pub names_by_symbol: IndexMap<GlobalSymbolId, String>,
    /// Intrinsic symbols keyed by name.
    pub symbols_by_name: IndexMap<String, GlobalSymbolId>,
}

impl LanguageIntrinsics {
    /// Create an empty intrinsic map.
    pub fn new() -> Self {
        Self {
            names_by_symbol: IndexMap::new(),
            symbols_by_name: IndexMap::new(),
        }
    }

    /// Resolve an intrinsic name for a symbol.
    pub fn name_for_symbol(&self, symbol: GlobalSymbolId) -> Option<&str> {
        self.names_by_symbol.get(&symbol).map(String::as_str)
    }

    /// Resolve a symbol for an intrinsic name.
    pub fn symbol_for_name(&self, name: &str) -> Option<GlobalSymbolId> {
        self.symbols_by_name.get(name).copied()
    }
}
