use std::fmt::Display;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tspp_core::FxIndexMap as IndexMap;
use tspp_source::ModuleId;

use crate::{LocalSymbolId, StaticKey};

const SMALL_SCOPE_LOOKUP_KEYS: usize = 8;

/// A lexical container for symbols.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Scope {
    /// The kind of the scope.
    pub kind: ScopeKind,
    /// The parent scope.
    pub parent: Option<LocalScope>,
    /// The owner of the scope.
    pub owner: Option<LocalSymbolId>,

    /// The bindings in lexical order.
    pub bindings: Vec<ScopeBinding>,

    /// Index for named bindings.
    pub index: ScopeIndex,

    /// The children scopes.
    pub children: Vec<LocalScopeId>,
}

impl Scope {
    /// Whether the scope is the root scope.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Get the current scope mark.
    pub fn mark(&self) -> LocalScopeMark {
        LocalScopeMark(self.bindings.len() as u32)
    }

    /// Insert a symbol into the scope.
    pub fn append(&mut self, key: Option<StaticKey>, symbol_id: LocalSymbolId) -> LocalScopeMark {
        let mark = LocalScopeMark(self.bindings.len() as u32);
        self.bindings.push(ScopeBinding {
            key,
            symbol: symbol_id,
        });
        if let Some(key) = key {
            self.index.insert(key, mark.0);
        }

        mark
    }

    /// Insert a child scope into the scope.
    pub fn append_child(&mut self, scope_id: LocalScopeId) {
        self.children.push(scope_id);
    }

    /// Iterate named symbols in lexical order.
    pub fn named_symbols(&self) -> impl Iterator<Item = (StaticKey, LocalSymbolId)> + '_ {
        self.bindings
            .iter()
            .filter_map(|binding| binding.key.map(|key| (key, binding.symbol)))
    }

    /// Iterate named symbols in lexical order up to a mark.
    pub fn named_symbols_up_to(
        &self,
        mark: LocalScopeMark,
    ) -> impl Iterator<Item = (StaticKey, LocalSymbolId)> + '_ {
        let limit = mark.0 as usize;

        self.bindings
            .iter()
            .take(limit)
            .filter_map(|binding| binding.key.map(|key| (key, binding.symbol)))
    }

    /// Visit symbols bound by one key in lexical order.
    pub fn for_symbols_by_key(&self, key: StaticKey, mut visit: impl FnMut(LocalSymbolId)) {
        self.index.for_indices(key, LocalScopeMark::end(), |index| {
            let symbol = self.bindings[index as usize].symbol;

            visit(symbol);
        });
    }

    /// Visit symbols bound by one key in lexical order up to a mark.
    pub fn for_symbols_by_key_up_to(
        &self,
        key: StaticKey,
        mark: LocalScopeMark,
        mut visit: impl FnMut(LocalSymbolId),
    ) {
        self.index.for_indices(key, mark, |index| {
            let symbol = self.bindings[index as usize].symbol;

            visit(symbol);
        });
    }

    /// Iterate anonymous symbols in lexical order.
    pub fn anonymous_symbols(&self) -> impl Iterator<Item = LocalSymbolId> + '_ {
        self.bindings
            .iter()
            .filter_map(|binding| binding.key.is_none().then_some(binding.symbol))
    }

    /// Find the latest symbol with one key.
    pub fn find_symbol(&self, key: StaticKey) -> Option<LocalSymbolId> {
        let index = self.index.last_index(key, LocalScopeMark::end())?;

        Some(self.bindings[index as usize].symbol)
    }

    /// Find the latest symbol with one key up to a mark.
    pub fn find_symbol_up_to(&self, key: StaticKey, mark: LocalScopeMark) -> Option<LocalSymbolId> {
        let index = self.index.last_index(key, mark)?;

        Some(self.bindings[index as usize].symbol)
    }
}

/// A compact name index for one lexical scope.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ScopeIndex {
    /// No named bindings.
    #[default]
    Empty,
    /// A small inline table for common tiny scopes.
    Small {
        /// Binding indices grouped by key.
        entries: Vec<ScopeIndexEntry>,
    },
    /// A keyed table for larger scopes.
    Large {
        /// Binding indices grouped by key.
        table: Box<IndexMap<StaticKey, SmallVec<[u32; 1]>>>,
    },
}

