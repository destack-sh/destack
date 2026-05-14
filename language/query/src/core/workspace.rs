use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{ArtifactKey, ModuleQueryIndex, WorkspaceQueryIndex};
use destack_dir::GlobalSymbolId;
use destack_qir::{
    CallEntry, ExtensionEntry, ImportEntry, NominalEntry, SpecifierEntry, SymbolEntry,
};
use destack_source::{ModuleId, ProfileId};
use destack_workspace::{Repository, Revision};

/// Search import candidates for the current workspace root.
pub(crate) fn search_import_candidates(
    repository: &Repository,
    revision: Revision,
    query: &str,
    exclude_module: Option<ModuleId>,
) -> Vec<ImportEntry> {
    let mut entries = Vec::new();

    let indexes = if let Some(index) =
        workspace_query_index_for_module(repository, revision, exclude_module)
    {
        module_query_indexes(repository, &index)
    } else {
        all_module_query_indexes(repository, revision)
    };

    for index in indexes {
        extend_unique(
            &mut entries,
            index.index.imports.search(query, exclude_module),
        );
    }
    sort_import_entries(&mut entries);

    entries
}

/// Search workspace symbol candidates across user modules.
pub(crate) fn search_workspace_symbol_candidates(
    repository: &Repository,
    revision: Revision,
    query: &str,
) -> Vec<SymbolEntry> {
    let mut entries = Vec::new();

    for index in all_module_query_indexes(repository, revision) {
        entries.extend(index.index.symbols.search(query));
    }

    entries.sort_by(|left, right| symbol_entry_key(left).cmp(&symbol_entry_key(right)));
    entries.dedup();

    entries
}

/// Collect nominal relation candidates for one target symbol.
pub(crate) fn nominal_relations_for_target(
    repository: &Repository,
    revision: Revision,
    target_symbol: GlobalSymbolId,
) -> Vec<NominalEntry> {
    let mut entries = Vec::new();

    for index in all_module_query_indexes(repository, revision) {
        extend_unique(&mut entries, index.index.nominal.to(target_symbol));
    }
    entries.sort();
    entries.dedup();

    entries
}

/// Collect extension candidates for one target symbol.
pub(crate) fn extension_candidates_for_target(
    repository: &Repository,
    revision: Revision,
    target_symbol: GlobalSymbolId,
) -> Vec<ExtensionEntry> {
    let mut entries = Vec::new();

    for index in all_module_query_indexes(repository, revision) {
        extend_unique(&mut entries, index.index.extensions.to(target_symbol));
    }
    entries.sort();
    entries.dedup();

    entries
}

/// Collect modules that may reference one target symbol.
pub(crate) fn modules_referencing_symbol(
    repository: &Repository,
    revision: Revision,
    target_symbol: GlobalSymbolId,
) -> Vec<ModuleId> {
    let mut module_ids = Vec::new();

    for index in all_module_query_indexes(repository, revision) {
        module_ids.extend(index.index.references.modules(target_symbol));
    }

    module_ids.sort();
    module_ids.dedup();

    module_ids
}

/// Collect call candidates for one callee symbol.
pub(crate) fn call_candidates_for_callee(
    repository: &Repository,
    revision: Revision,
    callee_symbol: GlobalSymbolId,
) -> Vec<CallEntry> {
    let mut entries = Vec::new();

    for index in all_module_query_indexes(repository, revision) {
        extend_unique(&mut entries, index.index.calls.to(callee_symbol));
    }
    sort_call_entries(&mut entries);

    entries
}

/// Collect call candidates for one caller symbol.
pub(crate) fn call_candidates_for_caller(
    repository: &Repository,
    revision: Revision,
    caller_symbol: GlobalSymbolId,
) -> Vec<CallEntry> {
    let mut entries = Vec::new();

    for index in all_module_query_indexes(repository, revision) {
        extend_unique(&mut entries, index.index.calls.from(caller_symbol));
    }
    sort_call_entries(&mut entries);

    entries
}

/// Collect specifier candidates relevant to one set of renamed paths.
pub(crate) fn specifier_candidates_for_rename_paths<I>(
    repository: &Repository,
    revision: Revision,
    old_paths: I,
) -> Vec<SpecifierEntry>
where
    I: IntoIterator<Item = PathBuf>,
{
    let old_paths = old_paths.into_iter().collect::<HashSet<_>>();
    let mut entries = Vec::new();

    for index in all_module_query_indexes(repository, revision) {
        extend_unique(
            &mut entries,
            index.index.specifiers.renaming(&old_paths).cloned(),
        );
    }
    sort_specifier_entries(&mut entries);

    entries
}

