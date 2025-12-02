use crate::{LocalNodeIdAny, Program};
use destack_source::StringId;

/// Key for a symbol / some static "identifier".
#[derive(Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Eq)]
pub enum StaticKey {
    /// Regular name key (like `x` or `"weird identifier"`).
    Name(StringId),
    /// Unique symbol expression (like `const x = Symbol("x");`).
    UniqueSymbol(LocalNodeIdAny),
    /// Global symbol key (like `Symbol.iterator`).
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
            StaticKey::UniqueSymbol(..) => None,
            StaticKey::GlobalSymbol(name) => Some(*name),
        }
    }

    /// Get the debug string in a given program.
    pub fn debug_string(&self, program: &Program) -> String {
        match self {
            StaticKey::Name(name) => {
                format!("'{}'", program.strings.get(*name).as_str()).to_string()
            }
            StaticKey::UniqueSymbol(..) => "<unique symbol>".to_string(),
            StaticKey::GlobalSymbol(name) => {
                format!("'{}'", program.strings.get(*name).as_str()).to_string()
            }
        }
    }
}
