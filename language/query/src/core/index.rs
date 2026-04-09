use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use destack_dir::GlobalSymbolId;
use destack_source::ModuleId;
use destack_workspace::{
    CallIndexEntry, ExtensionIndexEntry, ImportIndexEntry, Module, NominalIndexEntry, QueryIndex,
    Repository, Revision, SpecifierIndexEntry, SymbolIndexEntry,
};
use parking_lot::RwLock;

use crate::dir::{
    build_call_index_entries_for_module, build_extension_index_entries_for_module,
    build_import_index_entries, build_nominal_index_entries_for_module,
    build_reference_index_entries_for_module, build_specifier_index_entries_for_module,
    build_symbol_index_entries_for_module,
};

/// The repository verbs for derived workspace query indexes.
pub trait RepositoryQueryIndexExt {
    /// Index one set of modules into every workspace query index.
    fn index_query_modules<I>(&self, revision: Revision, module_ids: I)
    where
        I: IntoIterator<Item = ModuleId>;

    /// Remove one set of modules from every workspace query index.
    fn remove_query_modules<I>(&self, revision: Revision, module_ids: I)
    where
        I: IntoIterator<Item = ModuleId>;

    /// Index one import slice into the workspace query index.
    fn index_query_imports(&self, revision: Revision);

    /// Remove one import slice from the workspace query index.
    fn remove_query_imports(&self, revision: Revision);

    /// Search import entries for the current workspace root.
    fn search_import_entries(
        &self,
        revision: Revision,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<ImportIndexEntry>;

    /// Search cached workspace symbol entries across user modules.
    fn search_workspace_symbol_entries(
        &self,
        revision: Revision,
        query: &str,
    ) -> Vec<SymbolIndexEntry>;

    /// Collect nominal relation entries for one target symbol.
    fn nominal_index_entries_for_target(
        &self,
        revision: Revision,
        target_symbol: GlobalSymbolId,
    ) -> Vec<NominalIndexEntry>;

    /// Collect extension entries for one target symbol.
    fn extension_index_entries_for_target(
        &self,
        revision: Revision,
        target_symbol: GlobalSymbolId,
    ) -> Vec<ExtensionIndexEntry>;

    /// Collect candidate modules for one reference target symbol.
    fn reference_index_modules_for_target(
        &self,
        revision: Revision,
        target_symbol: GlobalSymbolId,
    ) -> Vec<ModuleId>;

    /// Collect call entries for one callee symbol.
    fn call_index_entries_for_callee(
        &self,
        revision: Revision,
        callee_symbol: GlobalSymbolId,
    ) -> Vec<CallIndexEntry>;

    /// Collect call entries for one caller symbol.
    fn call_index_entries_for_caller(
        &self,
        revision: Revision,
        caller_symbol: GlobalSymbolId,
    ) -> Vec<CallIndexEntry>;

    /// Collect module specifier entries relevant to one set of renamed paths.
    fn specifier_index_entries_for_rename_paths<I>(
        &self,
        revision: Revision,
        old_paths: I,
    ) -> Vec<SpecifierIndexEntry>
    where
        I: IntoIterator<Item = PathBuf>;
}

/// One fully-derived index slice replacement for one module.
struct ModuleQueryIndexSlice {
    /// The module to replace.
    module_id: ModuleId,
    /// The searchable workspace symbol entries for this module.
    symbol_entries: Option<Vec<SymbolIndexEntry>>,
    /// The nominal hierarchy edges for this module.
    nominal_entries: Vec<NominalIndexEntry>,
    /// The extension lookup entries for this module.
    extension_entries: Vec<ExtensionIndexEntry>,
    /// The reference target keys for this module.
    reference_entries: Vec<GlobalSymbolId>,
    /// The call-site entries for this module.
    call_entries: Vec<CallIndexEntry>,
    /// The specifier rewrite entries for this module.
    specifier_entries: Vec<SpecifierIndexEntry>,
}

impl RepositoryQueryIndexExt for Repository {
    fn index_query_modules<I>(&self, revision: Revision, module_ids: I)
    where
        I: IntoIterator<Item = ModuleId>,
    {
        // deduplicate module ids before rebuilding slices
        let module_ids: HashSet<ModuleId> = module_ids.into_iter().collect();
        let mut slices = Vec::new();

        // derive replacement slices before taking the write lock
        for module_id in module_ids {
            let Some(module) = self.module(revision, module_id).ok().flatten() else {
                continue;
            };
            let module = module.as_ref();
            let slice = build_module_query_index_slice(self, revision, module_id, module);
            slices.push(slice);
        }

        // swap the prepared slices into the shared index
        with_query_index_mut(self, revision, |query_index| {
            for slice in slices {
                if let Some(symbol_entries) = slice.symbol_entries {
                    query_index
                        .symbol
                        .replace_module(slice.module_id, symbol_entries);
                } else {
                    query_index.symbol.remove_module(slice.module_id);
                }

                query_index
                    .nominal
                    .replace_module(slice.module_id, slice.nominal_entries);
                query_index
                    .extension
                    .replace_module(slice.module_id, slice.extension_entries);
                query_index
                    .reference
                    .replace_module(slice.module_id, slice.reference_entries);
                query_index
                    .call
                    .replace_module(slice.module_id, slice.call_entries);
                query_index
                    .specifier
                    .replace_module(slice.module_id, slice.specifier_entries);
            }
        });
    }

