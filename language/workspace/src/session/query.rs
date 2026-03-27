use std::path::{Path, PathBuf};

use rustc_hash::{FxHashMap, FxHashSet};

use destack_dir::{GlobalSymbolId, LocalExtensionId, LocalSymbolId, SymbolSpace, SymbolType};
use destack_source::{FileId, ModuleId, PathExt, Span};

/// Build all lowercase prefixes for one search key.
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

/// Build one lowercase word-boundary key for a symbol name.
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

/// Build the distinct lowercase characters for one search key.
fn search_characters(search_name: &str) -> FxHashSet<char> {
    search_name.chars().collect()
}

/// Remove one entry id from one posting list.
fn remove_entry_id(entry_ids: &mut Vec<usize>, entry_id: usize) {
    entry_ids.retain(|candidate| *candidate != entry_id);
}

/// Build all normalized ancestor paths for one target path.
fn path_prefixes(path: &Path) -> Vec<PathBuf> {
    let mut path = path.normalize();
    let mut prefixes = vec![path.clone()];

    while path.pop() {
        prefixes.push(path.clone());
    }

    prefixes
}

/// One importable exported symbol entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportIndexEntry {
    /// The exported name.
    pub name: String,
    /// The symbol type.
    pub kind: SymbolType,
    /// The symbol space.
    pub space: SymbolSpace,
    /// The exporting module id.
    pub module_id: ModuleId,
    /// The local symbol id within the exporting module.
    pub local_id: LocalSymbolId,
    /// The canonical import path when available.
    pub module_path: Option<String>,
}

/// One stored import index entry with normalized search state.
#[derive(Debug, Clone)]
struct StoredImportIndexEntry {
    /// The lowercase search key.
    search_name: String,
    /// The lowercase word-boundary key.
    boundary_name: String,
    /// The exported symbol entry.
    entry: ImportIndexEntry,
}

/// One program slice inside the import index.
#[derive(Debug, Clone)]
struct ImportIndexProgramSlice {
    /// The stored entry ids for this program.
    entry_ids: Vec<usize>,
    /// The stored entry ids for constant-time membership checks.
    entry_id_set: FxHashSet<usize>,
}

/// The workspace index for importable exported symbols.
#[derive(Debug, Default)]
pub struct ImportIndex {
    /// The next stored entry id.
    next_entry_id: usize,
    /// The entries grouped by owning program root.
    entries_by_program: FxHashMap<PathBuf, ImportIndexProgramSlice>,
    /// The stored entries by id.
    entries_by_id: FxHashMap<usize, StoredImportIndexEntry>,
    /// The entry ids grouped by lowercase prefixes.
    entry_ids_by_prefix: FxHashMap<String, Vec<usize>>,
    /// The entry ids grouped by word-boundary prefixes.
    entry_ids_by_boundary_prefix: FxHashMap<String, Vec<usize>>,
    /// The entry ids grouped by contained lowercase characters.
    entry_ids_by_character: FxHashMap<char, Vec<usize>>,
}

impl ImportIndex {
    /// Return true when one program slice exists.
    pub fn has_program(&self, root: &Path) -> bool {
        self.entries_by_program.contains_key(root)
    }

    /// Return the number of indexed program slices.
    pub fn program_count(&self) -> usize {
        self.entries_by_program.len()
    }

    /// Replace one program slice in the import index.
    pub fn replace_program(&mut self, root: PathBuf, entries: Vec<ImportIndexEntry>) {
        self.remove_program(root.as_path());

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

        self.entries_by_program.insert(
            root,
            ImportIndexProgramSlice {
                entry_ids,
                entry_id_set,
            },
        );
    }

