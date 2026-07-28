use crate::GlobalSymbolId;
use destack_core::{StringId, StringPool};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Symbol as a key.
#[derive(
    Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Ord, Eq, Serialize, Deserialize, Reflect,
)]
pub enum SymbolKey {
    /// Unique symbol key from a declaration.
    Unique(GlobalSymbolId),
    /// Symbol.for registry key (string is the content of `Symbol.for`).
    Registry(StringId),
}

/// Key for some static "identifier" (name, positional index, symbol).
#[derive(
    Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Ord, Eq, Serialize, Deserialize, Reflect,
)]
pub enum StaticKey {
    /// Regular name key (like `x` or `"weird identifier"`).
    Name(StringId),
    /// Positional index key (like `0` or `1`).
    Index(usize),
    /// Symbol key.
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
            StaticKey::Index(_) => None,
            StaticKey::Symbol(_) => None,
        }
    }

    /// Check if two keys are equivalent for structural matching.
    pub fn matches(&self, other: &StaticKey) -> bool {
        match (self, other) {
            (StaticKey::Name(left), StaticKey::Name(right)) => left == right,
            (StaticKey::Index(left), StaticKey::Index(right)) => left == right,
            (StaticKey::Symbol(left), StaticKey::Symbol(right)) => left == right,
            _ => false,
        }
    }

    /// Return true when this key behaves as a string like key.
    pub fn is_string_like(&self) -> bool {
        matches!(self, StaticKey::Name(_))
    }

    /// Return true when this key behaves as a number like key.
    pub fn is_number_like(&self) -> bool {
        matches!(self, StaticKey::Index(_))
    }

    /// Return true when this key behaves as a symbol like key.
    pub fn is_symbol_like(&self) -> bool {
        matches!(self, StaticKey::Symbol(_))
    }

    /// Return whether this exact key stores directly in one primitive carrier.
    pub fn widens_to_primitive(&self, primitive: crate::PrimitiveType) -> bool {
        match primitive {
            crate::PrimitiveType::String => self.is_string_like(),
            crate::PrimitiveType::Symbol | crate::PrimitiveType::UniqueSymbol => {
                self.is_symbol_like()
            }
            // index keys are exact values and must fit the integer width
            crate::PrimitiveType::Integer(integer) => match self {
                StaticKey::Index(index) => {
                    i64::try_from(*index).is_ok_and(|value| integer.fits_literal(value))
                }
                _ => false,
            },
            crate::PrimitiveType::Boolean
            | crate::PrimitiveType::Character
            | crate::PrimitiveType::Float(_)
            | crate::PrimitiveType::Bigint => false,
        }
    }

    /// Get the debug string given a mutable string pool.
    pub fn debug_string(&self, strings: &StringPool) -> String {
        match self {
            StaticKey::Name(name) => {
                format!("'{}'", strings.get(*name))
            }
            StaticKey::Index(index) => format!("#{index}"),
            StaticKey::Symbol(symbol) => symbol.debug_string(strings),
        }
    }
}

impl SymbolKey {
    /// Get the debug string given a mutable string pool.
    pub fn debug_string(&self, strings: &StringPool) -> String {
        match self {
            SymbolKey::Unique(_) => "<unique symbol>".to_string(),
            SymbolKey::Registry(name) => {
                let name = strings.get(*name);
                format!("'Symbol.for(\"{name}\")'")
            }
        }
    }
}
