use destack_builtin::LanguageSymbol;
use destack_core::{StringId, StringPool};
use destack_dir::{
    GlobalSymbolId, StaticKey, SymbolSpace, SymbolSpaceOrder, WellKnownSymbol, WellKnownSymbolKey,
};
use destack_source::ModuleId;
use indexmap::IndexMap;

/// A symbol group containing type and value space entries.
/// Used to track both spaces for dual-space symbols like interfaces with constructors.
#[derive(Debug, Clone, Copy, Default)]
pub struct SymbolGroup {
    /// The type-space symbol, if any.
    pub ty: Option<GlobalSymbolId>,
    /// The value-space symbol, if any.
    pub value: Option<GlobalSymbolId>,
}

/// A symbol key for selected lib sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LibSymbolKey {
    /// The symbol key.
    pub key: StaticKey,
    /// The symbol space.
    pub space: SymbolSpace,
}

impl SymbolGroup {
    /// Create a new SymbolGroup from a type symbol.
    pub fn from_type(symbol: GlobalSymbolId) -> Self {
        Self {
            ty: Some(symbol),
            value: None,
        }
    }

    /// Create a new SymbolGroup from a value symbol.
    pub fn from_value(symbol: GlobalSymbolId) -> Self {
        Self {
            ty: None,
            value: Some(symbol),
        }
    }

    /// Create a new SymbolGroup from a type-value symbol.
    pub fn from_type_value(symbol: GlobalSymbolId) -> Self {
        Self {
            ty: Some(symbol),
            value: Some(symbol),
        }
    }

    /// Check if the group is empty.
    pub fn is_empty(&self) -> bool {
        self.ty.is_none() && self.value.is_none()
    }

    /// Get the symbol for the given space order preference.
    pub fn symbol_for_space_order(&self, order: SymbolSpaceOrder) -> Option<GlobalSymbolId> {
        match order {
            SymbolSpaceOrder::None => None,
            SymbolSpaceOrder::TypeOnly => self.ty,
            SymbolSpaceOrder::ValueOnly => self.value,
            SymbolSpaceOrder::TypeThenValue => self.ty.or(self.value),
            SymbolSpaceOrder::ValueThenType => self.value.or(self.ty),
        }
    }

    /// Merge another group into this one, overwriting none values.
    pub fn merge(&mut self, other: SymbolGroup) {
        if other.ty.is_some() {
            self.ty = other.ty;
        }
        if other.value.is_some() {
            self.value = other.value;
        }
    }
}

/// Well-known symbol key metadata.
#[derive(Debug, Clone, Copy)]
pub struct WellKnownKey {
    /// The base symbol for the key.
    pub symbol: GlobalSymbolId,
    /// The member name on the base symbol.
    pub member: StringId,
    /// The full global key name.
    pub global_name: StringId,
}

/// Resolved compiler-known symbols for a profile.
#[derive(Debug, Clone, Default)]
pub struct WellKnownSymbols {
    /// Top-level builtin symbols by well-known id.
    pub symbols: IndexMap<WellKnownSymbol, SymbolGroup>,
    /// Well-known symbol keys by id.
    pub keys: IndexMap<WellKnownSymbolKey, WellKnownKey>,
}

impl WellKnownSymbols {
    /// Build the well-known symbol map from resolved lib symbols.
    pub fn build(strings: &StringPool, lib_symbols: &IndexMap<StringId, SymbolGroup>) -> Self {
        let mut symbols = IndexMap::new();
        let mut keys = IndexMap::new();

        for item in WellKnownSymbol::all() {
            let name_id = strings.intern(item.export_name());
            let Some(group) = lib_symbols.get(&name_id).copied() else {
                continue;
            };
            if group.is_empty() {
                continue;
            }
            symbols.insert(item, group);
        }

        // for well-known keys, use the value symbol as the base
        for item in WellKnownSymbolKey::all() {
            let base_symbol = item.base_symbol();
            let Some(group) = symbols.get(&base_symbol) else {
                continue;
            };
            let Some(base_symbol_id) = group.value.or(group.ty) else {
                continue;
            };
            let member_id = strings.intern(item.member_name());
            let global_name_id = strings.intern(item.global_symbol_name());
            keys.insert(
                item,
                WellKnownKey {
                    symbol: base_symbol_id,
                    member: member_id,
                    global_name: global_name_id,
                },
            );
        }

        Self { symbols, keys }
    }

    /// Get the symbol group for a well-known symbol.
    pub fn get_group(&self, item: WellKnownSymbol) -> Option<SymbolGroup> {
        self.symbols.get(&item).copied()
    }

    /// Get the value-space well-known symbol by id.
    pub fn get_symbol(&self, item: WellKnownSymbol) -> Option<GlobalSymbolId> {
        self.symbols
            .get(&item)
            .and_then(|group| group.value.or(group.ty))
    }

