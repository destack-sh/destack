use destack_dir as dir;

use destack_source::{ModuleId, PathExt};
use destack_workspace::{Repository, Revision};

use super::module_specifier_in_expression;
use crate::core::{
    DirQueryContext, ImportEntry, ModuleQueryContext, SpecifierEntry, WorkspaceQueryContext,
};

/// Information about an exported symbol from a module.
#[derive(Debug, Clone)]
pub(crate) struct ExportedSymbol {
    /// The name of the exported symbol.
    pub name: String,
    /// The kind of symbol.
    pub kind: dir::SymbolKind,
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

impl ModuleQueryContext<'_> {
    /// Build import index entries for this module.
    pub(crate) fn build_import_candidates(&self) -> Vec<ImportEntry> {
        let Some(exports) = self.module_exports() else {
            return Vec::new();
        };

        exports
            .into_iter()
            .map(|export| ImportEntry {
                name: export.name,
                kind: export.kind,
                space: export.space,
                module_id: export.module_id,
                local_id: export.local_id,
                module_path: export.module_path,
            })
            .collect()
    }

    /// Get all exported symbols from this module.
    pub(crate) fn module_exports(&self) -> Option<Vec<ExportedSymbol>> {
        let repository = self.repository();
        let revision = self.revision();
        let module_id = self.module_id();
        let module = repository.module(revision, module_id).ok().flatten()?;
        let module_path = module_path_for_import(module.as_ref());

        let mut exports = Vec::new();

        // collect named local exports
        for (key, export) in self.dir().exports().exports() {
            let dir::ExportKey::Named(dir::StaticKey::Name(string_id)) = *key else {
                continue;
            };
            let dir::ExportEntry::Local(export) = export else {
                continue;
            };

            let target_symbol = export.source.into_global(module_id);
            let Some(kind) = self.export_symbol_kind(target_symbol) else {
                continue;
            };

            let name = self.dir().strings().get(string_id).to_string();
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

    /// Return the symbol kind for an export entry.
    fn export_symbol_kind(&self, symbol_id: dir::GlobalSymbolId) -> Option<dir::SymbolKind> {
        if symbol_id.module_id != self.module_id() {
            return None;
        }

        let symbols = self.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);

        Some(symbol.kind)
    }

    /// Build module specifier index entries for this module.
    pub(crate) fn build_specifier_candidates(&self) -> Vec<SpecifierEntry> {
        let dir = self.dir();
        let repository = self.repository();
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

        dir.build_specifier_candidates(repository, revision, module_id, dir_targets)
    }
}

impl WorkspaceQueryContext<'_> {
    /// Search for importable symbols across indexed modules.
    pub(crate) fn search_importable_symbols(
        &self,
        query: &str,
        exclude_module: Option<ModuleId>,
    ) -> Vec<ExportedSymbol> {
        let mut exports = Vec::new();

        // search the explicit profile selected for this import request
        let entries = self.search_import_candidates(query, exclude_module);
        exports.extend(entries.into_iter().map(|entry| ExportedSymbol {
            name: entry.name,
            kind: entry.kind,
            space: entry.space,
            module_id: entry.module_id,
            local_id: entry.local_id,
            module_path: entry.module_path,
        }));

        exports
    }
}

impl DirQueryContext<'_> {
    /// Build module specifier index entries.
    fn build_specifier_candidates(
        self,
        repository: &Repository,
        revision: Revision,
        module_id: ModuleId,
        dir_targets: std::collections::HashMap<u32, ModuleId>,
    ) -> Vec<SpecifierEntry> {
        let dir = self;
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
}
