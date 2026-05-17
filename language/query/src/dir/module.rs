use destack_dir as dir;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolForm};
use destack_source::{ModuleId, PathExt};
use destack_workspace::{Repository, Revision};

use super::module_specifier_in_expression;
use crate::core::{ImportEntry, QueryContext, SpecifierEntry, search_import_candidates};

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
fn export_symbol_shape(ctx: &QueryContext<'_>, symbol_id: GlobalSymbolId) -> Option<SymbolForm> {
    if symbol_id.module_id != ctx.module_id() {
        return None;
    }

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
    let profile_ids = import_search_profile_ids(repository, revision, exclude_module);

    // search the explicit query profiles selected for this import request
    for profile_id in profile_ids {
        let entries =
            search_import_candidates(repository, revision, profile_id, query, exclude_module);
        exports.extend(entries.into_iter().map(|entry| ExportedSymbol {
            name: entry.name,
            kind: entry.form,
            space: entry.space,
            module_id: entry.module_id,
            local_id: entry.local_id,
            module_path: entry.module_path,
        }));
    }

    exports
}

/// Return profile ids searched by one import completion request.
fn import_search_profile_ids(
    repository: &Repository,
    revision: Revision,
    exclude_module: Option<ModuleId>,
) -> Vec<destack_source::ProfileId> {
    // file-backed auto imports use the file's active profile
    if let Some(module_id) = exclude_module
        && let Ok(profile) = repository.module_profile(revision, module_id)
    {
        return vec![profile.id()];
    }

    // workspace-level import searches use declared profiles
    repository.profile_ids(revision).unwrap_or_default()
}

/// Build import index entries for one module.
pub(crate) fn build_import_candidates_for_module(
    repository: &Repository,
    ctx: &QueryContext<'_>,
) -> Vec<ImportEntry> {
    let Some(exports) = module_exports(repository, ctx) else {
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

/// Get all exported symbols from one module context.
fn module_exports(repository: &Repository, ctx: &QueryContext<'_>) -> Option<Vec<ExportedSymbol>> {
    let revision = ctx.revision();
    let module_id = ctx.module_id();
    let module = repository.module(revision, module_id).ok().flatten()?;
    let module_path = module_path_for_import(module.as_ref());

    let mut exports = Vec::new();

    for (key, export) in ctx.dir().exported().exports.exports() {
        let dir::ExportKey::Named(StaticKey::Name(string_id)) = *key else {
            continue;
        };
        let dir::ExportEntry::Local(export) = export else {
            continue;
        };

        let target_symbol = export.source.into_global(module_id);

        let Some(kind) = export_symbol_shape(ctx, target_symbol) else {
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
    ctx: &QueryContext<'_>,
) -> Vec<SpecifierEntry> {
    let parsed = ctx.source();
    let revision = ctx.revision();
    let module_id = ctx.module_id();
    let mut dir_targets = std::collections::HashMap::new();
    let dir_tree = ctx.dir().view();

    // collect semantic targets for resolved module specifiers
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        let target_module = match expression {
            dir::Expression::Import { .. } | dir::Expression::Export { .. } => {
                let node_id = expression_id.into_global_any(module_id);
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

    let mut entries = Vec::new();

    // index each syntactic module specifier with its semantic target when known
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
}
