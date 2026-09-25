use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tspp_serde::Reflect;

use crate::{GlobalSymbolId, StaticKey};

/// Root of one value access path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AccessRoot {
    /// A source binding symbol.
    Symbol(GlobalSymbolId),
    /// The contextual receiver instance, named by `this` and by `super`.
    Receiver,
}

/// Structural path to one value access.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct AccessPath {
    /// The root value.
    root: AccessRoot,
    /// The keys projected from the root.
    keys: SmallVec<[StaticKey; 2]>,
}

impl AccessPath {
    /// Create a path rooted at one binding symbol.
    pub fn symbol(symbol: GlobalSymbolId) -> Self {
        Self {
            root: AccessRoot::Symbol(symbol),
            keys: SmallVec::new(),
        }
    }

    /// Create a path rooted at the contextual receiver instance.
    pub fn receiver() -> Self {
        Self {
            root: AccessRoot::Receiver,
            keys: SmallVec::new(),
        }
    }

    /// Return the root value.
    pub fn root(&self) -> AccessRoot {
        self.root
    }

    /// Return the projected keys.
    pub fn keys(&self) -> &[StaticKey] {
        &self.keys
    }

    /// Append one projected key.
    pub fn push(&mut self, key: StaticKey) {
        self.keys.push(key);
    }

    /// Return whether this access is below another access.
    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.root == prefix.root && self.keys.starts_with(&prefix.keys)
    }

    /// Return this access split into its parent and final key.
    pub fn split_last(&self) -> Option<(Self, StaticKey)> {
        let key = self.keys.last().copied()?;
        let mut parent = self.clone();

        parent.keys.pop();

        Some((parent, key))
    }
}

/// Stable storage access selected by check.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct AccessResolution {
    /// The selected structural path.
    path: AccessPath,
}

impl AccessResolution {
    /// Record one selected structural path.
    pub fn new(path: AccessPath) -> Self {
        Self { path }
    }

    /// Return the selected structural path.
    pub fn path(&self) -> &AccessPath {
        &self.path
    }
}
