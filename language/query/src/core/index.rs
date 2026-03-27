use std::collections::HashSet;
use std::path::PathBuf;

use destack_dir::GlobalSymbolId;
use destack_source::ModuleId;
use destack_workspace::{
    CallIndexEntry, ExtensionIndexEntry, ImportIndexEntry, Module, NominalIndexEntry, Program,
    Session, SpecifierIndexEntry, SymbolIndexEntry,
};

use crate::dir::{
    build_call_index_entries_for_module, build_extension_index_entries_for_module,
    build_import_index_entries_for_program, build_nominal_index_entries_for_module,
    build_reference_index_entries_for_module, build_specifier_index_entries_for_module,
    build_symbol_index_entries_for_module,
};

/// The session verbs for derived workspace query indexes.
pub trait SessionQueryIndexExt {
    /// Index one set of modules into every workspace query index.
    fn index_query_modules<I>(&self, module_ids: I)
    where
        I: IntoIterator<Item = ModuleId>;

    /// Remove one set of modules from every workspace query index.
    fn remove_query_modules<I>(&self, module_ids: I)
    where
        I: IntoIterator<Item = ModuleId>;

    /// Index one program import slice into the workspace query index.
    fn index_query_imports_for_program(&self, program: &Program);

    /// Remove one program import slice from the workspace query index.
    fn remove_query_imports_for_program(&self, root: &std::path::Path);

    /// Search import entries for one program.
    fn search_import_entries_for_program(
        &self,
        program: &Program,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<ImportIndexEntry>;

    /// Search cached workspace symbol entries across user modules.
    fn search_workspace_symbol_entries(&self, query: &str) -> Vec<SymbolIndexEntry>;

    /// Collect nominal relation entries for one target symbol.
    fn nominal_index_entries_for_target(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Vec<NominalIndexEntry>;

    /// Collect extension entries for one target symbol.
    fn extension_index_entries_for_target(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Vec<ExtensionIndexEntry>;

    /// Collect candidate modules for one reference target symbol.
    fn reference_index_modules_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<ModuleId>;

    /// Collect call entries for one callee symbol.
    fn call_index_entries_for_callee(&self, callee_symbol: GlobalSymbolId) -> Vec<CallIndexEntry>;

    /// Collect call entries for one caller symbol.
    fn call_index_entries_for_caller(&self, caller_symbol: GlobalSymbolId) -> Vec<CallIndexEntry>;

    /// Collect module specifier entries relevant to one set of renamed paths.
    fn specifier_index_entries_for_rename_paths<I>(&self, old_paths: I) -> Vec<SpecifierIndexEntry>
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

impl SessionQueryIndexExt for Session {
    fn index_query_modules<I>(&self, module_ids: I)
    where
        I: IntoIterator<Item = ModuleId>,
    {
        // deduplicate module ids before rebuilding slices
        let module_ids: HashSet<ModuleId> = module_ids.into_iter().collect();
        let mut slices = Vec::new();

        // derive replacement slices before taking the write lock
        for module_id in module_ids {
            if !self.modules.contains(module_id) {
                continue;
            }

            let module = self.modules.get(module_id);
            let module = module.as_ref();
            let slice = build_module_query_index_slice(self, module_id, module);
            slices.push(slice);
        }

        // swap the prepared slices into the shared index
        self.with_query_index_mut(|query_index| {
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

    fn remove_query_modules<I>(&self, module_ids: I)
    where
        I: IntoIterator<Item = ModuleId>,
    {
        self.with_query_index_mut(|query_index| {
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

    fn index_query_imports_for_program(&self, program: &Program) {
        // derive the full program import slice before taking the write lock
        let entries = build_import_index_entries_for_program(self, program);
        let root = program.cwd.clone();

        // swap the prepared import slice into the shared index
        self.with_query_index_mut(|query_index| {
            query_index.import.replace_program(root, entries);
        });
    }

    fn remove_query_imports_for_program(&self, root: &std::path::Path) {
        self.with_query_index_mut(|query_index| {
            query_index.import.remove_program(root);
        });
    }

    fn search_import_entries_for_program(
        &self,
        program: &Program,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<ImportIndexEntry> {
        self.with_query_index(|query_index| {
            query_index
                .import
                .search_program(program.cwd.as_path(), query, exclude_module)
        })
    }

    fn search_workspace_symbol_entries(&self, query: &str) -> Vec<SymbolIndexEntry> {
        self.with_query_index(|query_index| query_index.symbol.search(query))
    }

    fn nominal_index_entries_for_target(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Vec<NominalIndexEntry> {
        self.with_query_index(|query_index| query_index.nominal.entries_for_target(target_symbol))
    }

    fn extension_index_entries_for_target(
        &self,
        target_symbol: GlobalSymbolId,
    ) -> Vec<ExtensionIndexEntry> {
        self.with_query_index(|query_index| query_index.extension.entries_for_target(target_symbol))
    }

    fn reference_index_modules_for_target(&self, target_symbol: GlobalSymbolId) -> Vec<ModuleId> {
        self.with_query_index(|query_index| query_index.reference.modules_for_target(target_symbol))
    }

    fn call_index_entries_for_callee(&self, callee_symbol: GlobalSymbolId) -> Vec<CallIndexEntry> {
        self.with_query_index(|query_index| query_index.call.entries_for_callee(callee_symbol))
    }

    fn call_index_entries_for_caller(&self, caller_symbol: GlobalSymbolId) -> Vec<CallIndexEntry> {
        self.with_query_index(|query_index| query_index.call.entries_for_caller(caller_symbol))
    }

    fn specifier_index_entries_for_rename_paths<I>(&self, old_paths: I) -> Vec<SpecifierIndexEntry>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        self.with_query_index(|query_index| {
            query_index
                .specifier
                .entries_for_paths_or_unresolved(old_paths)
        })
    }
}

/// Derive every query-index slice for one module.
fn build_module_query_index_slice(
    session: &Session,
    module_id: ModuleId,
    module: &Module,
) -> ModuleQueryIndexSlice {
    // workspace symbol search
    let symbol_entries = module
        .is_user()
        .then(|| build_symbol_index_entries_for_module(session, module));

    // hierarchy and extension lookup
    let nominal_entries = build_nominal_index_entries_for_module(session, module);
    let extension_entries = build_extension_index_entries_for_module(session, module);

    // reference and call fanout
    let reference_entries = build_reference_index_entries_for_module(session, module);
    let call_entries = build_call_index_entries_for_module(session, module);

    // specifier rewrites
    let specifier_entries = build_specifier_index_entries_for_module(session, module);

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
