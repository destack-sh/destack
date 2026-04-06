use std::path::{Path, PathBuf};

use destack_dir::GlobalSymbolId;
use destack_source::{ModuleId, PathExt};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    CallIndexEntry, ExtensionIndexEntry, ImportIndexEntry, NominalIndexEntry, SpecifierIndexEntry,
    SymbolIndexEntry,
};

fn search_prefixes(search_name: &str) -> Vec<String> {
    let mut prefixes = Vec::new();

    for end in 1..=search_name.len() {
        if !search_name.is_char_boundary(end) {
            continue;
        }

        prefixes.push(search_name[..end].to_string());
    }

    prefixes
}

fn word_boundary_key(name: &str) -> String {
    let mut boundary = String::new();

    for (index, character) in name.char_indices() {
        let previous = name[..index].chars().next_back();
        let is_boundary = index == 0
            || character.is_uppercase()
            || previous == Some('_')
            || previous == Some('-')
            || previous
                .map(|previous| !previous.is_alphanumeric() && character.is_alphanumeric())
                .unwrap_or(false);

        if is_boundary {
            boundary.extend(character.to_lowercase());
        }
    }

    boundary
}

fn search_characters(search_name: &str) -> FxHashSet<char> {
    search_name.chars().collect()
}

fn remove_entry_id(entry_ids: &mut Vec<usize>, entry_id: usize) {
    entry_ids.retain(|candidate| *candidate != entry_id);
}

fn path_prefixes(path: &Path) -> Vec<PathBuf> {
    let mut path = path.normalize();
    let mut prefixes = vec![path.clone()];

    while path.pop() {
        prefixes.push(path.clone());
    }

    prefixes
}

#[derive(Debug, Clone)]
struct StoredImportIndexEntry {
    search_name: String,
    boundary_name: String,
    entry: ImportIndexEntry,
}

#[derive(Debug, Clone)]
struct ImportIndexRootSlice {
    entry_ids: Vec<usize>,
    entry_id_set: FxHashSet<usize>,
}

/// The repository index for importable exported symbols.
#[derive(Debug, Default)]
pub struct ImportIndex {
    next_entry_id: usize,
    entries_by_root: FxHashMap<PathBuf, ImportIndexRootSlice>,
    entries_by_id: FxHashMap<usize, StoredImportIndexEntry>,
    entry_ids_by_prefix: FxHashMap<String, Vec<usize>>,
    entry_ids_by_boundary_prefix: FxHashMap<String, Vec<usize>>,
    entry_ids_by_character: FxHashMap<char, Vec<usize>>,
}

impl ImportIndex {
    /// Replace one workspace-root import slice.
    pub fn replace_root(&mut self, root: PathBuf, entries: Vec<ImportIndexEntry>) {
        self.remove_root(root.as_path());

        let mut entry_ids = Vec::new();
        let mut entry_id_set = FxHashSet::default();

        for entry in entries {
            let search_name = entry.name.to_lowercase();
            let boundary_name = word_boundary_key(&entry.name);
            let entry_id = self.next_entry_id;
            self.next_entry_id += 1;

            for prefix in search_prefixes(&search_name) {
                self.entry_ids_by_prefix
                    .entry(prefix)
                    .or_default()
                    .push(entry_id);
            }

            for prefix in search_prefixes(&boundary_name) {
                self.entry_ids_by_boundary_prefix
                    .entry(prefix)
                    .or_default()
                    .push(entry_id);
            }

            for character in search_characters(&search_name) {
                self.entry_ids_by_character
                    .entry(character)
                    .or_default()
                    .push(entry_id);
            }

            self.entries_by_id.insert(
                entry_id,
                StoredImportIndexEntry {
                    search_name,
                    boundary_name,
                    entry,
                },
            );
            entry_ids.push(entry_id);
            entry_id_set.insert(entry_id);
        }

        self.entries_by_root.insert(
            root,
            ImportIndexRootSlice {
                entry_ids,
                entry_id_set,
            },
        );
    }

