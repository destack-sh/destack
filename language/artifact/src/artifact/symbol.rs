use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace, WellKnownSymbol, WellKnownSymbolKey};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Type and value symbols for one well-known surface name.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SymbolPair {
    /// The type-space symbol, if any.
    pub ty: Option<GlobalSymbolId>,
    /// The value-space symbol, if any.
    pub value: Option<GlobalSymbolId>,
}

/// A lookup key for selected global environment symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GlobalEnvironmentKey {
    /// The static symbol key.
    pub key: StaticKey,
    /// The symbol space.
    pub space: SymbolSpace,
}

impl SymbolPair {
    /// Create a new symbol pair from a type symbol.
    pub fn from_type(symbol: GlobalSymbolId) -> Self {
        Self {
            ty: Some(symbol),
            value: None,
        }
    }

    /// Create a new symbol pair from a value symbol.
    pub fn from_value(symbol: GlobalSymbolId) -> Self {
        Self {
            ty: None,
            value: Some(symbol),
        }
    }

    /// Create a new symbol pair from one type and value symbol.
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
}

/// Well-known symbol key metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellKnownKey {
    /// The base symbol for the key.
    pub symbol: GlobalSymbolId,
    /// The member name on the base symbol.
    pub member: String,
    /// The full global key name.
    pub global_name: String,
}

/// Resolved compiler-known symbols for a profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WellKnownSymbols {
    /// Top-level builtin symbols by well-known id.
    pub symbols: IndexMap<WellKnownSymbol, SymbolPair>,
    /// Well-known symbol keys by id.
    pub keys: IndexMap<WellKnownSymbolKey, WellKnownKey>,
}

impl WellKnownSymbols {
    /// Get the symbol pair for a well-known symbol.
    pub fn get_pair(&self, item: WellKnownSymbol) -> Option<SymbolPair> {
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
