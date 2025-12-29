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
    /// FUGU: support (and use?) intrinsic/well-known Symbols (see `symbol.wellknown.d.ds`)
    /// (and maybe also use intrinsic symbols for intrinsic operator intefaces..? Symbol.add, ..
    ///  .. should we make the wellknown symbols language items directly..?)
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
