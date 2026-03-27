use destack_dir as dir;
use std::sync::Arc;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace, SymbolType};
use destack_source::{FileId, ModuleId, PathExt};
use destack_workspace::{ImportIndexEntry, Program, Session, SpecifierIndexEntry};

use super::module_specifier_in_expression;
use crate::core::{
    SessionQueryIndexExt, query_context, with_ast_query_for_module, with_query_context_for_module,
};

/// Information about an exported symbol from a module.
#[derive(Debug, Clone)]
pub(crate) struct ExportedSymbol {
    /// The name of the exported symbol.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolType,
    /// The symbol space.
    pub space: dir::SymbolSpace,
    /// The module that exports this symbol.
    pub module_id: ModuleId,
    /// The local symbol id within the module.
    pub local_id: dir::LocalSymbolId,
    /// The module path (for import statement generation).
    pub module_path: Option<String>,
}

/// Resolve the program that owns a file.
pub(crate) fn program_for_file(session: &Session, file_id: FileId) -> Arc<Program> {
    // resolve the file path for program lookup
    let source_file = session.files.get(file_id);
    if let Some(path) = source_file.path.as_ref() {
        return session.find_program_for_path(path);
    }

    // fall back to the cwd program when the file has no path
    session.get_or_create_program(session.cwd.clone())
}

/// Resolve the program that owns a module.
pub(crate) fn program_for_module(
    session: &Session,
    module: &destack_workspace::Module,
) -> Arc<Program> {
    program_for_file(session, module.file_id)
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

/// Resolve the symbol facts for an export entry.
fn resolve_export_symbol_info(
    session: &Session,
    symbol_id: GlobalSymbolId,
) -> Option<(SymbolType, SymbolSpace)> {
    // resolve the module query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    with_query_context_for_module(session, module, |ctx| {
        let symbols = ctx.dir().resolved_symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        (symbol.ty, symbol.space)
    })
}

/// Get all exported symbols from a module when DIR is available.
pub(crate) fn get_module_exports_maybe(
    session: &Session,
    module_id: ModuleId,
) -> Option<Vec<ExportedSymbol>> {
    // get module AST and resolved DIR
    let module = session.modules.get(module_id);
    let module = module.as_ref();
    let ctx = query_context(session, module)?;
    let module_path = module_path_for_import(module);

    // initialize export collection
    let mut exports = Vec::new();

    // collect exported symbols from the resolved export table
    let exported_symbols = &ctx.dir().resolved().exported_symbols;
    for ((_, key), export) in exported_symbols.iter() {
        let StaticKey::Name(string_id) = *key else {
            continue;
        };

        let Some(target_symbol) = export.target.resolved() else {
            continue;
        };

        let Some((kind, space)) = resolve_export_symbol_info(session, target_symbol) else {
            continue;
        };

        let name = session.strings.get(string_id).to_string();
        exports.push(ExportedSymbol {
            name,
            kind,
            space,
            module_id,
            local_id: target_symbol.local_id,
            module_path: module_path.clone(),
        });
    }

    // return the collected exports
    Some(exports)
}

/// Search for importable symbols within a program.
pub(crate) fn search_importable_symbols_for_program(
    session: &Session,
    program: &Program,
    query: &str,
    exclude_module: Option<ModuleId>,
) -> Vec<ExportedSymbol> {
    // search cached entries owned by the program
    let entries = session.search_import_entries_for_program(program, query, exclude_module);

    entries
        .into_iter()
        .map(|entry| ExportedSymbol {
            name: entry.name,
            kind: entry.kind,
            space: entry.space,
            module_id: entry.module_id,
            local_id: entry.local_id,
            module_path: entry.module_path,
        })
        .collect()
}

/// Build import index entries for one program.
pub(crate) fn build_import_index_entries_for_program(
    session: &Session,
    program: &Program,
) -> Vec<ImportIndexEntry> {
    let mut entries = Vec::new();

    // collect every export visible from this program
    for module in program.modules.iter() {
        entries.extend(build_import_index_entries_for_module(session, module.id));
    }

    entries
}

/// Build import index entries for one module.
pub(crate) fn build_import_index_entries_for_module(
    session: &Session,
    module_id: ModuleId,
) -> Vec<ImportIndexEntry> {
    let Some(exports) = get_module_exports_maybe(session, module_id) else {
        return Vec::new();
    };

    exports
        .into_iter()
        .map(|export| ImportIndexEntry {
            name: export.name,
            kind: export.kind,
            space: export.space,
            module_id: export.module_id,
            local_id: export.local_id,
            module_path: export.module_path,
        })
        .collect()
}

/// Build module specifier index entries for one module.
pub(crate) fn build_specifier_index_entries_for_module(
    session: &Session,
    module: &destack_workspace::Module,
) -> Vec<SpecifierIndexEntry> {
    let query_context = query_context(session, module);

    let Some(entries) = with_ast_query_for_module(session, module, |ast| {
        let mut dir_targets = std::collections::HashMap::new();
        if let Some(ctx) = query_context.as_ref() {
            let dir_tree = ctx.dir().tree();
            for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
                let target_module = match expression {
                    dir::Expression::Import { target_module, .. }
                    | dir::Expression::ReExport { target_module, .. } => Some(*target_module),
                    _ => None,
                };
                let Some(target_module) = target_module else {
                    continue;
                };

                let source_id = dir_tree.get_source(expression_id.id);
                dir_targets.insert(source_id, target_module.module_id());
            }
        }

        let mut entries = Vec::new();
        for expression_id in ast.tree().iter_nodes::<destack_ast::Expression>() {
            let expression = ast.tree().get(expression_id);
            let Some((target, _kind)) = module_specifier_in_expression(ast.tree(), expression)
            else {
                continue;
            };

            let specifier = ast.strings().get(target).to_string();
            let target_module_id = dir_targets.get(&expression_id.id).copied().flatten();
            let target_path = target_module_id.and_then(|target_module_id| {
                let target_module = session.modules.get(target_module_id);
                let target_module = target_module.as_ref();
                target_module.path.as_ref().map(|path| path.normalize())
            });

            entries.push(SpecifierIndexEntry {
                module_id: module.id,
                file_id: module.file_id,
                ast_node_id: expression_id.id,
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
