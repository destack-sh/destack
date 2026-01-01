use crate::LocalNodeIdAny;
use destack_base::{StringId, StringPool};

/// Key for a symbol / some static "identifier".
#[derive(Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Eq)]
pub enum StaticKey {
    /// Regular name key (like `x` or `"weird identifier"`).
    Name(StringId),
    /// Numeric name key (like `1` or `1e3`).
    Number(StringId),
    /// Unique symbol expression (like `const x = Symbol("x");`).
    UniqueSymbol(LocalNodeIdAny),
    /// Global symbol key (like `Symbol.iterator`).
    /// FUGU: wire WellKnownSymbol into StaticKey?
    GlobalSymbol(StringId),
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
            StaticKey::UniqueSymbol(..) => None,
            StaticKey::GlobalSymbol(name) => Some(*name),
        }
    }

    /// Check if two keys are equivalent for structural matching.
    pub fn matches(&self, other: &StaticKey) -> bool {
        match (self, other) {
            (StaticKey::Name(left), StaticKey::Name(right)) => left == right,
            (StaticKey::Number(left), StaticKey::Number(right)) => left == right,
            (StaticKey::Name(left), StaticKey::Number(right)) => left == right,
            (StaticKey::Number(left), StaticKey::Name(right)) => left == right,
            _ => self == other,
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
        matches!(
            self,
            StaticKey::UniqueSymbol(_) | StaticKey::GlobalSymbol(_)
        )
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
            StaticKey::UniqueSymbol(..) => "<unique symbol>".to_string(),
            StaticKey::GlobalSymbol(name) => {
                format!("'{}'", &*strings.get(*name))
            }
        }
    }
}