impl ScopeIndex {
    /// Insert one binding index.
    fn insert(&mut self, key: StaticKey, index: u32) {
        match self {
            ScopeIndex::Empty => {
                let mut entries = Vec::with_capacity(SMALL_SCOPE_LOOKUP_KEYS);
                entries.push(ScopeIndexEntry::new(key, index));

                *self = ScopeIndex::Small { entries };
            }
            ScopeIndex::Small { entries } => {
                if let Some(entry) = entries.iter_mut().find(|entry| entry.key == key) {
                    entry.indices.push(index);

                    return;
                }

                if entries.len() < SMALL_SCOPE_LOOKUP_KEYS {
                    entries.push(ScopeIndexEntry::new(key, index));

                    return;
                }

                // promote large scopes to keyed lookup
                let mut table =
                    IndexMap::with_capacity_and_hasher(entries.len() + 1, Default::default());
                for entry in entries.drain(..) {
                    table.insert(entry.key, entry.indices);
                }
                table.insert(key, SmallVec::from_buf([index]));

                *self = ScopeIndex::Large {
                    table: Box::new(table),
                };
            }
            ScopeIndex::Large { table } => {
                table.entry(key).or_default().push(index);
            }
        }
    }

    /// Visit indices bound by one key in lexical order up to a mark.
    fn for_indices(&self, key: StaticKey, mark: LocalScopeMark, mut visit: impl FnMut(u32)) {
        let Some(indices) = self.indices(key) else {
            return;
        };

        for index in indices {
            if *index >= mark.0 {
                break;
            }

            visit(*index);
        }
    }

    /// Return the latest index for one key before a mark.
    fn last_index(&self, key: StaticKey, mark: LocalScopeMark) -> Option<u32> {
        let indices = self.indices(key)?;
        let index = indices.iter().rev().find(|index| **index < mark.0)?;

        Some(*index)
    }

    /// Return indices for one key.
    fn indices(&self, key: StaticKey) -> Option<&SmallVec<[u32; 1]>> {
        match self {
            ScopeIndex::Empty => None,
            ScopeIndex::Small { entries } => entries
                .iter()
                .find(|entry| entry.key == key)
                .map(|entry| &entry.indices),
            ScopeIndex::Large { table } => table.get(&key),
        }
    }
}

/// Binding indices for one key in a small scope lookup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ScopeIndexEntry {
    /// The binding key.
    pub key: StaticKey,
    /// Binding indices in lexical order.
    pub indices: SmallVec<[u32; 1]>,
}

impl ScopeIndexEntry {
    /// Create one small lookup entry.
    fn new(key: StaticKey, index: u32) -> Self {
        Self {
            key,
            indices: SmallVec::from_buf([index]),
        }
    }
}

/// The kind of a scope.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ScopeKind {
    /// Module root.
    Module,
    /// Global declaration contribution.
    Global,
    /// Namespace declaration or object declaration surface.
    Namespace,
    /// Function body and parameter surface.
    Function,
    /// Type expression or declaration surface.
    Type,
    /// Conditional type infer scope.
    TypeConditional,
    /// Block expression or statement surface.
    Block,
}

/// Unique identifier for local scopes.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct LocalScopeId(pub u32);

impl LocalScopeId {
    /// Wrap an id as a ScopeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a GlobalScopeId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalScopeId {
        GlobalScopeId {
            module_id,
            local_id: self,
        }
    }
}

impl Display for LocalScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl Display for LocalScopeMark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == u32::MAX {
            write!(f, ".END")
        } else {
            write!(f, ".{}", self.0)
        }
    }
}

/// Global scope id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct GlobalScopeId {
    /// The module id of the global scope.
    pub module_id: ModuleId,
    /// The local id of the global scope.
    pub local_id: LocalScopeId,
}

impl GlobalScopeId {
    /// Create a new global scope id.
    pub fn new(module_id: ModuleId, local_id: LocalScopeId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a LocalScopeId.
    #[inline]
    pub fn into_local(self) -> LocalScopeId {
        self.local_id
    }
}

impl From<GlobalScopeId> for LocalScopeId {
    fn from(id: GlobalScopeId) -> Self {
        id.local_id
    }
}

/// Mark a position in a scope.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
#[repr(transparent)]
pub struct LocalScopeMark(pub u32);

/// Local scope id and mark pair used for node and symbol insertion.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct LocalScope {
    /// The scope id.
    pub id: LocalScopeId,
    /// The visible binding mark.
    pub mark: LocalScopeMark,
}

impl LocalScope {
    /// Create a local scope cursor.
    #[inline]
    pub fn new(id: LocalScopeId, mark: LocalScopeMark) -> Self {
        Self { id, mark }
    }
}

impl LocalScopeMark {
    /// Get the full scope view.
    pub fn end() -> Self {
        Self(u32::MAX)
    }
}

/// One binding entry in lexical order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ScopeBinding {
    /// The binding key.
    pub key: Option<StaticKey>,
    /// The bound symbol.
    pub symbol: LocalSymbolId,
}
