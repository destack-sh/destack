use destack_dir as dir;
use std::sync::Arc;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolType};
use destack_source::{FileId, ModuleId};
use destack_workspace::{Program, Session};

use crate::core::{query_context, with_query_context_for_module};

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

/// Resolve a symbol type for an export entry.
fn resolve_export_symbol_type(session: &Session, symbol_id: GlobalSymbolId) -> Option<SymbolType> {
    // resolve the module query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    with_query_context_for_module(session, module, |ctx| {
        let symbols = ctx.dir().resolved_symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.ty
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
    for ((space, key), export) in exported_symbols.iter() {
        let StaticKey::Name(string_id) = *key else {
            continue;
        };

        let Some(target_symbol) = export.target.resolved() else {
            continue;
        };

        let Some(kind) = resolve_export_symbol_type(session, target_symbol) else {
            continue;
        };

        let name = session.strings.get(string_id).to_string();
        exports.push(ExportedSymbol {
            name,
            kind,
            space: *space,
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
    // search only modules owned by the program
    search_importable_symbols_for_modules(program.modules.iter(), session, query, exclude_module)
}

/// Search for importable symbols within a set of modules.
fn search_importable_symbols_for_modules<I>(
    modules: I,
    session: &Session,
    query: &str,
    exclude_module: Option<ModuleId>,
) -> Vec<ExportedSymbol>
where
    I: IntoIterator<Item = Arc<destack_workspace::Module>>,
{
    // prepare the result buffer and normalized query
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    // scan modules for exported symbols
    for module in modules {
        let module = module.as_ref();
        let module_id = module.id;

        // skip excluded modules
        if Some(module_id) == exclude_module {
            continue;
        }

        // resolve the module path for imports
        let module_path = module_path_for_import(module);

        // resolve exported symbols for the module
        let Some(exports) = get_module_exports_maybe(session, module_id) else {
            continue;
        };

        // collect exports that match the query
        for export in exports {
            let name = export.name;

            // skip symbols that do not match the query prefix
            if !query.is_empty() && !name.to_lowercase().starts_with(&query_lower) {
                continue;
            }

            results.push(ExportedSymbol {
                name,
                kind: export.kind,
                space: export.space,
                module_id,
                local_id: export.local_id,
                module_path: export.module_path.or_else(|| module_path.clone()),
            });
        }
    }

    // return results sorted by name
    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}