    /// Get the type-space well-known symbol by id.
    pub fn get_type_symbol(&self, item: WellKnownSymbol) -> Option<GlobalSymbolId> {
        self.symbols
            .get(&item)
            .and_then(|group| group.ty.or(group.value))
    }

    /// Get the value-space well-known symbol by id.
    pub fn get_value_symbol(&self, item: WellKnownSymbol) -> Option<GlobalSymbolId> {
        self.symbols.get(&item).and_then(|group| group.value)
    }

    /// Get a well-known key by id.
    pub fn get_key(&self, item: WellKnownSymbolKey) -> Option<&WellKnownKey> {
        self.keys.get(&item)
    }

    /// Resolve a well-known key id for a base symbol and member name.
    pub fn symbol_key_for_member(
        &self,
        symbol: GlobalSymbolId,
        member: StringId,
    ) -> Option<WellKnownSymbolKey> {
        self.keys.iter().find_map(|(key_id, key)| {
            if key.symbol == symbol && key.member == member {
                return Some(*key_id);
            }
            None
        })
    }
}

/// Resolved compiler-known intrinsic bindings for a profile.
#[derive(Debug, Clone, Default)]
pub struct WellKnownIntrinsics {
    /// Intrinsic names keyed by symbol id.
    pub names_by_symbol: IndexMap<GlobalSymbolId, StringId>,
    /// Intrinsic symbols keyed by name.
    pub symbols_by_name: IndexMap<StringId, GlobalSymbolId>,
}

impl WellKnownIntrinsics {
    /// Create an empty well-known intrinsic map.
    pub fn new() -> Self {
        Self {
            names_by_symbol: IndexMap::new(),
            symbols_by_name: IndexMap::new(),
        }
    }

    /// Resolve an intrinsic name for a symbol.
    pub fn name_for_symbol(&self, symbol: GlobalSymbolId) -> Option<StringId> {
        self.names_by_symbol.get(&symbol).copied()
    }

    /// Resolve a symbol for an intrinsic name.
    pub fn symbol_for_name(&self, name: StringId) -> Option<GlobalSymbolId> {
        self.symbols_by_name.get(&name).copied()
    }
}

/// Language semantic environment for one profile.
#[derive(Debug, Clone, Default)]
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
    pub fn symbol(&self, name: StringId) -> Option<GlobalSymbolId> {
        self.symbols.get(&name).copied()
    }
}

/// Intrinsic semantic environment for one profile.
#[derive(Debug, Clone, Default)]
pub struct IntrinsicEnvironment {
    /// Well-known intrinsic bindings for this profile.
    pub intrinsics: WellKnownIntrinsics,
}

/// Lib semantic environment for one profile.
#[derive(Debug, Clone, Default)]
pub struct LibEnvironment {
    /// Selected lib modules in load order.
    pub modules: Vec<ModuleId>,
    /// Ambient lib modules selected for this profile.
    pub ambient_modules: Vec<ModuleId>,
    /// Declared lib symbols for this profile.
    pub declared_symbols: IndexMap<StringId, SymbolGroup>,
    /// Selected lib symbols for this profile.
    pub symbols: IndexMap<StringId, SymbolGroup>,
    /// Selected lib symbol sources for this profile.
    pub symbol_sources: IndexMap<LibSymbolKey, Vec<GlobalSymbolId>>,
    /// Well known symbols for this profile.
    pub well_known_symbols: WellKnownSymbols,
}

impl LibEnvironment {
    /// Return the supporting modules needed to consume this environment.
    pub fn supporting_modules(&self) -> Vec<ModuleId> {
        self.modules.clone()
    }

    /// Return one declared symbol group.
    pub fn declared_symbol_group(&self, name: StringId) -> Option<SymbolGroup> {
        self.declared_symbols.get(&name).copied()
    }

    /// Return one declared symbol by space order.
    pub fn declared_symbol_from(
        &self,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        self.declared_symbol_group(name)
            .and_then(|group| group.symbol_for_space_order(order))
    }

    /// Return selected lib symbol sources for one key and space.
    pub fn symbol_sources(
        &self,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<&Vec<GlobalSymbolId>> {
        self.symbol_sources.get(&LibSymbolKey { key, space })
    }

    /// Return one selected lib symbol group.
    pub fn symbol_group(&self, name: StringId) -> Option<SymbolGroup> {
        self.symbols.get(&name).copied()
    }

    /// Return one selected lib symbol by space order.
    pub fn symbol_from(&self, name: StringId, order: SymbolSpaceOrder) -> Option<GlobalSymbolId> {
        self.symbol_group(name)
            .and_then(|group| group.symbol_for_space_order(order))
    }
}
