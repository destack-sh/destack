use std::collections::VecDeque;

use dashmap::DashMap;
use destack_core::StringId;
use destack_dir::{
    Declaration, ExtensionKind, GlobalSymbolId, LocalExtensionId, LocalNodeId, LocalSymbolId,
    StaticKey, SymbolSpace, SymbolType,
};
use destack_source::{ModuleId, ModuleVersion};
use indexmap::IndexMap;

use crate::{
    ModuleGraph, ModuleGraphKey, ModuleGraphVersion, ModuleSignature, ModuleSignatureDigest,
    ModuleSignatureKey, ProfileId, TargetId,
};
use destack_source::PackageId;

/// Derived indexes and tables for a Program.
#[derive(Debug, Default)]
pub struct ProgramIndex {
    /// Global symbol tables indexed by target and profile.
    pub global_symbol_tables: DashMap<GlobalSymbolTableKey, GlobalSymbolTable>,
    /// Module binding registry indexed by package.
    pub module_binding_registry: DashMap<PackageId, ModuleBindingRegistry>,
    /// Module binding tables indexed by target and profile.
    pub module_binding_tables: DashMap<ModuleBindingTableKey, ModuleBindingTable>,
    /// Module graphs indexed by profile.
    pub module_graphs: DashMap<ModuleGraphKey, ModuleGraph>,
    /// Module graph versions indexed by profile.
    pub module_graph_versions: DashMap<ProfileId, ModuleGraphVersion>,
    /// Module signatures indexed by module and profile.
    pub module_signatures: DashMap<ModuleSignatureKey, ModuleSignature>,
    /// Module signature digests indexed by module and profile.
    pub module_signature_digests: DashMap<ModuleSignatureKey, ModuleSignatureDigest>,
    /// Extension index for O(1) extension lookups.
    pub extensions: ExtensionIndex,
    /// Exported symbol index for O(1) auto-import lookups.
    pub exports: ExportedSymbolIndex,
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
    /// Versions for modules that contribute package declared module bindings.
    pub registry_module_versions: IndexMap<ModuleId, ModuleVersion>,
    /// Module bindings by specifier.
    pub bindings_by_specifier: IndexMap<StringId, Vec<ModuleBindingReference>>,
}

/// Track module bindings declared in a package.
#[derive(Debug, Clone)]
pub struct ModuleBindingRegistry {
    /// Versions for modules that declared bindings.
    pub module_versions: IndexMap<ModuleId, ModuleVersion>,
    /// Module bindings by specifier.
    pub bindings_by_specifier: IndexMap<StringId, Vec<ModuleBindingReference>>,
}

impl Default for ModuleBindingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleBindingRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            module_versions: IndexMap::new(),
            bindings_by_specifier: IndexMap::new(),
        }
    }
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
            registry_module_versions: IndexMap::new(),
            bindings_by_specifier: IndexMap::new(),
        }
    }
}

/// Index for O(1) extension lookups by target symbol.
#[derive(Debug, Default)]
pub struct ExtensionIndex {
    /// Extensions indexed by target symbol.
    entries: DashMap<GlobalSymbolId, Vec<ExtensionEntry>>,
    /// Module versions for cache invalidation.
    versions: DashMap<ModuleId, ModuleVersion>,
}

/// An entry in the extension index.
#[derive(Debug, Clone)]
pub struct ExtensionEntry {
    /// The module containing the extension.
    pub module_id: ModuleId,
    /// The extension id within the module.
    pub extension_id: LocalExtensionId,
    /// The kind of extension (inherent, local, nominal).
    pub kind: ExtensionKind,
    /// Member names for fast filtering.
    pub members: Vec<StringId>,
}

impl ExtensionIndex {
    /// Create a new empty extension index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get extensions for a target symbol.
    pub fn get_extensions(&self, target: GlobalSymbolId) -> Vec<ExtensionEntry> {
        self.entries
            .get(&target)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Check if an entry for a module is stale.
    pub fn is_stale(&self, module_id: ModuleId, current_version: ModuleVersion) -> bool {
        self.versions
            .get(&module_id)
            .map(|v| *v != current_version)
            .unwrap_or(true)
    }

    /// Clear entries for a module.
    pub fn clear_module(&self, module_id: ModuleId) {
        // remove all entries from this module
        self.entries.retain(|_, entries| {
            entries.retain(|e| e.module_id != module_id);
            !entries.is_empty()
        });
        self.versions.remove(&module_id);
    }

    /// Add an extension entry.
    pub fn add_extension(&self, target: GlobalSymbolId, entry: ExtensionEntry) {
        self.entries.entry(target).or_default().push(entry);
    }

    /// Mark a module as indexed with the given version.
    pub fn set_module_version(&self, module_id: ModuleId, version: ModuleVersion) {
        self.versions.insert(module_id, version);
    }
}

// ----------------------------------------------------------------------------
// EXPORTED SYMBOL INDEX
// ----------------------------------------------------------------------------

/// Index for O(1) exported symbol lookups by name.
#[derive(Debug, Default)]
pub struct ExportedSymbolIndex {
    /// Exports indexed by symbol name.
    by_name: DashMap<StringId, Vec<ExportEntry>>,
    /// Module versions for cache invalidation.
    versions: DashMap<ModuleId, ModuleVersion>,
}

/// An entry in the exported symbol index.
#[derive(Debug, Clone)]
pub struct ExportEntry {
    /// The module exporting the symbol.
    pub module_id: ModuleId,
    /// The local symbol id.
    pub symbol_id: LocalSymbolId,
    /// The symbol type.
    pub symbol_type: SymbolType,
    /// The symbol space.
    pub space: SymbolSpace,
    /// The symbol name (for display).
    pub name: String,
    /// The module path (for import generation).
    pub module_path: Option<String>,
}

impl ExportedSymbolIndex {
    /// Create a new empty export index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get exports by name.
    pub fn get_by_name(&self, name: StringId) -> Vec<ExportEntry> {
        self.by_name
            .get(&name)
            .map(|v| v.clone())
            .unwrap_or_default()
    }

    /// Search exports by name prefix.
    pub fn search_by_prefix(
        &self,
        prefix: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<ExportEntry> {
        let prefix_lower = prefix.to_lowercase();
        let mut results = Vec::new();

        for entry in self.by_name.iter() {
            for export in entry.value() {
                if Some(export.module_id) == exclude_module {
                    continue;
                }
                if export.name.to_lowercase().starts_with(&prefix_lower) {
                    results.push(export.clone());
                }
            }
        }

        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    /// Check if an entry for a module is stale.
    pub fn is_stale(&self, module_id: ModuleId, current_version: ModuleVersion) -> bool {
        self.versions
            .get(&module_id)
            .map(|v| *v != current_version)
            .unwrap_or(true)
    }

    /// Clear entries for a module.
    pub fn clear_module(&self, module_id: ModuleId) {
        // remove all entries from this module
        self.by_name.retain(|_, entries| {
            entries.retain(|e| e.module_id != module_id);
            !entries.is_empty()
        });
        self.versions.remove(&module_id);
    }

    /// Add an export entry.
    pub fn add_export(&self, name: StringId, entry: ExportEntry) {
        self.by_name.entry(name).or_default().push(entry);
    }

    /// Mark a module as indexed with the given version.
    pub fn set_module_version(&self, module_id: ModuleId, version: ModuleVersion) {
        self.versions.insert(module_id, version);
    }
}
