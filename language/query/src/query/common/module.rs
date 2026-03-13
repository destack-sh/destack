use destack_dir as dir;
use std::sync::Arc;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolType};
use destack_source::{FileId, ModuleId};
use parking_lot::RwLock;

use destack_workspace::{ExportEntry, Program, Session};

use super::query_context;

/// Information about an exported symbol from a module.
#[derive(Debug, Clone)]
pub struct ExportedSymbol {
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
pub fn program_for_file(session: &Session, file_id: FileId) -> Arc<Program> {
    // resolve the file path for program lookup
    let source_file = session.files.get(file_id);
    if let Some(path) = source_file.path.as_ref() {
        return session.find_program_for_path(path);
    }

    // fall back to the cwd program when the file has no path
    session.get_or_create_program(session.cwd.clone())
}

/// Resolve the program that owns a module.
pub fn program_for_module(session: &Session, module: &destack_workspace::Module) -> Arc<Program> {
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
    let module = module.read();
    let ctx = query_context(session, &module)?;
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    Some(symbol.ty)
}

/// Get all exported symbols from a module.
pub fn get_module_exports(session: &Session, module_id: ModuleId) -> Vec<ExportedSymbol> {
    get_module_exports_maybe(session, module_id).unwrap_or_default()
}

/// Get all exported symbols from a module when DIR is available.
pub fn get_module_exports_maybe(
    session: &Session,
    module_id: ModuleId,
) -> Option<Vec<ExportedSymbol>> {
    // get module AST and DIR
    let module = session.modules.get(module_id);
    let module = module.read();
    let ctx = query_context(session, &module)?;
    let module_path = module_path_for_import(&module);
    let symbols = ctx.symbols();

    // initialize export collection
    let mut exports = Vec::new();

    // read the exported symbols table
    let exported_symbols = &ctx.dir.exported_symbols;

    if !exported_symbols.is_empty() {
        // collect exported symbols from the export table
        for ((space, key), export) in exported_symbols.iter() {
            let StaticKey::Name(string_id) = *key else {
                continue;
            };

            let Some(target_symbol) = export.target.resolved() else {
                continue;
            };

            let name = session.strings.get(string_id).to_string();
            let kind =
                resolve_export_symbol_type(session, target_symbol).unwrap_or(SymbolType::Void);

            exports.push(ExportedSymbol {
                name,
                kind,
                space: *space,
                module_id,
                local_id: target_symbol.local_id,
                module_path: module_path.clone(),
            });
        }
    } else {
        // collect exported symbols from local symbol metadata
        for (idx, symbol) in symbols.symbols().enumerate() {
            // skip non exported symbols
            if symbol.export.is_none() {
                continue;
            }

            // resolve the symbol name
            let Some(string_id) = symbol.name() else {
                continue;
            };

            let name = session.strings.get(string_id).to_string();
            let local_id = dir::LocalSymbolId::new_typed(idx as u32, symbol.ty);

            // record the export entry
            exports.push(ExportedSymbol {
                name,
                kind: symbol.ty,
                space: symbol.space,
                module_id,
                local_id,
                module_path: module_path.clone(),
            });
        }
    }

    // return the collected exports
    Some(exports)
}

/// Ensure the program export index is populated for the current module versions.
pub fn ensure_program_export_index(session: &Session, program: &Program) {
    // snapshot module handles to avoid holding registry locks
    let modules: Vec<_> = program.modules.iter().collect();

    for module_ref in modules {
        // read module metadata needed for staleness checks
        let module = module_ref.read();
        let module_id = module.id;
        let module_version = module.version;
        drop(module);

        // skip modules with fresh export index entries
        if !program.index.exports.is_stale(module_id, module_version) {
            continue;
        }

        // skip modules without an available query context
        let Some(exports) = get_module_exports_maybe(session, module_id) else {
            continue;
        };

        // clear stale entries before inserting fresh exports
        program.index.exports.clear_module(module_id);

        // insert export entries for the module
        for export in exports {
            // create an export index entry
            let name_id = session.strings.intern(&export.name);
            let entry = ExportEntry {
                module_id: export.module_id,
                symbol_id: export.local_id,
                symbol_type: export.kind,
                space: export.space,
                name: export.name,
                module_path: export.module_path,
            };

            program.index.exports.add_export(name_id, entry);
        }

        // record the module version used for indexing
        program
            .index
            .exports
            .set_module_version(module_id, module_version);
    }
}

/// Search for importable symbols (exported from other modules).
///
/// Useful for auto import suggestions.
pub fn search_importable_symbols(
    session: &Session,
    query: &str,
    exclude_module: Option<ModuleId>,
) -> Vec<ExportedSymbol> {
    // fall back to searching across all modules
    search_importable_symbols_for_modules(session.modules.iter(), session, query, exclude_module)
}

/// Search for importable symbols within a program.
pub fn search_importable_symbols_for_program(
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
    I: IntoIterator<Item = Arc<RwLock<destack_workspace::Module>>>,
{
    // prepare the result buffer and normalized query
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    // scan modules for exported symbols
    for module in modules {
        let module = module.read();
        let module_id = module.id;

        // skip excluded modules
        if Some(module_id) == exclude_module {
            continue;
        }

        // resolve the module path for imports
        let module_path = module_path_for_import(&module);

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
