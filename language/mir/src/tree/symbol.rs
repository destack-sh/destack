use destack_core::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{GenericArgument, Tree};

use super::fingerprint::TypeHasher;

/// Persistent, mangled identity of a function, global, or type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct Symbol(u64);

impl Symbol {
    /// Create the symbol of one resolved declaration name.
    pub fn named(name: StringId) -> Self {
        Self(name.raw())
    }

    /// Create the symbol of one declaration, distinguished by its declaring identity.
    pub fn declared(name: StringId, identity: u64) -> Self {
        TypeHasher::declared(name, identity)
    }

    /// Derive one generic instance symbol from its concrete arguments.
    pub fn instantiate(self, arguments: &[GenericArgument], tree: &Tree) -> Self {
        TypeHasher::symbol(self, arguments, tree)
    }

    /// Return the stable symbol bits.
    pub fn raw(self) -> u64 {
        self.0
    }

    /// Restore a symbol from its persistent bits.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}
