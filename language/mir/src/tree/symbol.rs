use destack_core::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{StaticId, Tree};

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

    /// Derive one generic instance symbol from its concrete arguments.
    pub fn instantiate(self, arguments: &[StaticId], tree: &Tree) -> Self {
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