    /// Remove one workspace-root import slice.
    pub fn remove_root(&mut self, root: &Path) {
        let Some(slice) = self.entries_by_root.remove(root) else {
            return;
        };

        for entry_id in slice.entry_ids {
            let Some(stored) = self.entries_by_id.remove(&entry_id) else {
                continue;
            };

            for prefix in search_prefixes(&stored.search_name) {
                let Some(entry_ids) = self.entry_ids_by_prefix.get_mut(&prefix) else {
                    continue;
                };

                remove_entry_id(entry_ids, entry_id);
                if entry_ids.is_empty() {
                    self.entry_ids_by_prefix.remove(&prefix);
                }
            }

            for prefix in search_prefixes(&stored.boundary_name) {
                let Some(entry_ids) = self.entry_ids_by_boundary_prefix.get_mut(&prefix) else {
                    continue;
                };

                remove_entry_id(entry_ids, entry_id);
                if entry_ids.is_empty() {
                    self.entry_ids_by_boundary_prefix.remove(&prefix);
                }
            }

            for character in search_characters(&stored.search_name) {
                let Some(entry_ids) = self.entry_ids_by_character.get_mut(&character) else {
                    continue;
                };

                remove_entry_id(entry_ids, entry_id);
                if entry_ids.is_empty() {
                    self.entry_ids_by_character.remove(&character);
                }
            }
        }
    }

    /// Search import entries for one root slice.
    pub fn search_root(
        &self,
        root: &Path,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<ImportIndexEntry> {
        let query = query.to_lowercase();
        let mut results = Vec::new();
        let Some(root_slice) = self.entries_by_root.get(root) else {
            return results;
        };

        if query.is_empty() {
            for &entry_id in &root_slice.entry_ids {
                let Some(stored) = self.entries_by_id.get(&entry_id) else {
                    continue;
                };

                if Some(stored.entry.module_id) == exclude_module {
                    continue;
                }

                results.push(stored.entry.clone());
            }
        } else {
            let mut candidate_ids = FxHashSet::default();

            if let Some(entry_ids) = self.entry_ids_by_prefix.get(&query) {
                candidate_ids.extend(entry_ids.iter().copied());
            }

            if let Some(entry_ids) = self.entry_ids_by_boundary_prefix.get(&query) {
                candidate_ids.extend(entry_ids.iter().copied());
            }

            candidate_ids.extend(self.character_candidates(&query));

            for entry_id in candidate_ids {
                let Some(stored) = self.entries_by_id.get(&entry_id) else {
                    continue;
                };

                if Some(stored.entry.module_id) == exclude_module {
                    continue;
                }
                if !root_slice.entry_id_set.contains(&entry_id) {
                    continue;
                }

                results.push(stored.entry.clone());
            }
        }

        results.sort_by(|left, right| {
            let left_key = (&left.name, left.module_id, left.local_id.id);
            let right_key = (&right.name, right.module_id, right.local_id.id);
            left_key.cmp(&right_key)
        });

        results
    }

    fn character_candidates(&self, query: &str) -> FxHashSet<usize> {
        let mut query_characters = search_characters(query).into_iter();
        let Some(first_character) = query_characters.next() else {
            return FxHashSet::default();
        };

        let Some(initial_candidates) = self.entry_ids_by_character.get(&first_character) else {
            return FxHashSet::default();
        };

        let mut candidate_ids: FxHashSet<_> = initial_candidates.iter().copied().collect();

        for character in query_characters {
            let Some(entry_ids) = self.entry_ids_by_character.get(&character) else {
                return FxHashSet::default();
            };

            let character_candidates: FxHashSet<_> = entry_ids.iter().copied().collect();
            candidate_ids.retain(|entry_id| character_candidates.contains(entry_id));
            if candidate_ids.is_empty() {
                return candidate_ids;
            }
        }

        candidate_ids
    }
}

#[derive(Debug, Clone)]
struct StoredSymbolIndexEntry {
    search_name: String,
    boundary_name: String,
    entry: SymbolIndexEntry,
}

#[derive(Debug, Clone)]
struct SymbolIndexModuleSlice {
    entry_ids: Vec<usize>,
}

/// The repository index for workspace symbol search.
#[derive(Debug, Default)]
pub struct SymbolIndex {
    next_entry_id: usize,
    entries_by_module: FxHashMap<ModuleId, SymbolIndexModuleSlice>,
    entries_by_id: FxHashMap<usize, StoredSymbolIndexEntry>,
    entry_ids_by_prefix: FxHashMap<String, Vec<usize>>,
    entry_ids_by_boundary_prefix: FxHashMap<String, Vec<usize>>,
    entry_ids_by_character: FxHashMap<char, Vec<usize>>,
}

impl SymbolIndex {
    /// Replace one module symbol slice.
    pub fn replace_module(&mut self, module_id: ModuleId, entries: Vec<SymbolIndexEntry>) {
        self.remove_module(module_id);

        let mut entry_ids = Vec::new();

        for entry in entries {
            let search_name = entry.name.to_lowercase();
            let boundary_name = word_boundary_key(&entry.name);
            let entry_id = self.next_entry_id;
            self.next_entry_id += 1;

            for prefix in search_prefixes(&search_name) {
                self.entry_ids_by_prefix
                    .entry(prefix)
                    .or_default()
                    .push(entry_id);
            }

            for prefix in search_prefixes(&boundary_name) {
                self.entry_ids_by_boundary_prefix
                    .entry(prefix)
                    .or_default()
                    .push(entry_id);
            }

            for character in search_characters(&search_name) {
                self.entry_ids_by_character
                    .entry(character)
                    .or_default()
                    .push(entry_id);
            }

            self.entries_by_id.insert(
                entry_id,
                StoredSymbolIndexEntry {
                    search_name,
                    boundary_name,
                    entry,
                },
            );
            entry_ids.push(entry_id);
        }

        self.entries_by_module
            .insert(module_id, SymbolIndexModuleSlice { entry_ids });
    }

