use destack_dir as dir;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolForm};
use destack_source::{ModuleId, PathExt};

use super::module_specifier_in_expression;
use crate::core::{
    ImportEntry, ModuleQueryContext, SpecifierEntry, WorkspaceQueryContext,
    search_import_candidates,
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

    // use the module uri
    let uri = module.uri.as_ref();
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    if path.is_empty() {
        return None;
    }

    // return the resolved module path
    Some(path.to_string())
}

/// Return the symbol form for an export entry.
fn export_symbol_form(
    ctx: &ModuleQueryContext<'_>,
    symbol_id: GlobalSymbolId,
) -> Option<SymbolForm> {
    if symbol_id.module_id != ctx.module_id() {
        return None;
    }

    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    Some(symbol.form)
}

/// Search for importable symbols across indexed modules.
pub(crate) fn search_importable_symbols(
    workspace: &WorkspaceQueryContext<'_>,
    query: &str,
    exclude_module: Option<ModuleId>,
) -> Vec<ExportedSymbol> {
    let mut exports = Vec::new();

    // search the explicit profile selected for this import request
    let entries = search_import_candidates(workspace, query, exclude_module);
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
pub(crate) fn build_import_candidates_for_module(ctx: &ModuleQueryContext<'_>) -> Vec<ImportEntry> {
    let Some(exports) = module_exports(ctx) else {
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
fn module_exports(ctx: &ModuleQueryContext<'_>) -> Option<Vec<ExportedSymbol>> {
    let repository = ctx.repository();
    let revision = ctx.revision();
    let module_id = ctx.module_id();
    let module = repository.module(revision, module_id).ok().flatten()?;
    let module_path = module_path_for_import(module.as_ref());

    let mut exports = Vec::new();

    for (key, export) in ctx.dir().exports().exports() {
        let dir::ExportKey::Named(StaticKey::Name(string_id)) = *key else {
            continue;
        };
        let dir::ExportEntry::Local(export) = export else {
            continue;
        };

        let target_symbol = export.source.into_global(module_id);

        let Some(kind) = export_symbol_form(ctx, target_symbol) else {
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
    ctx: &ModuleQueryContext<'_>,
) -> Vec<SpecifierEntry> {
    let dir = ctx.dir();
    let repository = ctx.repository();
    let revision = dir.revision();
    let module_id = dir.module_id();
    let mut dir_targets = std::collections::HashMap::new();
    let dir_tree = dir.view();

    // collect semantic targets for resolved module specifiers
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        let target_module = match expression {
            dir::Expression::Import { .. } | dir::Expression::Export { .. } => {
                let node_id = expression_id.into_global_any(module_id);
                let relation = match expression {
                    dir::Expression::Import { .. } => dir::ModuleRelation::Import,
                    dir::Expression::Export { .. } => dir::ModuleRelation::ReExport,
                    _ => unreachable!(),
                };

                dir.modules().target_for_source(node_id, relation)
            }
            _ => None,
        };
        let Some(target_module) = target_module else {
            continue;
        };

        let source_id = dir_tree.get_source(expression_id);
        dir_targets.insert(source_id, target_module);
    }

    let mut entries = Vec::new();

    // index each syntactic module specifier with its semantic target when known
    for expression_id in dir.tree().iter_nodes::<dir::Expression>() {
        let expression = dir.tree().get(expression_id);
        let Some((target, _kind)) = module_specifier_in_expression(expression) else {
            continue;
        };

        let specifier = dir.strings().get(target).to_string();
        let target_module_id = dir_targets.get(&expression_id.id).copied();
        let target_path = target_module_id.and_then(|target_module_id| {
            let target_module = repository
                .module(revision, target_module_id)
                .ok()
                .flatten()?;
            target_module.path.as_ref().map(|path| path.normalize())
        });

        entries.push(SpecifierEntry {
            module_id,
            file_id: dir.file_id(),
            source_node_id: expression_id.id,
            specifier,
            target_module_id,
            target_path,
        });
    }

    entries
}