    fn remove_query_modules<I>(&self, revision: Revision, module_ids: I)
    where
        I: IntoIterator<Item = ModuleId>,
    {
        with_query_index_mut(self, revision, |query_index| {
            for module_id in module_ids {
                query_index.symbol.remove_module(module_id);
                query_index.nominal.remove_module(module_id);
                query_index.extension.remove_module(module_id);
                query_index.reference.remove_module(module_id);
                query_index.call.remove_module(module_id);
                query_index.specifier.remove_module(module_id);
            }
        });
    }

    fn index_query_imports(&self, revision: Revision) {
        // derive the full import slice before taking the write lock
        let entries = build_import_index_entries(self, revision);
        let root = self.workspace_root().to_path_buf();

        // swap the prepared import slice into the shared index
        with_query_index_mut(self, revision, |query_index| {
            query_index.import.replace_root(root, entries);
        });
    }

    fn remove_query_imports(&self, revision: Revision) {
        with_query_index_mut(self, revision, |query_index| {
            query_index.import.remove_root(self.workspace_root());
        });
    }

    fn search_import_entries(
        &self,
        revision: Revision,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<ImportIndexEntry> {
        ensure_workspace_query_index(self, revision);

        with_query_index(self, revision, |query_index| {
            query_index
                .import
                .search_root(self.workspace_root(), query, exclude_module)
        })
    }

    fn search_workspace_symbol_entries(
        &self,
        revision: Revision,
        query: &str,
    ) -> Vec<SymbolIndexEntry> {
        ensure_workspace_query_index(self, revision);

        with_query_index(self, revision, |query_index| {
            query_index.symbol.search(query)
        })
    }

    fn nominal_index_entries_for_target(
        &self,
        revision: Revision,
        target_symbol: GlobalSymbolId,
    ) -> Vec<NominalIndexEntry> {
        ensure_workspace_query_index(self, revision);

        with_query_index(self, revision, |query_index| {
            query_index.nominal.entries_for_target(target_symbol)
        })
    }

    fn extension_index_entries_for_target(
        &self,
        revision: Revision,
        target_symbol: GlobalSymbolId,
    ) -> Vec<ExtensionIndexEntry> {
        ensure_workspace_query_index(self, revision);

        with_query_index(self, revision, |query_index| {
            query_index.extension.entries_for_target(target_symbol)
        })
    }

    fn reference_index_modules_for_target(
        &self,
        revision: Revision,
        target_symbol: GlobalSymbolId,
    ) -> Vec<ModuleId> {
        ensure_workspace_query_index(self, revision);

        with_query_index(self, revision, |query_index| {
            query_index.reference.modules_for_target(target_symbol)
        })
    }

    fn call_index_entries_for_callee(
        &self,
        revision: Revision,
        callee_symbol: GlobalSymbolId,
    ) -> Vec<CallIndexEntry> {
        ensure_workspace_query_index(self, revision);

        with_query_index(self, revision, |query_index| {
            query_index.call.entries_for_callee(callee_symbol)
        })
    }

    fn call_index_entries_for_caller(
        &self,
        revision: Revision,
        caller_symbol: GlobalSymbolId,
    ) -> Vec<CallIndexEntry> {
        ensure_workspace_query_index(self, revision);

        with_query_index(self, revision, |query_index| {
            query_index.call.entries_for_caller(caller_symbol)
        })
    }

    fn specifier_index_entries_for_rename_paths<I>(
        &self,
        revision: Revision,
        old_paths: I,
    ) -> Vec<SpecifierIndexEntry>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        ensure_workspace_query_index(self, revision);

        with_query_index(self, revision, |query_index| {
            query_index
                .specifier
                .entries_for_paths_or_unresolved(old_paths)
        })
    }
}

/// Ensure the workspace query cache is current for one revision.
fn ensure_workspace_query_index(repository: &Repository, revision: Revision) {
    if repository
        .revision(revision)
        .ok()
        .and_then(|revision| revision.query_index.get().cloned())
        .is_some()
    {
        return;
    }

    rebuild_workspace_query_index(repository, revision);
}

/// Rebuild the full workspace query cache for one revision.
fn rebuild_workspace_query_index(repository: &Repository, revision: Revision) {
    let Ok(workspace_module_ids) = repository.workspace_module_ids(revision) else {
        return;
    };

    let mut slices = Vec::new();
    for module_id in workspace_module_ids {
        let Some(module) = repository.module(revision, module_id).ok().flatten() else {
            continue;
        };

        let slice =
            build_module_query_index_slice(repository, revision, module_id, module.as_ref());
        slices.push(slice);
    }

    let import_entries = build_import_index_entries(repository, revision);
    let root = repository.workspace_root().to_path_buf();

    with_query_index_mut(repository, revision, |query_index| {
        *query_index = QueryIndex::default();

        for slice in slices {
            if let Some(symbol_entries) = slice.symbol_entries {
                query_index
                    .symbol
                    .replace_module(slice.module_id, symbol_entries);
            } else {
                query_index.symbol.remove_module(slice.module_id);
            }

            query_index
                .nominal
                .replace_module(slice.module_id, slice.nominal_entries);
            query_index
                .extension
                .replace_module(slice.module_id, slice.extension_entries);
            query_index
                .reference
                .replace_module(slice.module_id, slice.reference_entries);
            query_index
                .call
                .replace_module(slice.module_id, slice.call_entries);
            query_index
                .specifier
                .replace_module(slice.module_id, slice.specifier_entries);
        }

        query_index.import.replace_root(root, import_entries);
    });
}

/// Return the query index lock for one revision.
fn revision_query_index(repository: &Repository, revision: Revision) -> Arc<RwLock<QueryIndex>> {
    let revision = repository
        .revision(revision)
        .unwrap_or_else(|error| panic!("missing revision for query index: {error}"));

    revision
        .query_index
        .get_or_init(|| Arc::new(RwLock::new(QueryIndex::default())))
        .clone()
}

/// Read one revision-local query index.
fn with_query_index<R>(
    repository: &Repository,
    revision: Revision,
    read: impl FnOnce(&QueryIndex) -> R,
) -> R {
    let query_index = revision_query_index(repository, revision);
    let query_index = query_index.read();

    read(&query_index)
}

/// Mutate one revision-local query index.
fn with_query_index_mut<R>(
    repository: &Repository,
    revision: Revision,
    write: impl FnOnce(&mut QueryIndex) -> R,
) -> R {
    let query_index = revision_query_index(repository, revision);
    let mut query_index = query_index.write();

    write(&mut query_index)
}

/// Derive every query-index slice for one module.
fn build_module_query_index_slice(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    module: &Module,
) -> ModuleQueryIndexSlice {
    // workspace symbol search
    let symbol_entries = module
        .is_user()
        .then(|| build_symbol_index_entries_for_module(repository, revision, module_id));

    // hierarchy and extension lookup
    let nominal_entries = build_nominal_index_entries_for_module(repository, revision, module_id);
    let extension_entries =
        build_extension_index_entries_for_module(repository, revision, module_id);

    // reference and call fanout
    let reference_entries =
        build_reference_index_entries_for_module(repository, revision, module_id);
    let call_entries = build_call_index_entries_for_module(repository, revision, module_id);

    // specifier rewrites
    let specifier_entries =
        build_specifier_index_entries_for_module(repository, revision, module_id);

    ModuleQueryIndexSlice {
        module_id,
        symbol_entries,
        nominal_entries,
        extension_entries,
        reference_entries,
        call_entries,
        specifier_entries,
    }
}