/// Return the workspace query index for one module's profile.
fn workspace_query_index_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: Option<ModuleId>,
) -> Option<Arc<WorkspaceQueryIndex>> {
    let module_id = module_id?;
    let profile_id = repository.module_profile(revision, module_id).ok()?.id();

    workspace_query_index(repository, revision, profile_id)
}

/// Return one ready workspace query index artifact.
fn workspace_query_index(
    repository: &Repository,
    revision: Revision,
    profile_id: ProfileId,
) -> Option<Arc<WorkspaceQueryIndex>> {
    let key = ArtifactKey::workspace_query_index(profile_id);
    let version = repository.artifact_version(revision, &key).ok().flatten()?;

    repository.artifact_store().workspace_query_index(&version)
}

/// Return all module query indexes referenced by all ready workspace indexes.
fn all_module_query_indexes(
    repository: &Repository,
    revision: Revision,
) -> Vec<Arc<ModuleQueryIndex>> {
    let mut indexes = Vec::new();

    for workspace in workspace_query_indexes(repository, revision) {
        indexes.extend(module_query_indexes(repository, &workspace));
    }

    indexes
}

/// Return all module query indexes referenced by one workspace index.
fn module_query_indexes(
    repository: &Repository,
    workspace: &WorkspaceQueryIndex,
) -> Vec<Arc<ModuleQueryIndex>> {
    let mut indexes = Vec::with_capacity(workspace.modules.len());

    for version in &workspace.modules {
        let Some(index) = repository.artifact_store().module_query_index(version) else {
            continue;
        };

        indexes.push(index);
    }

    indexes
}

/// Return all ready workspace query index artifacts.
fn workspace_query_indexes(
    repository: &Repository,
    revision: Revision,
) -> Vec<Arc<WorkspaceQueryIndex>> {
    let profile_ids = repository.profile_ids(revision).unwrap_or_default();
    let mut indexes = Vec::new();

    for profile_id in profile_ids {
        let Some(index) = workspace_query_index(repository, revision, profile_id) else {
            continue;
        };

        indexes.push(index);
    }

    indexes
}

/// Extend one vector without adding duplicate entries.
fn extend_unique<T>(entries: &mut Vec<T>, new_entries: impl IntoIterator<Item = T>)
where
    T: PartialEq,
{
    for entry in new_entries {
        if entries.contains(&entry) {
            continue;
        }

        entries.push(entry);
    }
}

/// Return the stable ordering key for one symbol entry.
fn symbol_entry_key(entry: &SymbolEntry) -> (&str, ModuleId, u128, u32, u32) {
    (
        entry.name.as_str(),
        entry.module_id,
        entry.file_id.0,
        entry.range.start,
        entry.range.end,
    )
}

/// Sort and deduplicate import entries.
fn sort_import_entries(entries: &mut Vec<ImportEntry>) {
    entries.sort_by(|left, right| import_entry_key(left).cmp(&import_entry_key(right)));
    entries.dedup();
}

/// Return the stable ordering key for one import entry.
fn import_entry_key(entry: &ImportEntry) -> (&str, ModuleId, u32) {
    (entry.name.as_str(), entry.module_id, entry.local_id.id)
}

/// Sort and deduplicate call entries.
fn sort_call_entries(entries: &mut Vec<CallEntry>) {
    entries.sort_by_key(|entry| {
        (
            entry.module_id,
            entry.caller_symbol,
            entry.callee_symbol,
            entry.span.file,
            entry.span.start,
            entry.span.end,
        )
    });
    entries.dedup();
}

/// Sort and deduplicate specifier entries.
fn sort_specifier_entries(entries: &mut Vec<SpecifierEntry>) {
    entries.sort_by(|left, right| specifier_entry_key(left).cmp(&specifier_entry_key(right)));
    entries.dedup();
}

/// Return the stable ordering key for one specifier entry.
fn specifier_entry_key(entry: &SpecifierEntry) -> (ModuleId, u128, u32, &str) {
    (
        entry.module_id,
        entry.file_id.0,
        entry.source_node_id,
        entry.specifier.as_str(),
    )
}