    /// Remove one program slice from the import index.
    pub fn remove_program(&mut self, root: &Path) {
        let Some(slice) = self.entries_by_program.remove(root) else {
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

    /// Search import entries for one owning program.
    pub fn search_program(
        &self,
        root: &Path,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<ImportIndexEntry> {
        let query = query.to_lowercase();
        let mut results = Vec::new();
        let Some(program_slice) = self.entries_by_program.get(root) else {
            return results;
        };

        // empty query: return all entries for this program
        if query.is_empty() {
            for &entry_id in &program_slice.entry_ids {
                let Some(stored) = self.entries_by_id.get(&entry_id) else {
                    continue;
                };

                if Some(stored.entry.module_id) == exclude_module {
                    continue;
                }

                results.push(stored.entry.clone());
            }
        }
        // non-empty query: intersect keyed candidates with this program slice
        else {
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

                let entry = &stored.entry;
                if Some(entry.module_id) == exclude_module {
                    continue;
                }
                if !program_slice.entry_id_set.contains(&entry_id) {
                    continue;
                }

                results.push(entry.clone());
            }
        }

        // keep result order deterministic
        results.sort_by(|left, right| {
            let left_key = (&left.name, left.module_id, left.local_id.id);
            let right_key = (&right.name, right.module_id, right.local_id.id);
            left_key.cmp(&right_key)
        });

        results
    }

    /// Collect character-bucket candidates for one lowercase query.
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

/// The workspace index for named symbol search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolIndexKind {
    /// A namespace symbol.
    Namespace,
    /// A class symbol.
    Class,
    /// A method symbol.
    Method,
    /// A field symbol.
    Field,
    /// An enum symbol.
    Enum,
    /// An interface symbol.
    Interface,
    /// A function symbol.
    Function,
    /// A variable symbol.
    Variable,
    /// A constant symbol.
    Constant,
    /// An enum member symbol.
    EnumMember,
    /// A struct symbol.
    Struct,
    /// A type parameter symbol.
    TypeParameter,
}

/// One searchable workspace symbol entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolIndexEntry {
    /// The symbol name.
    pub name: String,
    /// The symbol kind.
    pub kind: SymbolIndexKind,
    /// The owning module id.
    pub module_id: ModuleId,
    /// The source file id.
    pub file_id: FileId,
    /// The symbol range.
    pub range: Span,
    /// The optional container name.
    pub container_name: Option<String>,
}

/// One stored symbol index entry with normalized search state.
#[derive(Debug, Clone)]
struct StoredSymbolIndexEntry {
    /// The lowercase symbol name.
    search_name: String,
    /// The lowercase word-boundary key.
    boundary_name: String,
    /// The stored symbol entry.
    entry: SymbolIndexEntry,
}

/// One module slice inside the symbol index.
#[derive(Debug, Clone)]
struct SymbolIndexModuleSlice {
    /// The stored entry ids for this module.
    entry_ids: Vec<usize>,
}

/// The workspace index for named symbol search.
#[derive(Debug, Default)]
pub struct SymbolIndex {
    /// The next stored entry id.
    next_entry_id: usize,
    /// The entries grouped by declaring module.
    entries_by_module: FxHashMap<ModuleId, SymbolIndexModuleSlice>,
    /// The stored entries by id.
    entries_by_id: FxHashMap<usize, StoredSymbolIndexEntry>,
    /// The entry ids grouped by lowercase prefixes.
    entry_ids_by_prefix: FxHashMap<String, Vec<usize>>,
    /// The entry ids grouped by word-boundary prefixes.
    entry_ids_by_boundary_prefix: FxHashMap<String, Vec<usize>>,
    /// The entry ids grouped by contained lowercase characters.
    entry_ids_by_character: FxHashMap<char, Vec<usize>>,
}

impl SymbolIndex {
    /// Return true when one module slice exists.
    pub fn has_module(&self, module_id: ModuleId) -> bool {
        self.entries_by_module.contains_key(&module_id)
    }

    /// Return the number of indexed module slices.
    pub fn module_count(&self) -> usize {
        self.entries_by_module.len()
    }

    /// Replace one module slice in the symbol index.
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

    /// Remove one module slice from the symbol index.
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

