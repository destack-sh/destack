use destack_core::StringId;
use serde::{Deserialize, Serialize};

/// Persistent, mangled identity of a function, global, or type.
///
/// Unique and stable across builds, minted from the resolved path during
/// lowering. MIR carries it but does not compute it; cross-module references
/// link by symbol equality, and analyses and profile data key on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Symbol(pub StringId);

impl Symbol {
    /// The interned symbol text.
    pub fn text(self) -> StringId {
        self.0
    }
}