    /// Remove one module symbol slice.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        for entry_id in slice.entry_ids {
            let Some(stored) = self.entries_by_id.remove(&entry_id) else {
                continue;
            };

            for prefix in search_prefixes(&stored.search_name) {
                let Some(entry_ids) = self.entry_ids_by_prefix.get_mut(&prefix) else {
                    continue;
                };

                remove_entry_id(entry_ids, entry_id);
                if entry_ids.is_empty() {
                    self.entry_ids_by_prefix.remove(&prefix);
                }
            }

            for prefix in search_prefixes(&stored.boundary_name) {
                let Some(entry_ids) = self.entry_ids_by_boundary_prefix.get_mut(&prefix) else {
                    continue;
                };

                remove_entry_id(entry_ids, entry_id);
                if entry_ids.is_empty() {
                    self.entry_ids_by_boundary_prefix.remove(&prefix);
                }
            }

            for character in search_characters(&stored.search_name) {
                let Some(entry_ids) = self.entry_ids_by_character.get_mut(&character) else {
                    continue;
                };

                remove_entry_id(entry_ids, entry_id);
                if entry_ids.is_empty() {
                    self.entry_ids_by_character.remove(&character);
                }
            }
        }
    }

    /// Search workspace symbols.
    pub fn search(&self, query: &str) -> Vec<SymbolIndexEntry> {
        let query = query.to_lowercase();
        let mut results = Vec::new();

        if query.is_empty() {
            for slice in self.entries_by_module.values() {
                for entry_id in &slice.entry_ids {
                    let Some(stored) = self.entries_by_id.get(entry_id) else {
                        continue;
                    };

                    results.push(stored.entry.clone());
                }
            }
        } else {
            let mut candidate_ids = FxHashSet::default();

            if let Some(entry_ids) = self.entry_ids_by_prefix.get(&query) {
                candidate_ids.extend(entry_ids.iter().copied());
            }

            if let Some(entry_ids) = self.entry_ids_by_boundary_prefix.get(&query) {
                candidate_ids.extend(entry_ids.iter().copied());
            }

            candidate_ids.extend(self.character_candidates(&query));

            for entry_id in candidate_ids {
                let Some(stored) = self.entries_by_id.get(&entry_id) else {
                    continue;
                };

                results.push(stored.entry.clone());
            }
        }

        results.sort_by(|left, right| {
            let left_key = (&left.name, left.file_id.0, left.range.start, left.range.end);
            let right_key = (
                &right.name,
                right.file_id.0,
                right.range.start,
                right.range.end,
            );
            left_key.cmp(&right_key)
        });

        results
    }

    fn character_candidates(&self, query: &str) -> FxHashSet<usize> {
        let mut query_characters = search_characters(query).into_iter();
        let Some(first_character) = query_characters.next() else {
            return FxHashSet::default();
        };

        let Some(initial_candidates) = self.entry_ids_by_character.get(&first_character) else {
            return FxHashSet::default();
        };

        let mut candidate_ids: FxHashSet<_> = initial_candidates.iter().copied().collect();

        for character in query_characters {
            let Some(entry_ids) = self.entry_ids_by_character.get(&character) else {
                return FxHashSet::default();
            };

            let character_candidates: FxHashSet<_> = entry_ids.iter().copied().collect();
            candidate_ids.retain(|entry_id| character_candidates.contains(entry_id));
            if candidate_ids.is_empty() {
                return candidate_ids;
            }
        }

        candidate_ids
    }
}

