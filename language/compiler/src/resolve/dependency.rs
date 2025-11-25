use dyst_ast::StringId;
use dyst_dir::{
    DependencyItem, DependencyMode, DependencySource, GlobalNodeIdAny, GlobalScopeId, LocalNodeId,
    Module, NodeTree, SymbolKey, SymbolTable,
};

use crate::{CompileTaskWait, Compiler, ImportTask, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
    /// Error to wait for an import.
    pub(super) fn resolve_wait_for_import(
        &self,
        module: &Module,
        node: GlobalNodeIdAny,
        scope: GlobalScopeId,
        source: DependencySource,
        target: StringId,
    ) -> ResolveError {
        let import_task = ImportTask::ImportModuleFromSpecifier {
            source,
            target,
            module: module.id,
        };
        let error = ResolveError::UnresolvedModule {
            node,
            scope,
            target,
        };
        let wait = CompileTaskWait {
            nodes: vec![node],
            tasks: vec![import_task.into()],
            error: Some(Box::new(error.into())),
        };
        ResolveError::Wait { wait }
    }

    /// Resolve a DependencyItem.
    pub(super) fn resolve_dependency_item(
        &self,
        module: &Module,
        item_id: LocalNodeId<DependencyItem>,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
    ) -> ResolveResult<()> {
        let item = tree.get(item_id);
        let scope = symbols.get_scope(item_id, tree);
        let resolved_item: DependencyItem = match item {
            DependencyItem::UnresolvedRemote {
                source,
                name,
                mode,
                kind,
                alias,
                target,
                module: _,
                symbol,
            } => {
                // target already resolved, use the resolved module
                if let Some(remote_module_id) = symbols.get_resolved_import(*target) {
                    let target_symbol_id = match mode {
                        DependencyMode::Item => {
                            // resolve symbol in the remote module for item mode
                            let key = SymbolKey::Name(
                                name.unwrap_or_else(|| panic!("name is required for item mode")),
                            );
                            let remote_module = self.program.modules.get(remote_module_id);
                            let remote_module = remote_module.read();
                            let remote_symbols = remote_module.symbols.read();
                            let remote_scope = remote_symbols.get_scope(item_id, tree);

                            self.resolve_absolute_symbol(
                                module,
                                item_id.into_global_any(module.id),
                                remote_scope,
                                key,
                                symbols,
                            )?
                        }
                        DependencyMode::Default => module.default_symbol,
                        DependencyMode::Namespace => module.namespace_symbol,
                    };
                    DependencyItem::Remote {
                        mode: *mode,
                        kind: *kind,
                        name: *name,
                        alias: *alias,
                        target: *target,
                        module: remote_module_id,
                        symbol: *symbol,
                        target_symbol: target_symbol_id.into_global(remote_module_id),
                    }
                }
                // target not ready yet, wait for import task
                else {
                    return Err(self.resolve_wait_for_import(
                        module,
                        item_id.into_global_any(module.id),
                        scope.id.into_global(module.id),
                        *source,
                        *target,
                    ));
                }
            }
            DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
            } => {
                let target_symbol_id = self.resolve_absolute_symbol(
                    module,
                    item_id.into_global_any(module.id),
                    scope,
                    SymbolKey::Name(*name),
                    symbols,
                )?;
                DependencyItem::Local {
                    mode: *mode,
                    kind: *kind,
                    name: *name,
                    alias: *alias,
                    target_symbol: target_symbol_id,
                }
            }
            DependencyItem::Value { .. }
            | DependencyItem::Local { .. }
            | DependencyItem::Remote { .. } => {
                // nothing to do
                return Ok(());
            }
        };

        *tree.get_mut(item_id) = resolved_item;
        Ok(())
    }
}
