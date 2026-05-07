use crate::{GlobalSymbolId, WellKnownSymbolKey};
use destack_core::{StringId, StringPool};
use serde::{Deserialize, Serialize};

/// Symbol as a key.
#[derive(Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Eq, Serialize, Deserialize)]
pub enum SymbolKey {
    /// Unique symbol key from a declaration.
    Unique(GlobalSymbolId),
    /// Well known Symbol.* key.
    WellKnown(WellKnownSymbolKey),
    /// Symbol.for registry key (string is the content of `Symbol.for`).
    Registry(StringId),
}

/// Key for some static "identifier" (name, numeric, symbol).
#[derive(Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Eq, Serialize, Deserialize)]
pub enum StaticKey {
    /// Regular name key (like `x` or `"weird identifier"`).
    Name(StringId),
    /// Numeric name key (like `1` or `1e3`).
    Number(StringId),
    /// Symbol key (unique, well known, registry).
    Symbol(SymbolKey),
}

impl From<StringId> for StaticKey {
    fn from(name: StringId) -> Self {
        StaticKey::Name(name)
    }
}

impl StaticKey {
    /// Get the name of the symbol key.
    pub fn name(&self) -> Option<StringId> {
        match self {
            StaticKey::Name(name) => Some(*name),
            StaticKey::Number(name) => Some(*name),
            StaticKey::Symbol(_) => None,
        }
    }

    /// Check if two keys are equivalent for structural matching.
    pub fn matches(&self, other: &StaticKey) -> bool {
        match (self, other) {
            (StaticKey::Name(left), StaticKey::Name(right)) => left == right,
            (StaticKey::Number(left), StaticKey::Number(right)) => left == right,
            (StaticKey::Name(left), StaticKey::Number(right)) => left == right,
            (StaticKey::Number(left), StaticKey::Name(right)) => left == right,
            (StaticKey::Symbol(left), StaticKey::Symbol(right)) => left == right,
            _ => false,
        }
    }

    /// Return true when this key behaves as a string like key.
    pub fn is_string_like(&self) -> bool {
        matches!(self, StaticKey::Name(_) | StaticKey::Number(_))
    }

    /// Return true when this key behaves as a number like key.
    pub fn is_number_like(&self) -> bool {
        matches!(self, StaticKey::Number(_))
    }

    /// Return true when this key behaves as a symbol like key.
    pub fn is_symbol_like(&self) -> bool {
        matches!(self, StaticKey::Symbol(_))
    }

    /// Get the debug string given a mutable string pool.
    pub fn debug_string(&self, strings: &StringPool) -> String {
        match self {
            StaticKey::Name(name) => {
                format!("'{}'", &*strings.get(*name))
            }
            StaticKey::Number(name) => {
                format!("'{}'", &*strings.get(*name))
            }
            StaticKey::Symbol(symbol) => symbol.debug_string(strings),
        }
    }
}

impl SymbolKey {
    /// Get the debug string given a mutable string pool.
    pub fn debug_string(&self, strings: &StringPool) -> String {
        match self {
            SymbolKey::Unique(_) => "<unique symbol>".to_string(),
            SymbolKey::WellKnown(symbol) => {
                format!("'{}'", symbol.global_symbol_name())
            }
            SymbolKey::Registry(name) => {
                let name = &*strings.get(*name);
                format!("'Symbol.for(\"{name}\")'")
            }
        }
    }
}
