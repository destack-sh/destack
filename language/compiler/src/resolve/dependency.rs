use dyst_dir::{DependencyItem, LocalNodeId, Module, NodeTree, SymbolKey, SymbolTable};

use crate::{Compiler, ImportTask, ResolveError, ResolveResult};

impl<'a> Compiler<'a> {
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
                mode,
                kind,
                alias,
                target,
                module: remote_module_id,
                symbol,
            } => {
                // target already resolved, use the resolved module
                if let Some(remote_module_id) = symbols.get_resolved_import(*target) {
                    todo!("resolve remote item from {remote_module_id:?}");
                }
                // target not ready yet, wait for import task
                else {
                    let import_task = ImportTask::ImportModuleFromSpecifier {
                        source: *source,
                        target: *target,
                        module: module.id,
                    };
                    return Err(ResolveError::Wait {
                        nodes: vec![item_id.into_global_any(module.id)],
                        tasks: vec![import_task.into()],
                    });
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
                    item_id.into_any(),
                    scope,
                    SymbolKey::Name(*name),
                    tree,
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