#[derive(Debug, Clone)]
struct NominalIndexModuleSlice {
    entries: Vec<NominalIndexEntry>,
}

/// The repository index for nominal hierarchy relations.
#[derive(Debug, Default)]
pub struct NominalIndex {
    entries_by_module: FxHashMap<ModuleId, NominalIndexModuleSlice>,
    entries_by_target: FxHashMap<GlobalSymbolId, Vec<NominalIndexEntry>>,
}

impl NominalIndex {
    /// Replace one nominal slice for one module.
    pub fn replace_module(&mut self, module_id: ModuleId, entries: Vec<NominalIndexEntry>) {
        if let Some(previous_slice) = self.entries_by_module.remove(&module_id) {
            self.remove_target_entries(&previous_slice.entries);
        }

        self.insert_target_entries(&entries);
        self.entries_by_module
            .insert(module_id, NominalIndexModuleSlice { entries });
    }

    /// Remove one nominal slice for one module.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        self.remove_target_entries(&slice.entries);
    }

    /// Collect nominal entries for one target symbol.
    pub fn entries_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<NominalIndexEntry> {
        self.entries_by_target
            .get(&target_symbol)
            .cloned()
            .unwrap_or_default()
    }

    fn insert_target_entries(&mut self, entries: &[NominalIndexEntry]) {
        for entry in entries {
            self.entries_by_target
                .entry(entry.target_symbol)
                .or_default()
                .push(*entry);
        }
    }

    fn remove_target_entries(&mut self, entries: &[NominalIndexEntry]) {
        for entry in entries {
            let Some(target_entries) = self.entries_by_target.get_mut(&entry.target_symbol) else {
                continue;
            };

            target_entries.retain(|candidate: &NominalIndexEntry| candidate != entry);
            if target_entries.is_empty() {
                self.entries_by_target.remove(&entry.target_symbol);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ExtensionIndexModuleSlice {
    entries: Vec<ExtensionIndexEntry>,
}

/// The repository index for extension lookups.
#[derive(Debug, Default)]
pub struct ExtensionIndex {
    entries_by_module: FxHashMap<ModuleId, ExtensionIndexModuleSlice>,
    entries_by_target: FxHashMap<GlobalSymbolId, Vec<ExtensionIndexEntry>>,
}

impl ExtensionIndex {
    /// Replace one extension slice for one module.
    pub fn replace_module(&mut self, module_id: ModuleId, entries: Vec<ExtensionIndexEntry>) {
        if let Some(previous_slice) = self.entries_by_module.remove(&module_id) {
            self.remove_target_entries(&previous_slice.entries);
        }

        self.insert_target_entries(&entries);
        self.entries_by_module
            .insert(module_id, ExtensionIndexModuleSlice { entries });
    }

    /// Remove one extension slice for one module.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        self.remove_target_entries(&slice.entries);
    }

    /// Collect extension entries for one target symbol.
    pub fn entries_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<ExtensionIndexEntry> {
        self.entries_by_target
            .get(&target_symbol)
            .cloned()
            .unwrap_or_default()
    }

    fn insert_target_entries(&mut self, entries: &[ExtensionIndexEntry]) {
        for entry in entries {
            self.entries_by_target
                .entry(entry.target_symbol)
                .or_default()
                .push(*entry);
        }
    }

    fn remove_target_entries(&mut self, entries: &[ExtensionIndexEntry]) {
        for entry in entries {
            let Some(target_entries) = self.entries_by_target.get_mut(&entry.target_symbol) else {
                continue;
            };

            target_entries.retain(|candidate: &ExtensionIndexEntry| candidate != entry);
            if target_entries.is_empty() {
                self.entries_by_target.remove(&entry.target_symbol);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ReferenceIndexModuleSlice {
    targets: Vec<GlobalSymbolId>,
}

/// The repository index for reference candidate modules.
#[derive(Debug, Default)]
pub struct ReferenceIndex {
    entries_by_module: FxHashMap<ModuleId, ReferenceIndexModuleSlice>,
    modules_by_target: FxHashMap<GlobalSymbolId, Vec<ModuleId>>,
}

impl ReferenceIndex {
    /// Replace one reference slice for one module.
    pub fn replace_module(&mut self, module_id: ModuleId, targets: Vec<GlobalSymbolId>) {
        if let Some(previous_slice) = self.entries_by_module.remove(&module_id) {
            self.remove_target_modules(module_id, &previous_slice.targets);
        }

        self.insert_target_modules(module_id, &targets);
        self.entries_by_module
            .insert(module_id, ReferenceIndexModuleSlice { targets });
    }

    /// Remove one reference slice for one module.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        self.remove_target_modules(module_id, &slice.targets);
    }

    /// Collect modules for one reference target.
    pub fn modules_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<ModuleId> {
        self.modules_by_target
            .get(&target_symbol)
            .cloned()
            .unwrap_or_default()
    }

    fn insert_target_modules(&mut self, module_id: ModuleId, targets: &[GlobalSymbolId]) {
        for target_symbol in targets {
            let modules = self.modules_by_target.entry(*target_symbol).or_default();
            if !modules.contains(&module_id) {
                modules.push(module_id);
            }
        }
    }

    fn remove_target_modules(&mut self, module_id: ModuleId, targets: &[GlobalSymbolId]) {
        for target_symbol in targets {
            let Some(modules) = self.modules_by_target.get_mut(target_symbol) else {
                continue;
            };

            modules.retain(|candidate: &ModuleId| *candidate != module_id);
            if modules.is_empty() {
                self.modules_by_target.remove(target_symbol);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct CallIndexModuleSlice {
    entries: Vec<CallIndexEntry>,
}

/// The repository index for call hierarchy edges.
#[derive(Debug, Default)]
pub struct CallIndex {
    entries_by_module: FxHashMap<ModuleId, CallIndexModuleSlice>,
    entries_by_callee: FxHashMap<GlobalSymbolId, Vec<CallIndexEntry>>,
    entries_by_caller: FxHashMap<GlobalSymbolId, Vec<CallIndexEntry>>,
}

impl CallIndex {
    /// Replace one call slice for one module.
    pub fn replace_module(&mut self, module_id: ModuleId, entries: Vec<CallIndexEntry>) {
        if let Some(previous_slice) = self.entries_by_module.remove(&module_id) {
            self.remove_edges(&previous_slice.entries);
        }

        self.insert_edges(&entries);
        self.entries_by_module
            .insert(module_id, CallIndexModuleSlice { entries });
    }

    /// Remove one call slice for one module.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        self.remove_edges(&slice.entries);
    }

    /// Collect call entries for one callee.
    pub fn entries_for_callee(&self, callee_symbol: GlobalSymbolId) -> Vec<CallIndexEntry> {
        self.entries_by_callee
            .get(&callee_symbol)
            .cloned()
            .unwrap_or_default()
    }

    /// Collect call entries for one caller.
    pub fn entries_for_caller(&self, caller_symbol: GlobalSymbolId) -> Vec<CallIndexEntry> {
        self.entries_by_caller
            .get(&caller_symbol)
            .cloned()
            .unwrap_or_default()
    }

    fn insert_edges(&mut self, entries: &[CallIndexEntry]) {
        for entry in entries {
            self.entries_by_callee
                .entry(entry.callee_symbol)
                .or_default()
                .push(*entry);

            if let Some(caller_symbol) = entry.caller_symbol {
                self.entries_by_caller
                    .entry(caller_symbol)
                    .or_default()
                    .push(*entry);
            }
        }
    }

    fn remove_edges(&mut self, entries: &[CallIndexEntry]) {
        for entry in entries {
            if let Some(callee_entries) = self.entries_by_callee.get_mut(&entry.callee_symbol) {
                callee_entries.retain(|candidate: &CallIndexEntry| candidate != entry);
                if callee_entries.is_empty() {
                    self.entries_by_callee.remove(&entry.callee_symbol);
                }
            }

            let Some(caller_symbol) = entry.caller_symbol else {
                continue;
            };

            if let Some(caller_entries) = self.entries_by_caller.get_mut(&caller_symbol) {
                caller_entries.retain(|candidate: &CallIndexEntry| candidate != entry);
                if caller_entries.is_empty() {
                    self.entries_by_caller.remove(&caller_symbol);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct StoredSpecifierIndexEntry {
    entry: SpecifierIndexEntry,
}

#[derive(Debug, Clone)]
struct SpecifierIndexModuleSlice {
    entry_ids: Vec<usize>,
}

/// The repository index for module specifier rewrites.
#[derive(Debug, Default)]
pub struct SpecifierIndex {
    next_entry_id: usize,
    entries_by_module: FxHashMap<ModuleId, SpecifierIndexModuleSlice>,
    entries_by_id: FxHashMap<usize, StoredSpecifierIndexEntry>,
    entry_ids_by_target_path_prefix: FxHashMap<PathBuf, Vec<usize>>,
    unresolved_entry_ids: Vec<usize>,
}

impl SpecifierIndex {
    /// Replace one specifier slice for one module.
    pub fn replace_module(&mut self, module_id: ModuleId, entries: Vec<SpecifierIndexEntry>) {
        self.remove_module(module_id);

        let mut entry_ids = Vec::new();

        for entry in entries {
            let entry_id = self.next_entry_id;
            self.next_entry_id += 1;

            if let Some(target_path) = entry.target_path.as_deref() {
                for prefix in path_prefixes(target_path) {
                    self.entry_ids_by_target_path_prefix
                        .entry(prefix)
                        .or_default()
                        .push(entry_id);
                }
            }

            if entry.target_module_id.is_none() && entry.target_path.is_none() {
                self.unresolved_entry_ids.push(entry_id);
            }

            self.entries_by_id
                .insert(entry_id, StoredSpecifierIndexEntry { entry });
            entry_ids.push(entry_id);
        }

        self.entries_by_module
            .insert(module_id, SpecifierIndexModuleSlice { entry_ids });
    }

    /// Remove one specifier slice for one module.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        for entry_id in slice.entry_ids {
            let Some(stored) = self.entries_by_id.remove(&entry_id) else {
                continue;
            };

            if let Some(target_path) = stored.entry.target_path.as_deref() {
                for prefix in path_prefixes(target_path) {
                    let Some(entry_ids) = self.entry_ids_by_target_path_prefix.get_mut(&prefix)
                    else {
                        continue;
                    };

                    remove_entry_id(entry_ids, entry_id);
                    if entry_ids.is_empty() {
                        self.entry_ids_by_target_path_prefix.remove(&prefix);
                    }
                }
            }

            if stored.entry.target_module_id.is_none() && stored.entry.target_path.is_none() {
                remove_entry_id(&mut self.unresolved_entry_ids, entry_id);
            }
        }
    }

    /// Collect specifier entries for one set of renamed paths plus unresolved fallbacks.
    pub fn entries_for_paths_or_unresolved<I>(&self, target_paths: I) -> Vec<SpecifierIndexEntry>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let mut candidate_ids = FxHashSet::default();

        for target_path in target_paths {
            let target_path = target_path.normalize();
            let Some(entry_ids) = self.entry_ids_by_target_path_prefix.get(&target_path) else {
                continue;
            };

            candidate_ids.extend(entry_ids.iter().copied());
        }

        candidate_ids.extend(self.unresolved_entry_ids.iter().copied());

        let mut entries = Vec::new();
        for entry_id in candidate_ids {
            let Some(stored) = self.entries_by_id.get(&entry_id) else {
                continue;
            };

            entries.push(stored.entry.clone());
        }

        entries.sort_by(|left, right| {
            let left_key = (
                left.file_id.0,
                left.ast_node_id,
                left.module_id,
                left.specifier.as_str(),
            );
            let right_key = (
                right.file_id.0,
                right.ast_node_id,
                right.module_id,
                right.specifier.as_str(),
            );
            left_key.cmp(&right_key)
        });

        entries
    }
}

/// The repository-owned workspace query indexes.
#[derive(Debug, Default)]
pub struct QueryIndex {
    /// The importable exported symbol index.
    pub import: ImportIndex,
    /// The workspace symbol search index.
    pub symbol: SymbolIndex,
    /// The nominal hierarchy index.
    pub nominal: NominalIndex,
    /// The extension lookup index.
    pub extension: ExtensionIndex,
    /// The reference candidate module index.
    pub reference: ReferenceIndex,
    /// The call hierarchy edge index.
    pub call: CallIndex,
    /// The module specifier index.
    pub specifier: SpecifierIndex,
}
