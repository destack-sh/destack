use std::collections::HashSet;
use std::path::PathBuf;

use destack_dir::GlobalSymbolId;
use destack_qir::{
    AnnotationEntry, CallEntry, ExtensionEntry, ImportEntry, NominalEntry, SpecifierEntry,
    SymbolEntry,
};
use destack_source::{ModuleId, ProfileId};

use super::WorkspaceQueryContext;

/// Search import candidates across indexed modules.
pub(crate) fn search_import_candidates(
    ctx: &WorkspaceQueryContext<'_>,
    query: &str,
    exclude_module: Option<ModuleId>,
) -> Vec<ImportEntry> {
    let mut entries = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            entries.extend(index.index.imports.search(query, exclude_module));
        }
    }
    sort_import_entries(&mut entries);

    entries
}

/// Search workspace symbol candidates across indexed modules.
pub(crate) fn search_workspace_symbol_candidates(
    ctx: &WorkspaceQueryContext<'_>,
    query: &str,
) -> Vec<(ProfileId, SymbolEntry)> {
    let mut entries = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            entries.extend(
                index
                    .index
                    .symbols
                    .search(query)
                    .into_iter()
                    .map(|entry| (profile.profile_id(), entry)),
            );
        }
    }

    entries.sort_by(|left, right| {
        (
            left.0,
            left.1.name.as_str(),
            left.1.module_id,
            left.1.file_id.0,
            left.1.range.start,
            left.1.range.end,
        )
            .cmp(&(
                right.0,
                right.1.name.as_str(),
                right.1.module_id,
                right.1.file_id.0,
                right.1.range.start,
                right.1.range.end,
            ))
    });
    entries.dedup();

    entries
}

/// Search annotation candidates across indexed modules.
pub(crate) fn search_annotation_candidates(
    ctx: &WorkspaceQueryContext<'_>,
    name: Option<&str>,
) -> Vec<(ProfileId, AnnotationEntry)> {
    let mut entries = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            entries.extend(
                index
                    .index
                    .annotations
                    .search(name)
                    .into_iter()
                    .map(|entry| (profile.profile_id(), entry)),
            );
        }
    }

    entries.sort_by_key(|(profile_id, entry)| {
        (
            *profile_id,
            entry.name.clone(),
            entry.module_id,
            entry.target_id.local_id.id,
            entry.decorator_id.local_id.id,
        )
    });
    entries.dedup();

    entries
}

/// Collect nominal relation candidates for one target symbol.
pub(crate) fn nominal_relations_for_target(
    ctx: &WorkspaceQueryContext<'_>,
    target_symbol: GlobalSymbolId,
) -> Vec<NominalEntry> {
    let mut entries = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            entries.extend(index.index.nominal.for_target(target_symbol));
        }
    }
    entries.sort();
    entries.dedup();

    entries
}

/// Collect extension candidates for one target symbol.
pub(crate) fn extension_candidates_for_target(
    ctx: &WorkspaceQueryContext<'_>,
    target_symbol: GlobalSymbolId,
) -> Vec<ExtensionEntry> {
    let mut entries = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            entries.extend(index.index.extensions.for_target(target_symbol));
        }
    }
    entries.sort();
    entries.dedup();

    entries
}

/// Collect modules that may reference one target symbol.
pub(crate) fn modules_referencing_symbol(
    ctx: &WorkspaceQueryContext<'_>,
    target_symbol: GlobalSymbolId,
) -> Vec<ModuleId> {
    let mut module_ids = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            module_ids.extend(index.index.references.modules(target_symbol));
        }
    }

    module_ids.sort();
    module_ids.dedup();

    module_ids
}

/// Collect call candidates for one callee symbol.
pub(crate) fn call_candidates_for_callee(
    ctx: &WorkspaceQueryContext<'_>,
    callee_symbol: GlobalSymbolId,
) -> Vec<CallEntry> {
    let mut entries = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            entries.extend(index.index.calls.for_callee(callee_symbol));
        }
    }
    sort_call_entries(&mut entries);

    entries
}

/// Collect call candidates for one caller symbol.
pub(crate) fn call_candidates_for_caller(
    ctx: &WorkspaceQueryContext<'_>,
    caller_symbol: GlobalSymbolId,
) -> Vec<CallEntry> {
    let mut entries = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            entries.extend(index.index.calls.for_caller(caller_symbol));
        }
    }
    sort_call_entries(&mut entries);

    entries
}

/// Collect specifier candidates relevant to one set of renamed paths.
pub(crate) fn specifier_candidates_for_rename_paths<I>(
    ctx: &WorkspaceQueryContext<'_>,
    old_paths: I,
) -> Vec<(ProfileId, SpecifierEntry)>
where
    I: IntoIterator<Item = PathBuf>,
{
    let old_paths = old_paths.into_iter().collect::<HashSet<_>>();
    let mut entries = Vec::new();

    for profile in ctx.indexes() {
        for index in profile.modules() {
            entries.extend(
                index
                    .index
                    .specifiers
                    .renaming(&old_paths)
                    .map(|entry| (profile.profile_id(), entry.clone())),
            );
        }
    }
    entries.sort_by(|left, right| {
        (
            left.0,
            left.1.module_id,
            left.1.file_id.0,
            left.1.source_node_id,
            left.1.specifier.as_str(),
        )
            .cmp(&(
                right.0,
                right.1.module_id,
                right.1.file_id.0,
                right.1.source_node_id,
                right.1.specifier.as_str(),
            ))
    });
    entries.dedup();

    entries
}

/// Sort and deduplicate import entries.
fn sort_import_entries(entries: &mut Vec<ImportEntry>) {
    entries.sort_by(|left, right| {
        (left.name.as_str(), left.module_id, left.local_id.id).cmp(&(
            right.name.as_str(),
            right.module_id,
            right.local_id.id,
        ))
    });
    entries.dedup();
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
