use destack_core::StringId;
use destack_dir::{GlobalSymbolId, LanguageItem};
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::LanguageSymbols;

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
    /// Selected modules in load order.
    pub modules: Vec<ModuleId>,
}

impl GlobalEnvironment {
    /// Return the supporting modules needed to consume these bindings.
    pub fn supporting_modules(&self) -> Vec<ModuleId> {
        self.modules.clone()
    }

    /// Return the resolved language symbol table.
    pub fn language_symbols(&self) -> LanguageSymbols {
        LanguageSymbols {
            symbols: self.language.items.clone(),
        }
    }
}