    /// Collect symbol entries across one set of modules.
    pub fn search(&self, query: &str) -> Vec<SymbolIndexEntry> {
        let query = query.to_lowercase();
        let mut results = Vec::new();

        // empty query: return all indexed workspace symbols
        if query.is_empty() {
            for slice in self.entries_by_module.values() {
                for entry_id in &slice.entry_ids {
                    let Some(stored) = self.entries_by_id.get(entry_id) else {
                        continue;
                    };

                    results.push(stored.entry.clone());
                }
            }
        }
        // non-empty query: use keyed candidate buckets instead of scanning every symbol
        else {
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

        // keep result order deterministic
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

    /// Collect character-bucket candidates for one lowercase query.
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

/// The workspace index for nominal hierarchy relations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NominalRelationKind {
    /// A direct `extends` relationship.
    Extends,
    /// A direct `implements` relationship.
    Implements,
    /// A direct `embedded` relationship.
    Embeds,
}

/// One nominal hierarchy relation entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NominalIndexEntry {
    /// The symbol that owns the relation.
    pub source_symbol: GlobalSymbolId,
    /// The directly related target symbol.
    pub target_symbol: GlobalSymbolId,
    /// The direct relation kind.
    pub relation: NominalRelationKind,
}

/// One module slice inside the nominal index.
#[derive(Debug, Clone)]
struct NominalIndexModuleSlice {
    /// The stored entries for this module.
    entries: Vec<NominalIndexEntry>,
}

/// The workspace index for nominal hierarchy relations.
#[derive(Debug, Default)]
pub struct NominalIndex {
    /// The entries grouped by declaring module.
    entries_by_module: FxHashMap<ModuleId, NominalIndexModuleSlice>,
    /// The entries grouped by target symbol.
    entries_by_target: FxHashMap<GlobalSymbolId, Vec<NominalIndexEntry>>,
}

impl NominalIndex {
    /// Return true when one module slice exists.
    pub fn has_module(&self, module_id: ModuleId) -> bool {
        self.entries_by_module.contains_key(&module_id)
    }

    /// Return the number of indexed module slices.
    pub fn module_count(&self) -> usize {
        self.entries_by_module.len()
    }

    /// Replace one module slice in the nominal index.
    pub fn replace_module(&mut self, module_id: ModuleId, entries: Vec<NominalIndexEntry>) {
        if let Some(previous_slice) = self.entries_by_module.remove(&module_id) {
            self.remove_target_entries(&previous_slice.entries);
        }

        self.insert_target_entries(&entries);
        self.entries_by_module
            .insert(module_id, NominalIndexModuleSlice { entries });
    }

    /// Remove one module slice from the nominal index.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        self.remove_target_entries(&slice.entries);
    }

    /// Collect direct relation entries for one target symbol.
    pub fn entries_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<NominalIndexEntry> {
        self.entries_by_target
            .get(&target_symbol)
            .cloned()
            .unwrap_or_default()
    }

    /// Add one batch of target entries to the reverse lookup map.
    fn insert_target_entries(&mut self, entries: &[NominalIndexEntry]) {
        for entry in entries {
            self.entries_by_target
                .entry(entry.target_symbol)
                .or_default()
                .push(*entry);
        }
    }

    /// Remove one batch of target entries from the reverse lookup map.
    fn remove_target_entries(&mut self, entries: &[NominalIndexEntry]) {
        for entry in entries {
            let Some(target_entries) = self.entries_by_target.get_mut(&entry.target_symbol) else {
                continue;
            };

            target_entries.retain(|candidate| candidate != entry);
            if target_entries.is_empty() {
                self.entries_by_target.remove(&entry.target_symbol);
            }
        }
    }
}

/// One extension lookup entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtensionIndexEntry {
    /// The module that owns the extension.
    pub module_id: ModuleId,
    /// The local extension id within the module.
    pub extension_id: LocalExtensionId,
    /// The extension target symbol.
    pub target_symbol: GlobalSymbolId,
}

/// One module slice inside the extension index.
#[derive(Debug, Clone)]
struct ExtensionIndexModuleSlice {
    /// The stored entries for this module.
    entries: Vec<ExtensionIndexEntry>,
}

/// The workspace index for extensions by target symbol.
#[derive(Debug, Default)]
pub struct ExtensionIndex {
    /// The entries grouped by module.
    entries_by_module: FxHashMap<ModuleId, ExtensionIndexModuleSlice>,
    /// The entries grouped by target symbol.
    entries_by_target: FxHashMap<GlobalSymbolId, Vec<ExtensionIndexEntry>>,
}

impl ExtensionIndex {
    /// Return true when one module slice exists.
    pub fn has_module(&self, module_id: ModuleId) -> bool {
        self.entries_by_module.contains_key(&module_id)
    }

    /// Return the number of indexed module slices.
    pub fn module_count(&self) -> usize {
        self.entries_by_module.len()
    }

    /// Replace one module slice in the extension index.
    pub fn replace_module(&mut self, module_id: ModuleId, entries: Vec<ExtensionIndexEntry>) {
        if let Some(previous_slice) = self.entries_by_module.remove(&module_id) {
            self.remove_target_entries(&previous_slice.entries);
        }

        self.insert_target_entries(&entries);
        self.entries_by_module
            .insert(module_id, ExtensionIndexModuleSlice { entries });
    }

    /// Remove one module slice from the extension index.
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

