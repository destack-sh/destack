use std::collections::VecDeque;

use dashmap::DashMap;
use destack_base::StringId;
use destack_dir::{Declaration, GlobalSymbolId, LocalNodeId, StaticKey, SymbolSpace};
use destack_source::{ModuleId, ModuleVersion};
use indexmap::IndexMap;

use crate::{ProfileId, TargetId};

/// Derived indexes and tables for a Program.
#[derive(Debug, Default)]
pub struct ProgramIndex {
    /// Global symbol tables indexed by target and profile.
    pub global_symbol_tables: DashMap<GlobalSymbolTableKey, GlobalSymbolTable>,
    /// Module binding tables indexed by target and profile.
    pub module_binding_tables: DashMap<ModuleBindingTableKey, ModuleBindingTable>,
}

impl ProgramIndex {
    /// Create a new ProgramIndex.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Identify a global symbol table view for a target and profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlobalSymbolTableKey {
    /// Target id for the module selection.
    pub target_id: TargetId,
    /// Profile id for the compilation.
    pub profile_id: ProfileId,
    /// Entry module id when target discovery is implicit.
    pub entry_module: Option<ModuleId>,
}

/// Track global symbols from declare global blocks reachable from a root set.
#[derive(Debug, Clone)]
pub struct GlobalSymbolTable {
    /// Versions for modules included in the table.
    pub module_versions: IndexMap<ModuleId, ModuleVersion>,
    /// First symbol observed for each global key.
    pub symbols: IndexMap<StaticKey, GlobalSymbolId>,
    /// First symbol observed for each global key and space.
    pub symbols_by_space: IndexMap<GlobalSymbolGroupKey, GlobalSymbolId>,
    /// All symbols observed for each global key.
    pub sources: IndexMap<StaticKey, Vec<GlobalSymbolId>>,
    /// All symbols observed for each global key and space.
    pub sources_by_space: IndexMap<GlobalSymbolGroupKey, Vec<GlobalSymbolId>>,
    /// Modules remaining to process (empty once complete).
    pub pending: VecDeque<ModuleId>,
}

impl Default for GlobalSymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::too_many_arguments)]
impl GlobalSymbolTable {
    /// Create an empty table.
    pub fn new() -> Self {
        Self {
            module_versions: IndexMap::new(),
            symbols: IndexMap::new(),
            symbols_by_space: IndexMap::new(),
            sources: IndexMap::new(),
            sources_by_space: IndexMap::new(),
            pending: VecDeque::new(),
        }
    }

    /// Check if the table is complete (no pending modules).
    pub fn is_complete(&self) -> bool {
        self.pending.is_empty()
    }

    /// Insert a global symbol and preserve the first binding for the key.
    pub fn insert_symbol(&mut self, key: StaticKey, space: SymbolSpace, symbol: GlobalSymbolId) {
        self.sources.entry(key).or_default().push(symbol);
        self.symbols.entry(key).or_insert(symbol);

        let group_key = GlobalSymbolGroupKey { key, space };
        self.sources_by_space
            .entry(group_key)
            .or_default()
            .push(symbol);
        self.symbols_by_space.entry(group_key).or_insert(symbol);
    }
}

/// Key for grouping global symbols by name and space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalSymbolGroupKey {
    /// The symbol key.
    pub key: StaticKey,
    /// The symbol space.
    pub space: SymbolSpace,
}

/// Identify a module binding table view for a target and profile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleBindingTableKey {
    /// Target id for the module selection.
    pub target_id: TargetId,
    /// Profile id for the compilation.
    pub profile_id: ProfileId,
    /// Entry module id when target discovery is implicit.
    pub entry_module: Option<ModuleId>,
}

/// Reference a module binding declaration in a module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModuleBindingReference {
    /// The module id that owns the binding.
    pub module_id: ModuleId,
    /// The declaration node for the binding.
    pub declaration: LocalNodeId<Declaration>,
}

/// Track module bindings reachable from a root set.
#[derive(Debug, Clone)]
pub struct ModuleBindingTable {
    /// Versions for modules included in the index.
    pub module_versions: IndexMap<ModuleId, ModuleVersion>,
    /// Module bindings by specifier.
    pub bindings_by_specifier: IndexMap<StringId, Vec<ModuleBindingReference>>,
}

impl Default for ModuleBindingTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleBindingTable {
    /// Create an empty table seeded with roots.
    pub fn new() -> Self {
        Self {
            module_versions: IndexMap::new(),
            bindings_by_specifier: IndexMap::new(),
        }
    }
}
