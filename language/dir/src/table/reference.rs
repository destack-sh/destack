use destack_serde::Reflect;
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{GlobalNodeIdAny, GlobalSymbolId};

/// Name resolutions for one module, keyed by the reference node.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ReferenceTable {
    /// The module id of the reference table.
    pub module_id: ModuleId,
    /// Resolved references keyed by their source node.
    pub entries: IndexMap<GlobalNodeIdAny, Reference>,
}

impl ReferenceTable {
    /// Create an empty reference table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            entries: IndexMap::new(),
        }
    }

    /// Insert one resolved reference.
    pub fn insert(&mut self, node: GlobalNodeIdAny, reference: Reference) {
        self.entries.insert(node, reference);
    }

    /// Return one resolved reference.
    pub fn get(&self, node: GlobalNodeIdAny) -> Option<&Reference> {
        self.entries.get(&node)
    }

    /// Return true when no references were resolved.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// How one source reference resolves by name, before types and conditions apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Reference {
    /// Resolved to declarations by name: lexical scope or a full namespace path.
    /// A set carries overloads, narrowed by availability and dispatch in check.
    Bound(SmallVec<[GlobalSymbolId; 2]>),
    /// Resolved to a namespace: a prefix awaiting a further segment, or a bare namespace value.
    Namespace(ModuleId),
    /// A flat path named through its first segments; `segments[from..]` project as members off `base`.
    Projected {
        /// The declaration the leading segments name.
        base: GlobalSymbolId,
        /// The segment index where member projection begins.
        from: u32,
    },
    /// Conflicting bindings with no single winner.
    Ambiguous(SmallVec<[GlobalSymbolId; 2]>),
    /// No binding by name.
    Missing,
}
