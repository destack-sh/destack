use destack_builtin::LanguageSymbol;
use destack_core::StringPool;
use destack_dir::{
    GlobalSymbolId, StaticKey, SymbolKey, SymbolSpace, SymbolSpaceOrder, SymbolType,
    WellKnownSymbol, WellKnownSymbolKey,
};
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A symbol group containing type and value space entries.
/// Used to track both spaces for dual-space symbols like interfaces with constructors.
#[derive(Debug, Clone, Copy, Default)]
pub struct SymbolGroup {
    /// The type-space symbol, if any.
    pub ty: Option<GlobalSymbolId>,
    /// The value-space symbol, if any.
    pub value: Option<GlobalSymbolId>,
}

/// A canonical symbol key that does not depend on ambient string identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CanonicalSymbolKey {
    /// Unique symbol key from a declaration.
    Unique(GlobalSymbolId),
    /// Well known Symbol.* key.
    WellKnown(WellKnownSymbolKey),
    /// Symbol.for registry key.
    Registry(String),
}

impl CanonicalSymbolKey {
    /// Build one canonical symbol key from a live symbol key.
    pub fn from_symbol_key(key: SymbolKey, strings: &StringPool) -> Self {
        match key {
            SymbolKey::Unique(symbol) => Self::Unique(symbol),
            SymbolKey::WellKnown(symbol) => Self::WellKnown(symbol),
            SymbolKey::Registry(name) => Self::Registry(strings.get(name).to_string()),
        }
    }
}

/// A canonical static key that does not depend on ambient string identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CanonicalStaticKey {
    /// Regular name key.
    Name(String),
    /// Numeric name key.
    Number(String),
    /// Symbol key.
    Symbol(CanonicalSymbolKey),
}

impl CanonicalStaticKey {
    /// Build one canonical static key from a live static key.
    pub fn from_static_key(key: StaticKey, strings: &StringPool) -> Self {
        match key {
            StaticKey::Name(name) => Self::Name(strings.get(name).to_string()),
            StaticKey::Number(name) => Self::Number(strings.get(name).to_string()),
            StaticKey::Symbol(symbol) => {
                Self::Symbol(CanonicalSymbolKey::from_symbol_key(symbol, strings))
            }
        }
    }
}

/// A symbol key for selected library sources.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LibrarySymbolKey {
    /// The symbol key.
    pub key: CanonicalStaticKey,
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
#[derive(Debug, Clone)]
pub struct WellKnownKey {
    /// The base symbol for the key.
    pub symbol: GlobalSymbolId,
    /// The member name on the base symbol.
    pub member: String,
    /// The full global key name.
    pub global_name: String,
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
        member: &str,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WellKnownIntrinsics {
    /// Intrinsic names keyed by symbol id.
    pub names_by_symbol: IndexMap<GlobalSymbolId, String>,
    /// Intrinsic symbols keyed by name.
    pub symbols_by_name: IndexMap<String, GlobalSymbolId>,
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
    pub fn name_for_symbol(&self, symbol: GlobalSymbolId) -> Option<&str> {
        self.names_by_symbol.get(&symbol).map(String::as_str)
    }

    /// Resolve a symbol for an intrinsic name.
    pub fn symbol_for_name(&self, name: &str) -> Option<GlobalSymbolId> {
        self.symbols_by_name.get(name).copied()
    }
}

/// Language semantic environment for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageEnvironment {
    /// Resolved language items by builtin id.
    pub items: IndexMap<LanguageSymbol, GlobalSymbolId>,
    /// Resolved builtin symbols by export name.
    pub symbols: IndexMap<String, GlobalSymbolId>,
}

impl LanguageEnvironment {
    /// Return one language item symbol.
    pub fn item(&self, item: LanguageSymbol) -> Option<GlobalSymbolId> {
        self.items.get(&item).copied()
    }

    /// Return one builtin symbol by export name.
    pub fn symbol(&self, name: &str) -> Option<GlobalSymbolId> {
        self.symbols.get(name).copied()
    }
}

/// Intrinsic semantic environment for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntrinsicEnvironment {
    /// Well-known intrinsic bindings for this profile.
    pub intrinsics: WellKnownIntrinsics,
}

/// Library semantic environment for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryEnvironment {
    /// Selected library modules in load order.
    pub modules: Vec<ModuleId>,
    /// Ambient library modules selected for this profile.
    pub ambient_modules: Vec<ModuleId>,
    /// Declared library symbol sources for this profile.
    pub declared_library_symbol_sources: IndexMap<LibrarySymbolKey, Vec<GlobalSymbolId>>,
    /// Selected library symbol sources for this profile.
    pub library_symbol_sources: IndexMap<LibrarySymbolKey, Vec<GlobalSymbolId>>,
}

impl LibraryEnvironment {
    /// Build one canonical name key for library symbol lookup.
    fn library_name_key(name: &str) -> CanonicalStaticKey {
        CanonicalStaticKey::Name(name.to_string())
    }