    /// Add one batch of target entries to the reverse lookup map.
    fn insert_target_entries(&mut self, entries: &[ExtensionIndexEntry]) {
        for entry in entries {
            self.entries_by_target
                .entry(entry.target_symbol)
                .or_default()
                .push(*entry);
        }
    }

    /// Remove one batch of target entries from the reverse lookup map.
    fn remove_target_entries(&mut self, entries: &[ExtensionIndexEntry]) {
        for entry in entries {
            let Some(target_entries) = self.entries_by_target.get_mut(&entry.target_symbol) else {
                continue;
            };

            target_entries.retain(|candidate| candidate != entry);
            if target_entries.is_empty() {
                self.entries_by_target.remove(&entry.target_symbol);
            }
        }
    }
}

/// One module slice inside the reference index.
#[derive(Debug, Clone)]
struct ReferenceIndexModuleSlice {
    /// The indexed target symbols referenced in this module.
    targets: Vec<GlobalSymbolId>,
}

/// The workspace index for reference candidate modules.
#[derive(Debug, Default)]
pub struct ReferenceIndex {
    /// The indexed targets grouped by module.
    entries_by_module: FxHashMap<ModuleId, ReferenceIndexModuleSlice>,
    /// The candidate modules grouped by target symbol.
    modules_by_target: FxHashMap<GlobalSymbolId, Vec<ModuleId>>,
}

impl ReferenceIndex {
    /// Return true when one module slice exists.
    pub fn has_module(&self, module_id: ModuleId) -> bool {
        self.entries_by_module.contains_key(&module_id)
    }

    /// Return the number of indexed module slices.
    pub fn module_count(&self) -> usize {
        self.entries_by_module.len()
    }

    /// Replace one module slice in the reference index.
    pub fn replace_module(&mut self, module_id: ModuleId, targets: Vec<GlobalSymbolId>) {
        if let Some(previous_slice) = self.entries_by_module.remove(&module_id) {
            self.remove_target_modules(module_id, &previous_slice.targets);
        }

        self.insert_target_modules(module_id, &targets);
        self.entries_by_module
            .insert(module_id, ReferenceIndexModuleSlice { targets });
    }

    /// Remove one module slice from the reference index.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        self.remove_target_modules(module_id, &slice.targets);
    }

    /// Collect candidate modules for one target symbol.
    pub fn modules_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<ModuleId> {
        self.modules_by_target
            .get(&target_symbol)
            .cloned()
            .unwrap_or_default()
    }

    /// Add one module id to each target bucket.
    fn insert_target_modules(&mut self, module_id: ModuleId, targets: &[GlobalSymbolId]) {
        for target_symbol in targets {
            let modules = self.modules_by_target.entry(*target_symbol).or_default();
            if !modules.contains(&module_id) {
                modules.push(module_id);
            }
        }
    }

    /// Remove one module id from each target bucket.
    fn remove_target_modules(&mut self, module_id: ModuleId, targets: &[GlobalSymbolId]) {
        for target_symbol in targets {
            let Some(modules) = self.modules_by_target.get_mut(target_symbol) else {
                continue;
            };

            modules.retain(|candidate| *candidate != module_id);
            if modules.is_empty() {
                self.modules_by_target.remove(target_symbol);
            }
        }
    }
}

/// One indexed call edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallIndexEntry {
    /// The module that contains the call site.
    pub module_id: ModuleId,
    /// The caller function symbol when the call is inside one function body.
    pub caller_symbol: Option<GlobalSymbolId>,
    /// The callee function symbol.
    pub callee_symbol: GlobalSymbolId,
    /// The call expression span.
    pub span: Span,
}

/// One module slice inside the call index.
#[derive(Debug, Clone)]
struct CallIndexModuleSlice {
    /// The stored entries for this module.
    entries: Vec<CallIndexEntry>,
}

/// The workspace index for call hierarchy edges.
#[derive(Debug, Default)]
pub struct CallIndex {
    /// The entries grouped by module.
    entries_by_module: FxHashMap<ModuleId, CallIndexModuleSlice>,
    /// The entries grouped by callee.
    entries_by_callee: FxHashMap<GlobalSymbolId, Vec<CallIndexEntry>>,
    /// The entries grouped by caller.
    entries_by_caller: FxHashMap<GlobalSymbolId, Vec<CallIndexEntry>>,
}

impl CallIndex {
    /// Return true when one module slice exists.
    pub fn has_module(&self, module_id: ModuleId) -> bool {
        self.entries_by_module.contains_key(&module_id)
    }

