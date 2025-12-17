use destack_dir::SymbolType;
use destack_source::ModuleId;

use crate::Session;

/// Information about an exported symbol from a module.
#[derive(Debug, Clone)]
pub struct ExportedSymbol {
    /// The name of the exported symbol.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolType,
    /// The module that exports this symbol.
    pub module_id: ModuleId,
    /// The local symbol id within the module.
    pub local_id: destack_dir::LocalSymbolId,
    /// The module path (for import statement generation).
    pub module_path: Option<String>,
}

/// Get all exported symbols from a module.
pub fn get_module_exports(session: &Session, module_id: ModuleId) -> Vec<ExportedSymbol> {
    let mut exports = Vec::new();

    let module = session.modules.get(module_id);
    let module_guard = module.read();
    let module_path = module_guard
        .path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    let symbols = module_guard.dir.symbols.read();

    for (idx, symbol) in symbols.symbols().enumerate() {
        if symbol.export.is_none() {
            continue;
        }

        let Some(string_id) = symbol.name() else {
            continue;
        };

        let name = module_guard.ast.strings.get(string_id).to_string();
        let local_id = destack_dir::LocalSymbolId::new_typed(idx as u32, symbol.ty);

        exports.push(ExportedSymbol {
            name,
            kind: symbol.ty,
            module_id,
            local_id,
            module_path: module_path.clone(),
        });
    }

    exports
}

/// Search for importable symbols (exported from other modules).
///
/// Useful for auto-import suggestions.
pub fn search_importable_symbols(
    session: &Session,
    query: &str,
    exclude_module: Option<ModuleId>,
) -> Vec<ExportedSymbol> {
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    for module in session.modules.iter() {
        let module_guard = module.read();
        let module_id = module_guard.id;

        if Some(module_id) == exclude_module {
            continue;
        }

        let module_path = module_guard
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string());

        let symbols = module_guard.dir.symbols.read();

        for (idx, symbol) in symbols.symbols().enumerate() {
            if symbol.export.is_none() {
                continue;
            }

            let Some(string_id) = symbol.name() else {
                continue;
            };

            let name = module_guard.ast.strings.get(string_id).to_string();

            if !query.is_empty() && !name.to_lowercase().starts_with(&query_lower) {
                continue;
            }

            let local_id = destack_dir::LocalSymbolId::new_typed(idx as u32, symbol.ty);

            results.push(ExportedSymbol {
                name,
                kind: symbol.ty,
                module_id,
                local_id,
                module_path: module_path.clone(),
            });
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}