    /// Collect ordered candidates for one key and space order.
    fn collect_symbol_sources_for_space_order(
        &self,
        key: &CanonicalStaticKey,
        order: SymbolSpaceOrder,
        declared_only: bool,
    ) -> Vec<GlobalSymbolId> {
        let mut sources = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut push_sources = |space| {
            let group = if declared_only {
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

    /// Return declared library symbol sources for one key and space.
    pub fn declared_symbol_sources(
        &self,
        key: &CanonicalStaticKey,
        space: SymbolSpace,
    ) -> Option<&[GlobalSymbolId]> {
        self.declared_library_symbol_sources
            .get(&LibrarySymbolKey {
                key: key.clone(),
                space,
            })
            .map(Vec::as_slice)
    }

    /// Return selected library symbol sources for one key and space.
    pub fn symbol_sources(
        &self,
        key: &CanonicalStaticKey,
        space: SymbolSpace,
    ) -> Option<&[GlobalSymbolId]> {
        self.library_symbol_sources
            .get(&LibrarySymbolKey {
                key: key.clone(),
                space,
            })
            .map(Vec::as_slice)
    }

    /// Return one declared symbol using semantic selection.
    pub fn declared_symbol_from(
        &self,
        name: &str,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let key = Self::library_name_key(name);
        self.declared_symbol_from_key(&key, order)
    }

    /// Return one declared symbol using semantic selection for a canonical key.
    pub fn declared_symbol_from_key(
        &self,
        key: &CanonicalStaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let sources = self.collect_symbol_sources_for_space_order(key, order, true);
        sources.first().copied()
    }

    /// Return one declared symbol using concrete runtime selection.
    pub fn declared_concrete_symbol_from(
        &self,
        name: &str,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let key = Self::library_name_key(name);
        self.declared_concrete_symbol_from_key(&key, order)
    }

    /// Return one declared symbol using concrete selection for a canonical key.
    pub fn declared_concrete_symbol_from_key(
        &self,
        key: &CanonicalStaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let sources = self.collect_symbol_sources_for_space_order(key, order, true);
        Self::first_concrete_symbol(&sources)
    }

    /// Return one selected symbol using semantic selection.
    pub fn symbol_from(&self, name: &str, order: SymbolSpaceOrder) -> Option<GlobalSymbolId> {
        let key = Self::library_name_key(name);
        self.symbol_from_key(&key, order)
    }

    /// Return one selected symbol using semantic selection for a canonical key.
    pub fn symbol_from_key(
        &self,
        key: &CanonicalStaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let sources = self.collect_symbol_sources_for_space_order(key, order, false);
        sources.first().copied()
    }

    /// Return selected symbol sources using a space order for a canonical key.
    pub fn symbol_sources_for_space_order(
        &self,
        key: &CanonicalStaticKey,
        order: SymbolSpaceOrder,
    ) -> Vec<GlobalSymbolId> {
        self.collect_symbol_sources_for_space_order(key, order, false)
    }

    /// Build the derived well-known symbol view from cached library facts.
    pub fn well_known_symbols(&self) -> WellKnownSymbols {
        let mut symbols = IndexMap::new();
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

#[cfg(test)]
mod tests {
    use super::*;
    use destack_core::StringPool;
    use destack_dir::{LocalSymbolId, SymbolType};
    use destack_source::{ModuleId, PackageId};

    /// Canonical static keys compare by string content across pools.
    #[test]
    fn test_canonical_static_key_ignores_string_pool_identity() {
        let left_strings = StringPool::new();
        let right_strings = StringPool::new();

        let left_name = left_strings.intern("value");
        let right_name = right_strings.intern("value");
        let left_key =
            CanonicalStaticKey::from_static_key(StaticKey::Name(left_name), &left_strings);
        let right_key =
            CanonicalStaticKey::from_static_key(StaticKey::Name(right_name), &right_strings);

        assert_eq!(left_key, right_key);
    }

    /// Canonical symbol registry keys compare by string content across pools.
    #[test]
    fn test_canonical_symbol_key_ignores_string_pool_identity() {
        let left_strings = StringPool::new();
        let right_strings = StringPool::new();

        let left_name = left_strings.intern("shared");
        let right_name = right_strings.intern("shared");
        let left_key =
            CanonicalSymbolKey::from_symbol_key(SymbolKey::Registry(left_name), &left_strings);
        let right_key =
            CanonicalSymbolKey::from_symbol_key(SymbolKey::Registry(right_name), &right_strings);

        assert_eq!(left_key, right_key);
    }

    /// Library environments resolve names and sources without ambient string ids.
    #[test]
    fn test_library_environment_uses_owned_names() {
        let module_id = ModuleId::new(PackageId::new(1), 2);
        let symbol_id = LocalSymbolId::new_typed(3, SymbolType::Void).into_global(module_id);
        let source_key = CanonicalStaticKey::Name("value".to_string());
        let environment = LibraryEnvironment {
            modules: vec![module_id],
            ambient_modules: vec![],
            declared_library_symbol_sources: IndexMap::from([(
                LibrarySymbolKey {
                    key: source_key.clone(),
                    space: SymbolSpace::Value,
                },
                vec![symbol_id],
            )]),
            library_symbol_sources: IndexMap::from([(
                LibrarySymbolKey {
                    key: source_key.clone(),
                    space: SymbolSpace::Value,
                },
                vec![symbol_id],
            )]),
        };

        assert_eq!(
            environment.declared_symbol_sources(&source_key, SymbolSpace::Value),
            Some([symbol_id].as_slice())
        );
        assert_eq!(
            environment.symbol_sources(&source_key, SymbolSpace::Value),
            Some([symbol_id].as_slice())
        );
    }
}
