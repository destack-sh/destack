use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::GlobalSymbolId;

/// Unique identifier for Lineages.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalLineageId(pub u32);

impl LocalLineageId {
    /// Wrap an id as a LineageId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a GlobalLineageId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalLineageId {
        GlobalLineageId {
            module_id,
            local_id: self,
        }
    }
}

/// Global lineage id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalLineageId {
    /// The module id of the global lineage.
    pub module_id: ModuleId,
    /// The local id of the global lineage.
    pub local_id: LocalLineageId,
}

impl GlobalLineageId {
    /// Create a new global lineage id.
    pub fn new(module_id: ModuleId, local_id: LocalLineageId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a LocalLineageId.
    pub fn into_local(self) -> LocalLineageId {
        self.local_id
    }
}

impl From<GlobalLineageId> for LocalLineageId {
    fn from(id: GlobalLineageId) -> Self {
        id.local_id
    }
}

impl Display for LocalLineageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "^{}", self.0)
    }
}

/// The resolved inheritance lineage of a nominal type.
///
/// ### Example
///
/// ```text
/// interface Printable { ... }
/// class Animal { ... }
/// class Dog extends Animal implements Printable { ... }
/// ```
///
/// The lineage for `Dog` would be:
/// - `extends: Some(AnimalSymbol)`
/// - `implements: [PrintableSymbol]`
/// - `embedded: []`
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Lineage {
    /// The extended parent type (single inheritance for classes).
    pub extends: Option<GlobalSymbolId>,
    /// The implemented interface types.
    pub implements: Vec<GlobalSymbolId>,
    /// The embedded/composed types (for struct-like types).
    pub embedded: Vec<GlobalSymbolId>,
}

impl Lineage {
    /// Create a new empty lineage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check whether the lineage has any relationships.
    pub fn is_empty(&self) -> bool {
        self.extends.is_none() && self.implements.is_empty() && self.embedded.is_empty()
    }

    /// Check if this type directly extends the given symbol.
    pub fn directly_extends(&self, symbol: GlobalSymbolId) -> bool {
        self.extends == Some(symbol)
    }

    /// Check if this type directly implements the given symbol.
    pub fn directly_implements(&self, symbol: GlobalSymbolId) -> bool {
        self.implements.contains(&symbol)
    }

    /// Check if this type directly embeds the given symbol.
    pub fn directly_embeds(&self, symbol: GlobalSymbolId) -> bool {
        self.embedded.contains(&symbol)
    }
}
