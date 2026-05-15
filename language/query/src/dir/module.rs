use destack_dir as dir;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolForm};
use destack_source::{ModuleId, PathExt, ProfileId};
use destack_workspace::{Repository, Revision};

use super::module_specifier_in_expression;
use crate::core::{
    ImportEntry, SpecifierEntry, query_context_for_profile, search_import_candidates,
    with_source_query_for_module,
};

/// Information about an exported symbol from a module.
#[derive(Debug, Clone)]
pub(crate) struct ExportedSymbol {
    /// The name of the exported symbol.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolForm,
    /// The export lookup space.
    pub space: dir::SymbolSpace,
    /// The module that exports this symbol.
    pub module_id: ModuleId,
    /// The local symbol id within the module.
    pub local_id: dir::LocalSymbolId,
    /// The module path (for import statement generation).
    pub module_path: Option<String>,
}

/// Resolve the importable module path for a module.
fn module_path_for_import(module: &destack_workspace::Module) -> Option<String> {
    // prefer a filesystem path when available
    if let Some(path) = module.path.as_ref() {
        return Some(path.to_string_lossy().to_string());
    }

    // fall back to the module uri
    let uri = module.uri.as_ref();
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    if path.is_empty() {
        return None;
    }

    // return the resolved module path
    Some(path.to_string())
}

/// Return the symbol shape for an export entry.
fn export_symbol_shape(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
    profile_id: ProfileId,
) -> Option<SymbolForm> {
    let ctx = query_context_for_profile(repository, revision, symbol_id.module_id, profile_id)?;
    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    Some(symbol.form)
}

/// Search for importable symbols across the current workspace root.
pub(crate) fn search_importable_symbols(
    repository: &Repository,
    revision: Revision,
    query: &str,
    exclude_module: Option<ModuleId>,
) -> Vec<ExportedSymbol> {
    let mut exports = Vec::new();

    let entries = search_import_candidates(repository, revision, query, exclude_module);
    exports.extend(entries.into_iter().map(|entry| ExportedSymbol {
        name: entry.name,
        kind: entry.form,
        space: entry.space,
        module_id: entry.module_id,
        local_id: entry.local_id,
        module_path: entry.module_path,
    }));

    exports
}

/// Build import index entries for one module.
pub(crate) fn build_import_candidates_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Vec<ImportEntry> {
    let Some(exports) = get_module_exports_maybe(repository, revision, module_id, profile_id)
    else {
        return Vec::new();
    };

    exports
        .into_iter()
        .map(|export| ImportEntry {
            name: export.name,
            form: export.kind,
            space: export.space,
            module_id: export.module_id,
            local_id: export.local_id,
            module_path: export.module_path,
        })
        .collect()
}

/// Get all exported symbols from a module when DIR is available.
pub(crate) fn get_module_exports_maybe(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Option<Vec<ExportedSymbol>> {
    let module = repository.module(revision, module_id).ok().flatten()?;
    let ctx = query_context_for_profile(repository, revision, module_id, profile_id)?;
    let module_path = module_path_for_import(module.as_ref());

    let mut exports = Vec::new();

    for (key, export) in ctx.dir().exported().exports.exports() {
        let dir::ExportName::Named(StaticKey::Name(string_id)) = *key else {
            continue;
        };
        let dir::ExportEntry::Local(export) = export else {
            continue;
        };

        let target_symbol = export.source.into_global(module_id);

        let Some(kind) = export_symbol_shape(repository, revision, target_symbol, profile_id)
        else {
            continue;
        };

        let name = ctx.dir().strings().get(string_id).to_string();
        exports.push(ExportedSymbol {
            name,
            kind,
            space: kind.symbol_space(),
            module_id,
            local_id: target_symbol.local_id,
            module_path: module_path.clone(),
        });
    }

    Some(exports)
}

/// Build module specifier index entries for one module.
pub(crate) fn build_specifier_candidates_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Vec<SpecifierEntry> {
    let query_context = query_context_for_profile(repository, revision, module_id, profile_id);

    let Some(entries) = with_source_query_for_module(repository, revision, module_id, |parsed| {
        let mut dir_targets = std::collections::HashMap::new();
        if let Some(ctx) = query_context.as_ref() {
            let dir_tree = ctx.dir().view();
            for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
                let target_module = match expression {
                    dir::Expression::Import { .. } | dir::Expression::Export { .. } => {
                        let node_id = expression_id.into_global_any(ctx.module_id());
                        ctx.dir()
                            .types()
                            .dependency_resolution(node_id)
                            .and_then(|resolution| match resolution {
                                dir::DependencyResolution::Module(target) => Some(*target),
                                dir::DependencyResolution::Symbol(_) => None,
                            })
                    }
                    _ => None,
                };
                let Some(target_module) = target_module else {
                    continue;
                };

                let source_id = dir_tree.get_source(expression_id);
                dir_targets.insert(source_id, target_module.module_id());
            }
        }

        let mut entries = Vec::new();
        for expression_id in parsed.tree().iter_nodes::<dir::Expression>() {
            let expression = parsed.tree().get(expression_id);
            let Some((target, _kind)) = module_specifier_in_expression(expression) else {
                continue;
            };

            let specifier = parsed.strings().get(target).to_string();
            let target_module_id = dir_targets.get(&expression_id.id).copied().flatten();
            let target_path = target_module_id.and_then(|target_module_id| {
                let target_module = repository
                    .module(revision, target_module_id)
                    .ok()
                    .flatten()?;
                target_module.path.as_ref().map(|path| path.normalize())
            });

            entries.push(SpecifierEntry {
                module_id,
                file_id: parsed.file_id(),
                source_node_id: expression_id.id,
                specifier,
                target_module_id,
                target_path,
            });
        }

        entries
    }) else {
        return Vec::new();
    };

    entries
}
