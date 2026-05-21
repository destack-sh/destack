use destack_core::StringId;
use destack_dir::{GlobalSymbolId, LanguageItem};
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Compiler-known language environment for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageEnvironment {
    /// Resolved language items by builtin id.
    pub items: IndexMap<LanguageItem, GlobalSymbolId>,
    /// Resolved builtin symbols by export name.
    pub symbols: IndexMap<StringId, GlobalSymbolId>,
}

impl LanguageEnvironment {
    /// Return one language item symbol.
    pub fn item(&self, item: LanguageItem) -> Option<GlobalSymbolId> {
        self.items.get(&item).copied()
    }

    /// Return one builtin symbol by export name.
    pub fn symbol(&self, name: &str) -> Option<GlobalSymbolId> {
        self.symbols.get(&StringId::for_text(name)).copied()
    }
}

/// Explicit global environment selected for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalEnvironment {
    /// Compiler-known language environment.
    pub language: LanguageEnvironment,
    /// Explicit global modules in load order.
    pub globals: Vec<ModuleId>,
}

impl GlobalEnvironment {
    /// Return the supporting modules needed to consume these bindings.
    pub fn supporting_modules(&self) -> Vec<ModuleId> {
        self.globals.clone()
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
