use destack_core::StringId;
use destack_dir::{
    GlobalSymbolId, LanguageSymbol, StaticKey, SymbolSpace, SymbolSpaceOrder, SymbolType,
    WellKnownSymbol, WellKnownSymbolKey,
};
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{AmbientLookupKey, SymbolGroup, WellKnownKey, WellKnownSymbols};

/// Language semantic environment for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageEnvironment {
    /// Resolved language items by builtin id.
    pub items: IndexMap<LanguageSymbol, GlobalSymbolId>,
    /// Resolved builtin symbols by export name.
    pub symbols: IndexMap<StringId, GlobalSymbolId>,
}

impl LanguageEnvironment {
    /// Return one language item symbol.
    pub fn item(&self, item: LanguageSymbol) -> Option<GlobalSymbolId> {
        self.items.get(&item).copied()
    }

    /// Return one builtin symbol by export name.
    pub fn symbol(&self, name: &str) -> Option<GlobalSymbolId> {
        self.symbols.get(&StringId::for_text(name)).copied()
    }
}

/// Ambient semantic environment for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AmbientEnvironment {
    /// Selected modules in load order.
    pub modules: Vec<ModuleId>,
    /// Ambient modules selected for this profile.
    pub ambient_modules: Vec<ModuleId>,
    /// Declared symbol sources for this profile.
    pub declared_symbols: IndexMap<AmbientLookupKey, Vec<GlobalSymbolId>>,
    /// Selected symbol sources for this profile.
    pub symbols: IndexMap<AmbientLookupKey, Vec<GlobalSymbolId>>,
}

impl AmbientEnvironment {
    /// Build one name key for ambient symbol lookup.
    fn name_key(name: &str) -> StaticKey {
        StaticKey::Name(StringId::for_text(name))
    }

    /// Collect ordered candidates for one key and space order.
    fn collect_symbol_sources_for_space_order(
        &self,
        key: StaticKey,
        order: SymbolSpaceOrder,
        is_declared_only: bool,
    ) -> Vec<GlobalSymbolId> {
        let mut sources = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // gather sources from each eligible space once
        let mut push_sources = |space| {
            let group = if is_declared_only {
                self.declared_symbol_sources(key, space)
            } else {
                self.symbol_sources(key, space)
            };

            if let Some(group) = group {
                for &symbol in group {
                    if seen.insert(symbol) {
                        sources.push(symbol);
                    }
                }
            }
        };

        // prefer the requested space order, while still considering dual space entries
        for space in order.spaces() {
            push_sources(*space);

            if matches!(space, SymbolSpace::Type | SymbolSpace::Value) {
                push_sources(SymbolSpace::TypeValue);
            }
        }

        sources
    }

    /// Return the first concrete declaration candidate.
    fn first_concrete_symbol(sources: &[GlobalSymbolId]) -> Option<GlobalSymbolId> {
        sources
            .iter()
            .copied()
            .find(|symbol| {
                matches!(
                    symbol.ty(),
                    SymbolType::Struct | SymbolType::Class | SymbolType::Enum | SymbolType::Newtype
                )
            })
            .or_else(|| sources.first().copied())
    }

    /// Return the supporting modules needed to consume this environment.
    pub fn supporting_modules(&self) -> Vec<ModuleId> {
        self.modules.clone()
    }

    /// Return declared ambient symbol sources for one key and space.
    pub fn declared_symbol_sources(
        &self,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<&[GlobalSymbolId]> {
        self.declared_symbols
            .get(&AmbientLookupKey { key, space })
            .map(<Vec<GlobalSymbolId>>::as_slice)
    }

    /// Return selected ambient symbol sources for one key and space.
    pub fn symbol_sources(&self, key: StaticKey, space: SymbolSpace) -> Option<&[GlobalSymbolId]> {
        self.symbols
            .get(&AmbientLookupKey { key, space })
            .map(<Vec<GlobalSymbolId>>::as_slice)
    }

    /// Return one declared symbol using semantic selection.
    pub fn declared_symbol_from(
        &self,
        name: &str,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let key = Self::name_key(name);
        self.declared_symbol_from_key(&key, order)
    }

    /// Return one declared symbol using semantic selection for a static key.
    pub fn declared_symbol_from_key(
        &self,
        key: &StaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let sources = self.collect_symbol_sources_for_space_order(*key, order, true);

        sources.first().copied()
    }

    /// Return one declared symbol using concrete runtime selection.
    pub fn declared_concrete_symbol_from(
        &self,
        name: &str,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let key = Self::name_key(name);
        self.declared_concrete_symbol_from_key(&key, order)
    }

    /// Return one declared symbol using concrete selection for a static key.
    pub fn declared_concrete_symbol_from_key(
        &self,
        key: &StaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let sources = self.collect_symbol_sources_for_space_order(*key, order, true);
        Self::first_concrete_symbol(&sources)
    }

    /// Return one selected symbol using semantic selection.
    pub fn symbol_from(&self, name: &str, order: SymbolSpaceOrder) -> Option<GlobalSymbolId> {
        let key = Self::name_key(name);
        self.symbol_from_key(&key, order)
    }

    /// Return one selected symbol using semantic selection for a static key.
    pub fn symbol_from_key(
        &self,
        key: &StaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let sources = self.collect_symbol_sources_for_space_order(*key, order, false);

        sources.first().copied()
    }

    /// Return selected symbol sources using a space order for a static key.
    pub fn symbol_sources_for_space_order(
        &self,
        key: StaticKey,
        order: SymbolSpaceOrder,
    ) -> Vec<GlobalSymbolId> {
        self.collect_symbol_sources_for_space_order(key, order, false)
    }

    /// Return the resolved well-known symbol table.
    pub fn well_known_symbols(&self) -> WellKnownSymbols {
        let mut symbols = IndexMap::new();

        // collect top level well known declarations from the selected library surface
        for item in WellKnownSymbol::all() {
            let group = SymbolGroup {
                ty: self.declared_symbol_from(item.export_name(), SymbolSpaceOrder::TypeThenValue),
                value: self
                    .declared_symbol_from(item.export_name(), SymbolSpaceOrder::ValueThenType),
            };

            if !group.is_empty() {
                symbols.insert(item, group);
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