    /// Return the number of indexed module slices.
    pub fn module_count(&self) -> usize {
        self.entries_by_module.len()
    }

    /// Replace one module slice in the call index.
    pub fn replace_module(&mut self, module_id: ModuleId, entries: Vec<CallIndexEntry>) {
        if let Some(previous_slice) = self.entries_by_module.remove(&module_id) {
            self.remove_edges(&previous_slice.entries);
        }

        self.insert_edges(&entries);
        self.entries_by_module
            .insert(module_id, CallIndexModuleSlice { entries });
    }

    /// Remove one module slice from the call index.
    pub fn remove_module(&mut self, module_id: ModuleId) {
        let Some(slice) = self.entries_by_module.remove(&module_id) else {
            return;
        };

        self.remove_edges(&slice.entries);
    }

    /// Collect call entries for one callee symbol.
    pub fn entries_for_callee(&self, callee_symbol: GlobalSymbolId) -> Vec<CallIndexEntry> {
        self.entries_by_callee
            .get(&callee_symbol)
            .cloned()
            .unwrap_or_default()
    }

    /// Collect call entries for one caller symbol.
    pub fn entries_for_caller(&self, caller_symbol: GlobalSymbolId) -> Vec<CallIndexEntry> {
        self.entries_by_caller
            .get(&caller_symbol)
            .cloned()
            .unwrap_or_default()
    }

    /// Add one batch of call edges to the reverse lookup maps.
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

    /// Remove one batch of call edges from the reverse lookup maps.
    fn remove_edges(&mut self, entries: &[CallIndexEntry]) {
        for entry in entries {
            if let Some(callee_entries) = self.entries_by_callee.get_mut(&entry.callee_symbol) {
                callee_entries.retain(|candidate| candidate != entry);
                if callee_entries.is_empty() {
                    self.entries_by_callee.remove(&entry.callee_symbol);
                }
            }

            let Some(caller_symbol) = entry.caller_symbol else {
                continue;
            };

            if let Some(caller_entries) = self.entries_by_caller.get_mut(&caller_symbol) {
                caller_entries.retain(|candidate| candidate != entry);
                if caller_entries.is_empty() {
                    self.entries_by_caller.remove(&caller_symbol);
                }
            }
        }
    }
}

/// One indexed module specifier entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecifierIndexEntry {
    /// The module that contains the specifier.
    pub module_id: ModuleId,
    /// The file that contains the specifier.
    pub file_id: FileId,
    /// The AST expression node id for the specifier expression.
    pub ast_node_id: u32,
    /// The raw specifier text.
    pub specifier: String,
    /// The resolved target module when available.
    pub target_module_id: Option<ModuleId>,
    /// The resolved target path when available.
    pub target_path: Option<PathBuf>,
}

/// One stored specifier index entry.
#[derive(Debug, Clone)]
struct StoredSpecifierIndexEntry {
    /// The stored public entry.
    entry: SpecifierIndexEntry,
}

/// One module slice inside the specifier index.
#[derive(Debug, Clone)]
struct SpecifierIndexModuleSlice {
    /// The stored entry ids for this module.
    entry_ids: Vec<usize>,
}

/// The workspace index for module specifier queries.
#[derive(Debug, Default)]
pub struct SpecifierIndex {
    /// The next stored entry id.
    next_entry_id: usize,
    /// The entries grouped by declaring module.
    entries_by_module: FxHashMap<ModuleId, SpecifierIndexModuleSlice>,
    /// The stored entries by id.
    entries_by_id: FxHashMap<usize, StoredSpecifierIndexEntry>,
    /// The entry ids grouped by resolved target path prefixes.
    entry_ids_by_target_path_prefix: FxHashMap<PathBuf, Vec<usize>>,
    /// The unresolved entry ids.
    unresolved_entry_ids: Vec<usize>,
}

impl SpecifierIndex {
    /// Return true when one module slice exists.
    pub fn has_module(&self, module_id: ModuleId) -> bool {
        self.entries_by_module.contains_key(&module_id)
    }

    /// Return the number of indexed module slices.
    pub fn module_count(&self) -> usize {
        self.entries_by_module.len()
    }

    /// Replace one module slice in the specifier index.
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

    /// Remove one module slice from the specifier index.
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

    /// Collect specifier entries for one set of renamed target paths plus unresolved fallbacks.
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

/// The workspace query indexes owned by one session.
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
