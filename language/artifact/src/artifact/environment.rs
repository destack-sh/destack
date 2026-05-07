use destack_core::StringId;
use destack_dir::{
    GlobalSymbolId, LanguageItem, StaticKey, SymbolSpace, WellKnownSymbol, WellKnownSymbolKey,
};
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{GlobalEnvironmentKey, SymbolPair, WellKnownKey, WellKnownSymbols};

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
    /// Selected global symbol by key and exact space.
    pub symbols: IndexMap<GlobalEnvironmentKey, GlobalSymbolId>,
}

impl GlobalEnvironment {
    /// Build one name key for global symbol lookup.
    fn name_key(name: &str) -> StaticKey {
        StaticKey::Name(StringId::for_text(name))
    }

    /// Return the supporting modules needed to consume these bindings.
    pub fn supporting_modules(&self) -> Vec<ModuleId> {
        self.modules.clone()
    }

    /// Return one selected global symbol for one key and space.
    pub fn symbol(&self, key: StaticKey, space: SymbolSpace) -> Option<GlobalSymbolId> {
        self.symbols
            .get(&GlobalEnvironmentKey { key, space })
            .copied()
    }

    /// Return one selected global symbol by name and space.
    pub fn symbol_from(&self, name: &str, space: SymbolSpace) -> Option<GlobalSymbolId> {
        let key = Self::name_key(name);
        self.symbol_from_key(key, space)
    }

    /// Return one selected global symbol by key and space.
    pub fn symbol_from_key(&self, key: StaticKey, space: SymbolSpace) -> Option<GlobalSymbolId> {
        self.symbol(key, space)
    }

    /// Return the resolved well-known symbol table.
    pub fn well_known_symbols(&self) -> WellKnownSymbols {
        let mut symbols = IndexMap::new();

        // collect top level well known declarations from the selected library surface
        for item in WellKnownSymbol::all() {
            let pair = SymbolPair {
                ty: self.symbol_from(item.export_name(), SymbolSpace::Type),
                value: self.symbol_from(item.export_name(), SymbolSpace::Value),
            };

            if !pair.is_empty() {
                symbols.insert(item, pair);
            }
        }

        // derive well known Symbol.* keys from the resolved base symbols
        let mut keys = IndexMap::new();
        for item in WellKnownSymbolKey::all() {
            let Some(base_symbol) = symbols
                .get(&item.base_symbol())
                .and_then(|group| group.value.or(group.ty))
            else {
                continue;
            };

            keys.insert(
                item,
                WellKnownKey {
                    symbol: base_symbol,
                    member: item.member_name().to_string(),
                    global_name: item.global_symbol_name().to_string(),
                },
            );
        }

        WellKnownSymbols { symbols, keys }
    }
}
